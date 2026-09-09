//! Inline lowering: mdast phrasing nodes to `Inline`, including the brace
//! groups (roles, attribute lists, moustaches), references, definitions and
//! the attention sugar. Spec §Inline text, §Roles, §Attributes, §Two sigils.

use tmark_ir::{
    registry::{self, ArgStyle},
    Aside, Attrs, Block, Code, CodeInline, Comment, CounterItem, Emph, Highlight, Image,
    IndexEntry, Inline, Keystroke, LineBreak, Link, Math, Note, Plain, RawInline, Ref, Side,
    SmallCaps, SoftBreak, Span, SpanNode, Str, Strikeout, Strong, Subscript, Superscript, Target,
    Underline, Var,
};
use tmark_markdown::mdast::{Node, TmarkMarkKind};
use tmark_markdown::tmark::looks_like_attributes;

use super::head::{parse_attrs, parse_ref_items, parse_role_head, RoleHead};
use super::{plain_text, Ctx, Lowerer};

/// What a brace group is, decided by its text (spec §Roles: "a parser
/// decides after reading one token").
enum BraceKind {
    Moustache(Vec<String>),
    Attrs(Attrs),
    Role(RoleHead),
    Literal,
}

fn classify(node: &tmark_markdown::mdast::TmarkBrace) -> BraceKind {
    if node.moustache {
        let path = node.value.trim();
        let ok = !path.is_empty()
            && path.split('.').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            });
        return if ok {
            BraceKind::Moustache(path.split('.').map(str::to_string).collect())
        } else {
            BraceKind::Literal
        };
    }
    let mut probe = String::with_capacity(node.value.len() + 2);
    probe.push('{');
    probe.push_str(&node.value);
    probe.push('}');
    if looks_like_attributes(probe.as_bytes(), 0) {
        return parse_attrs(&node.value).map_or(BraceKind::Literal, BraceKind::Attrs);
    }
    parse_role_head(&node.value).map_or(BraceKind::Literal, BraceKind::Role)
}

/// Inline lowering result: the inlines and, when the last node was an
/// attribute list that no inline could host, that list for the block to take
/// (headings, captions, block quotes).
pub(crate) struct Lowered {
    pub inlines: Vec<Inline>,
    pub tail_attrs: Option<(Attrs, Span)>,
}

impl Lowerer {
    pub fn lower_inlines(&mut self, nodes: &[Node], ctx: &Ctx) -> Lowered {
        let mut out: Vec<Inline> = Vec::new();
        let mut tail_attrs = None;
        let mut index = 0;
        while index < nodes.len() {
            let node = &nodes[index];
            index += 1;
            match node {
                Node::Text(text) => {
                    self.lower_text(&text.value, text.position.as_ref(), ctx, &mut out)
                }
                Node::Emphasis(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let content = self.lower_inlines(&n.children, ctx).inlines;
                    out.push(Inline::Emph(Emph { meta, content }));
                }
                Node::Strong(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let content = self.lower_inlines(&n.children, ctx).inlines;
                    let underscore = ctx.slice(n.position.as_ref()).starts_with("__");
                    if underscore && !self.options.strict {
                        out.push(Inline::SmallCaps(SmallCaps { meta, content }));
                    } else {
                        out.push(Inline::Strong(Strong { meta, content }));
                    }
                }
                Node::Delete(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let content = self.lower_inlines(&n.children, ctx).inlines;
                    out.push(Inline::Strikeout(Strikeout { meta, content }));
                }
                Node::InlineCode(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let (lang, text) = match n.value.strip_prefix("#!") {
                        Some(rest) => match rest.split_once(' ') {
                            Some((lang, code)) if !lang.is_empty() => {
                                (Some(lang.to_string()), code.to_string())
                            }
                            _ => (None, n.value.clone()),
                        },
                        None => (None, n.value.clone()),
                    };
                    out.push(Inline::Code(CodeInline { meta, text, lang }));
                }
                Node::InlineMath(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let display = ctx.slice(n.position.as_ref()).starts_with("$$");
                    out.push(Inline::Math(Math {
                        meta,
                        text: n.value.clone(),
                        display,
                    }));
                }
                Node::Break(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    out.push(Inline::LineBreak(LineBreak { meta }));
                }
                Node::Link(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let content = self.lower_inlines(&n.children, ctx).inlines;
                    out.push(Inline::Link(Link {
                        meta,
                        content,
                        target: target(&n.url),
                        title: n.title.clone(),
                    }));
                }
                Node::LinkReference(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let content = self.lower_inlines(&n.children, ctx).inlines;
                    let (url, title) = self
                        .definitions
                        .get(&n.identifier)
                        .cloned()
                        .unwrap_or_default();
                    out.push(Inline::Link(Link {
                        meta,
                        content,
                        target: target(&url),
                        title,
                    }));
                }
                Node::Image(n) => {
                    let span = self.span(ctx, n.position.as_ref());
                    let meta = self.meta(span);
                    let alt = self.lower_fragment(&n.alt, span);
                    let attrs =
                        self.take_adjacent_attrs(nodes, &mut index, n.position.as_ref(), ctx);
                    out.push(Inline::Image(Image {
                        meta,
                        src: n.url.clone(),
                        alt,
                        attrs,
                    }));
                }
                Node::ImageReference(n) => {
                    let span = self.span(ctx, n.position.as_ref());
                    let meta = self.meta(span);
                    let alt = self.lower_fragment(&n.alt, span);
                    let (url, _) = self
                        .definitions
                        .get(&n.identifier)
                        .cloned()
                        .unwrap_or_default();
                    let attrs =
                        self.take_adjacent_attrs(nodes, &mut index, n.position.as_ref(), ctx);
                    out.push(Inline::Image(Image {
                        meta,
                        src: url,
                        alt,
                        attrs,
                    }));
                }
                Node::Html(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    out.push(html_inline(meta, &n.value));
                }
                Node::FootnoteReference(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    out.push(Inline::Note(Note {
                        meta,
                        label: Some(n.identifier.clone()),
                        content: Vec::new(),
                    }));
                }
                Node::TmarkMark(n) => {
                    let span = self.span(ctx, n.position.as_ref());
                    let meta = self.meta(span);
                    let content = self.lower_inlines(&n.children, ctx).inlines;
                    match n.kind {
                        TmarkMarkKind::Highlight => {
                            out.push(Inline::Highlight(Highlight { meta, content }));
                        }
                        TmarkMarkKind::Superscript => {
                            out.push(Inline::Superscript(Superscript { meta, content }));
                        }
                        TmarkMarkKind::Subscript if !self.options.strict => {
                            out.push(Inline::Subscript(Subscript { meta, content }));
                        }
                        TmarkMarkKind::Insert if self.features.inline_insert => {
                            out.push(Inline::Underline(Underline { meta, content }));
                        }
                        TmarkMarkKind::Keystroke => {
                            let keys = plain_text(&content)
                                .split('+')
                                .map(str::to_string)
                                .collect();
                            out.push(Inline::Keystroke(Keystroke { meta, keys }));
                        }
                        // Off: the markers are literal text around the content.
                        TmarkMarkKind::Subscript | TmarkMarkKind::Insert => {
                            let marker = if n.kind == TmarkMarkKind::Subscript {
                                "~"
                            } else {
                                "^^"
                            };
                            out.push(self.literal_text(span, marker));
                            out.extend(content);
                            out.push(self.literal_text(span, marker));
                        }
                    }
                }
                Node::TmarkReference(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let (items, bracketed) = match n.value.strip_prefix('[') {
                        Some(inner) => (parse_ref_items(inner.trim_end_matches(']')), true),
                        None => (
                            vec![tmark_ir::RefItem {
                                key: n.value.clone(),
                                ..Default::default()
                            }],
                            false,
                        ),
                    };
                    out.push(Inline::Ref(Ref {
                        meta,
                        items,
                        bracketed,
                    }));
                }
                Node::TmarkSpan(n) => {
                    let meta = self.meta_at(ctx, n.position.as_ref());
                    let content = self.lower_inlines(&n.children, ctx).inlines;
                    let attrs =
                        self.take_adjacent_attrs(nodes, &mut index, n.position.as_ref(), ctx);
                    out.push(Inline::Span(SpanNode {
                        meta,
                        content,
                        attrs,
                    }));
                }
                Node::TmarkDefine(n) => self.lower_define(n, nodes, &mut index, ctx, &mut out),
                Node::TmarkBrace(n) => {
                    let last = index == nodes.len();
                    if let Some(attrs) = self.lower_brace(n, nodes, &mut index, ctx, &mut out, last)
                    {
                        tail_attrs = Some(attrs);
                    }
                }
                Node::TmarkGroup(n) => {
                    // A group nothing claimed: literal brackets around its content.
                    let span = self.span(ctx, n.position.as_ref());
                    out.push(self.literal_text(span, "["));
                    out.extend(self.lower_inlines(&n.children, ctx).inlines);
                    out.push(self.literal_text(span, "]"));
                }
                Node::TmarkArgument(n) => {
                    let span = self.span(ctx, n.position.as_ref());
                    out.push(self.literal_text(span, ctx.slice(n.position.as_ref())));
                }
                other => {
                    // MDX and anything unexpected: the source text, literally.
                    let position = other.position();
                    if position.is_some() {
                        let span = self.span(ctx, position);
                        out.push(self.literal_text(span, ctx.slice(position)));
                    }
                }
            }
        }
        Lowered {
            inlines: out,
            tail_attrs,
        }
    }

    /// Text with soft breaks split out.
    fn lower_text(
        &mut self,
        value: &str,
        position: Option<&tmark_markdown::unist::Position>,
        ctx: &Ctx,
        out: &mut Vec<Inline>,
    ) {
        let span = self.span(ctx, position);
        let mut offset = 0usize;
        for (i, piece) in value.split('\n').enumerate() {
            if i > 0 {
                let at = span.start + offset as u32;
                let meta = self.meta(Span::new(self.file, at, at + 1));
                out.push(Inline::SoftBreak(SoftBreak { meta }));
                offset += 1;
            }
            if !piece.is_empty() {
                let start = span.start + offset as u32;
                let meta = self.meta(Span::new(self.file, start, start + piece.len() as u32));
                out.push(Inline::Str(Str {
                    meta,
                    text: piece.to_string(),
                }));
                offset += piece.len();
            }
        }
    }

    /// If the next node is an attribute list right after `position`, take it.
    fn take_adjacent_attrs(
        &mut self,
        nodes: &[Node],
        index: &mut usize,
        position: Option<&tmark_markdown::unist::Position>,
        _ctx: &Ctx,
    ) -> Attrs {
        if let Some(Node::TmarkBrace(brace)) = nodes.get(*index) {
            let adjacent = match (position, brace.position.as_ref()) {
                (Some(host), Some(b)) => host.end.offset == b.start.offset,
                _ => false,
            };
            if adjacent && !brace.moustache {
                if let BraceKind::Attrs(attrs) = classify(brace) {
                    *index += 1;
                    return attrs;
                }
            }
        }
        Attrs::new()
    }

    /// A brace group: moustache, attribute list, role head, or literal.
    /// Returns an attribute list the block should host when the group is the
    /// last inline and no inline could host it.
    fn lower_brace(
        &mut self,
        node: &tmark_markdown::mdast::TmarkBrace,
        nodes: &[Node],
        index: &mut usize,
        ctx: &Ctx,
        out: &mut Vec<Inline>,
        last: bool,
    ) -> Option<(Attrs, Span)> {
        let span = self.span(ctx, node.position.as_ref());
        match classify(node) {
            BraceKind::Moustache(path) => {
                let meta = self.meta(span);
                out.push(Inline::Var(Var { meta, path }));
            }
            BraceKind::Attrs(attrs) => {
                // `{margin}[…]{l}`-style suffixes are handled by the role;
                // here an attribute list needs a host: the previous inline
                // when adjacent, else the block when last, else nowhere.
                if last {
                    trim_trailing_space(out);
                    return Some((attrs, span));
                }
                self.diag(
                    Code::AttrNoHost,
                    span,
                    "attribute list with no host element",
                );
                out.push(self.literal_text(span, ctx.slice(node.position.as_ref())));
            }
            BraceKind::Role(head) => self.lower_role(node, head, nodes, index, ctx, out),
            BraceKind::Literal => {
                out.push(self.literal_text(span, ctx.slice(node.position.as_ref())));
            }
        }
        None
    }

    fn lower_role(
        &mut self,
        node: &tmark_markdown::mdast::TmarkBrace,
        head: RoleHead,
        nodes: &[Node],
        index: &mut usize,
        ctx: &Ctx,
        out: &mut Vec<Inline>,
    ) {
        let head_span = self.span(ctx, node.position.as_ref());
        let Some(role) = registry::role(&head.name) else {
            // Unknown name: literal text (spec §Roles), a hint when it was
            // followed by a group or an argument (design C4).
            if matches!(
                nodes.get(*index),
                Some(Node::TmarkGroup(_) | Node::TmarkArgument(_))
            ) {
                self.diag(
                    Code::RoleUnknown,
                    head_span,
                    format!("`{}` is not a role; the brace group is literal", head.name),
                );
            }
            out.push(self.literal_text(head_span, ctx.slice(node.position.as_ref())));
            return;
        };

        // Collect what follows according to the argument style.
        let mut groups: Vec<&tmark_markdown::mdast::TmarkGroup> = Vec::new();
        let mut argument: Option<&tmark_markdown::mdast::TmarkArgument> = None;
        match role.arg {
            ArgStyle::Content | ArgStyle::ContentMany => {
                let max = if role.arg == ArgStyle::Content { 1 } else { 3 };
                while groups.len() < max {
                    match nodes.get(*index) {
                        Some(Node::TmarkGroup(g)) => {
                            groups.push(g);
                            *index += 1;
                        }
                        _ => break,
                    }
                }
            }
            ArgStyle::Argument => {
                if let Some(Node::TmarkArgument(a)) = nodes.get(*index) {
                    argument = Some(a);
                    *index += 1;
                }
            }
        }
        if groups.is_empty() && argument.is_none() {
            let what = if role.arg == ArgStyle::Argument {
                "(argument)"
            } else {
                "[content]"
            };
            self.diag(
                Code::RoleDanglingHead,
                head_span,
                format!("role `{}` needs {what} right after its head", head.name),
            );
            out.push(self.literal_text(head_span, ctx.slice(node.position.as_ref())));
            return;
        }

        // The span of the whole role: head to last group or argument.
        let end = groups
            .last()
            .and_then(|g| g.position.as_ref())
            .or_else(|| argument.and_then(|a| a.position.as_ref()))
            .map_or(head_span.end, |p| ctx.map.translate(p.end.offset) as u32);
        let span = Span::new(self.file, head_span.start, end);
        let meta = self.meta(span);

        if let Some(replacement) = role.replaced_by {
            self.deprecated(
                span,
                &format!("{{{}}}", head.name),
                &format!("{{{replacement}}}"),
            );
        }
        if let Some(suffix) = &head.registry_suffix {
            self.deprecated(
                head_span,
                &format!("{{{}:{suffix}}}", head.name),
                &format!("{{{} registry={suffix}}}", head.name),
            );
        }
        let key = |name: &str| -> Option<String> {
            head.kv
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .or_else(|| {
                    if role.principal == Some(name) {
                        head.positional.clone()
                    } else {
                        None
                    }
                })
        };
        let group_content = |this: &mut Self, g: &tmark_markdown::mdast::TmarkGroup| {
            this.lower_inlines(&g.children, ctx).inlines
        };
        let group_source = |g: &tmark_markdown::mdast::TmarkGroup| -> String {
            let s = ctx.slice(g.position.as_ref());
            s[1..s.len() - 1].to_string()
        };

        let inline = match role.name {
            "lead" => {
                // A lead-in mid-paragraph has no meaning: keep it visible as
                // a bold run-in. Paragraph-initial lead-ins are taken by the
                // block lowering before we get here.
                let content = group_content(self, groups[0]);
                Inline::Strong(Strong { meta, content })
            }
            "sc" => Inline::SmallCaps(SmallCaps {
                meta,
                content: group_content(self, groups[0]),
            }),
            "del" => Inline::Strikeout(Strikeout {
                meta,
                content: group_content(self, groups[0]),
            }),
            "underline" => Inline::Underline(Underline {
                meta,
                content: group_content(self, groups[0]),
            }),
            "mark" => Inline::Highlight(Highlight {
                meta,
                content: group_content(self, groups[0]),
            }),
            "sub" => Inline::Subscript(Subscript {
                meta,
                content: group_content(self, groups[0]),
            }),
            "sup" => Inline::Superscript(Superscript {
                meta,
                content: group_content(self, groups[0]),
            }),
            "keys" => Inline::Keystroke(Keystroke {
                meta,
                keys: group_source(groups[0])
                    .split('+')
                    .map(str::to_string)
                    .collect(),
            }),
            "code" => Inline::Code(CodeInline {
                meta,
                text: group_source(groups[0]),
                lang: key("lang"),
            }),
            "aside" | "margin" => {
                let mut side = key("side").as_deref().and_then(parse_side);
                // Deprecated `{margin}[…]{l}` suffix.
                if role.name == "margin" {
                    if let Some(Node::TmarkBrace(suffix)) = nodes.get(*index) {
                        if let Some(s) = parse_side(&suffix.value) {
                            side = Some(s);
                            *index += 1;
                        }
                    }
                }
                let content = group_content(self, groups[0]);
                let plain_meta = self.meta(span);
                Inline::Aside(Aside {
                    meta,
                    content: vec![Block::Plain(Plain {
                        meta: plain_meta,
                        content,
                    })],
                    side,
                })
            }
            "index" => {
                let path = groups.iter().map(|g| group_content(self, g)).collect();
                Inline::IndexEntry(IndexEntry {
                    meta,
                    path,
                    main: key("main").as_deref() == Some("true"),
                    registry: key("registry").or(head.registry_suffix.clone()),
                })
            }
            "counter" => match parse_counter(&argument.unwrap().value) {
                Some((prefix, key)) => Inline::CounterItem(CounterItem { meta, prefix, key }),
                None => {
                    self.diag(
                        Code::RoleDanglingHead,
                        span,
                        "`{counter}` takes a `prefix:key` argument",
                    );
                    self.literal_text(span, ctx.slice_range(span, self))
                }
            },
            "raw" => match key("backend") {
                Some(format) => Inline::RawInline(RawInline {
                    meta,
                    format,
                    text: argument.unwrap().value.clone(),
                }),
                None => {
                    self.diag(
                        Code::RoleDanglingHead,
                        span,
                        "`{raw}` needs a backend: `{raw latex}(…)`",
                    );
                    self.literal_text(span, ctx.slice_range(span, self))
                }
            },
            "latex" | "typst" | "html" => Inline::RawInline(RawInline {
                meta,
                format: role.name.to_string(),
                text: group_source(groups[0]),
            }),
            "include" => {
                self.diag(
                    Code::IncludeInline,
                    span,
                    "`{include}` is a block: put it alone on its line",
                );
                self.literal_text(span, ctx.slice_range(span, self))
            }
            _ => self.literal_text(span, ctx.slice_range(span, self)),
        };
        out.push(inline);
    }

    /// `#[…]…` index entries and `#(prefix:key)` counter items.
    fn lower_define(
        &mut self,
        node: &tmark_markdown::mdast::TmarkDefine,
        nodes: &[Node],
        index: &mut usize,
        ctx: &Ctx,
        out: &mut Vec<Inline>,
    ) {
        let sigil_span = self.span(ctx, node.position.as_ref());
        if let Some(Node::TmarkArgument(argument)) = node.children.first() {
            let span = Span::new(self.file, sigil_span.start, sigil_span.end);
            match parse_counter(&argument.value) {
                Some((prefix, key)) => {
                    if argument.marker == b'{' {
                        // Deprecated `#{prefix:key}`: only for a declared
                        // prefix (design C16); otherwise literal text.
                        if !self.is_prefix(&prefix) {
                            out.push(self.literal_text(span, ctx.slice(node.position.as_ref())));
                            return;
                        }
                        self.deprecated(span, "#{prefix:key}", "#(prefix:key)");
                    }
                    let meta = self.meta(span);
                    out.push(Inline::CounterItem(CounterItem { meta, prefix, key }));
                }
                None => out.push(self.literal_text(span, ctx.slice(node.position.as_ref()))),
            }
            return;
        }
        // Index entry: one to three groups follow.
        let mut groups: Vec<&tmark_markdown::mdast::TmarkGroup> = Vec::new();
        while groups.len() < 3 {
            match nodes.get(*index) {
                Some(Node::TmarkGroup(g)) => {
                    groups.push(g);
                    *index += 1;
                }
                _ => break,
            }
        }
        if groups.is_empty() {
            out.push(self.literal_text(sigil_span, "#"));
            return;
        }
        let end = groups
            .last()
            .and_then(|g| g.position.as_ref())
            .map_or(sigil_span.end, |p| ctx.map.translate(p.end.offset) as u32);
        let span = Span::new(self.file, sigil_span.start, end);
        let meta = self.meta(span);
        let mut main = false;
        let path: Vec<Vec<Inline>> = groups
            .iter()
            .map(|g| {
                let mut content = self.lower_inlines(&g.children, ctx).inlines;
                // `#[**term**]` is sugar for `main=true` (spec §IndexEntry).
                if groups.len() == 1 && content.len() == 1 {
                    if let Inline::Strong(strong) = &content[0] {
                        main = true;
                        content = strong.content.clone();
                    }
                }
                content
            })
            .collect();
        out.push(Inline::IndexEntry(IndexEntry {
            meta,
            path,
            main,
            registry: None,
        }));
    }
}

impl<'a> Ctx<'a> {
    /// Source text of a span already translated to the file (only valid for
    /// the identity map, where offsets are file offsets; for re-parsed text
    /// the caller passes the local text). Used for literal fallbacks.
    fn slice_range(&self, span: Span, lowerer: &Lowerer) -> String {
        let _ = lowerer;
        // Translate back: find the local range whose translation is `span`.
        // Only the identity map can do that exactly; approximate with the
        // whole text otherwise (the text is short: a role).
        let start = span.start as usize;
        let end = span.end as usize;
        if self.map.translate(0) == 0 && end <= self.text.len() {
            self.text[start..end].to_string()
        } else {
            self.text.to_string()
        }
    }
}

fn target(url: &str) -> Target {
    if let Some(anchor) = url.strip_prefix('#') {
        Target::Anchor(anchor.to_string())
    } else if url.ends_with(".md") && !url.contains("://") {
        Target::Document(url.to_string())
    } else {
        Target::Url(url.to_string())
    }
}

fn html_inline(meta: tmark_ir::Meta, value: &str) -> Inline {
    match value
        .strip_prefix("<!--")
        .and_then(|rest| rest.strip_suffix("-->"))
    {
        Some(text) => Inline::Comment(Comment {
            meta,
            text: text.to_string(),
        }),
        None => Inline::RawInline(RawInline {
            meta,
            format: "html".to_string(),
            text: value.to_string(),
        }),
    }
}

fn parse_side(value: &str) -> Option<Side> {
    match value {
        "left" | "l" => Some(Side::Left),
        "right" | "r" => Some(Side::Right),
        "outer" | "o" => Some(Side::Outer),
        "inner" | "i" => Some(Side::Inner),
        _ => None,
    }
}

/// `prefix:key` of a counter item.
pub(crate) fn parse_counter(value: &str) -> Option<(String, String)> {
    let (prefix, key) = value.split_once(':')?;
    let prefix_ok = prefix
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic())
        && prefix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    let key_ok = !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'));
    (prefix_ok && key_ok).then(|| (prefix.to_string(), key.to_string()))
}

/// Drop trailing whitespace before an attribute list taken by the block.
pub(crate) fn trim_trailing_space(inlines: &mut Vec<Inline>) {
    if let Some(Inline::Str(s)) = inlines.last_mut() {
        let trimmed = s.text.trim_end().len();
        if trimmed == 0 {
            inlines.pop();
        } else {
            s.text.truncate(trimmed);
        }
    }
}
