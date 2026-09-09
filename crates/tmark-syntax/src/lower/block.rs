//! Block lowering: mdast flow nodes to `Block`, the data directives, the
//! re-parsed bodies, and the passes that need neighbours (captions,
//! definition lists, abbreviations, includes, lead-ins).

use tmark_ir::{
    registry, AbbrDef, Admonition, Aside, Attrs, Block, BlockQuote, BulletList, Caption,
    CaptionKind, CaptionPosition, Code, CodeBlock, Comment, DefinitionList, Div, Document, Figure,
    Footnote, Header, HorizontalRule, Image, Include, Inline, ListItem, ListStyle, MathBlock,
    OrderedList, Para, RawBlock, Span, Table, TableConfig, Task,
};
use tmark_markdown::mdast::Node;

use super::head::{parse_admonition_info, parse_attrs, parse_container_info, parse_fence_info};
use super::inline::trim_trailing_space;
use super::{plain_text, Ctx, Lowerer};

/// A block with the mdast node it came from, for the neighbour passes.
enum Item {
    Block(Block),
    /// A definition item body; paired with the paragraph before it.
    Definition(Vec<Block>, Span),
}

impl Lowerer {
    pub fn lower_blocks(
        &mut self,
        nodes: &[Node],
        ctx: &Ctx,
        document: &mut Document,
    ) -> Vec<Block> {
        let mut items: Vec<Item> = Vec::new();
        for node in nodes {
            self.lower_block(node, ctx, document, &mut items);
        }
        let blocks = self.pair_definitions(items);
        self.attach_captions(blocks)
    }

    fn lower_block(
        &mut self,
        node: &Node,
        ctx: &Ctx,
        document: &mut Document,
        out: &mut Vec<Item>,
    ) {
        match node {
            Node::Paragraph(p) => {
                let span = self.span(ctx, p.position.as_ref());
                if let Some(defs) = abbreviations(ctx.slice(p.position.as_ref())) {
                    for (key, expansion) in defs {
                        let meta = self.meta(span);
                        document.abbreviations.push(AbbrDef {
                            meta,
                            key,
                            expansion,
                        });
                    }
                    return;
                }
                if let Some(include) = self.block_include(p, ctx) {
                    out.push(Item::Block(include));
                    return;
                }
                if let Some(math) = self.display_math_paragraph(p, ctx) {
                    out.push(Item::Block(math));
                    return;
                }
                if let Some(block) = self.compat_paragraph(p, ctx) {
                    out.push(Item::Block(block));
                    return;
                }
                let meta = self.meta(span);
                let lowered = self.lower_inlines(&p.children, ctx);
                let mut content = lowered.inlines;
                let lead = self.take_lead(&mut content);
                if let Some((attrs, attrs_span)) = lowered.tail_attrs {
                    // A paragraph cannot host attributes, except through the
                    // image that is its only content (already taken) or a
                    // caption line (decided later).
                    if is_caption(&content).is_none() {
                        self.diag(
                            Code::AttrNoHost,
                            attrs_span,
                            "attribute list with no host element",
                        );
                        content.push(self.literal_text(attrs_span, attrs_text(&attrs)));
                        out.push(Item::Block(Block::Para(Para {
                            meta,
                            content,
                            lead,
                        })));
                        return;
                    }
                    out.push(Item::Block(Block::Para(Para {
                        meta,
                        content,
                        lead,
                    })));
                    // Keep the attributes for the caption pass through a side
                    // channel: a trailing `Str` is ambiguous, so re-attach by
                    // storing them on a temporary caption right away.
                    if let Some(Item::Block(Block::Para(para))) = out.pop() {
                        if let Some(caption) = self.caption_from(para, Some(attrs)) {
                            out.push(Item::Block(Block::Caption(caption)));
                        }
                    }
                    return;
                }
                out.push(Item::Block(Block::Para(Para {
                    meta,
                    content,
                    lead,
                })));
            }
            Node::Heading(h) => {
                let meta = self.meta_at(ctx, h.position.as_ref());
                let lowered = self.lower_inlines(&h.children, ctx);
                let mut content = lowered.inlines;
                let attrs = lowered.tail_attrs.map_or_else(Attrs::new, |(a, _)| a);
                trim_trailing_space(&mut content);
                out.push(Item::Block(Block::Header(Header {
                    meta,
                    level: h.depth,
                    content,
                    attrs,
                })));
            }
            Node::Code(code) => {
                let block = self.lower_fence(code, ctx);
                out.push(Item::Block(block));
            }
            Node::Math(math) => {
                let meta = self.meta_at(ctx, math.position.as_ref());
                let attrs = math
                    .meta
                    .as_deref()
                    .map(str::trim)
                    .and_then(|m| m.strip_prefix('{'))
                    .and_then(|m| m.strip_suffix('}'))
                    .and_then(parse_attrs)
                    .unwrap_or_default();
                out.push(Item::Block(Block::MathBlock(MathBlock {
                    meta,
                    text: math.value.clone(),
                    attrs,
                })));
            }
            Node::Blockquote(q) => {
                let meta = self.meta_at(ctx, q.position.as_ref());
                let mut content = self.lower_blocks(&q.children, ctx, document);
                // `{.epigraph}` at the end of the quote.
                let mut attrs = Attrs::new();
                if let Some(Block::Para(para)) = content.last_mut() {
                    if let Some(taken) = take_trailing_attrs(&mut para.content) {
                        attrs = taken;
                    }
                }
                out.push(Item::Block(Block::BlockQuote(BlockQuote {
                    meta,
                    content,
                    attrs,
                })));
            }
            Node::List(list) => {
                let meta = self.meta_at(ctx, list.position.as_ref());
                let mut items = Vec::new();
                for child in &list.children {
                    let Node::ListItem(item) = child else {
                        continue;
                    };
                    let mut content = self.lower_blocks(&item.children, ctx, document);
                    let mut task = item
                        .checked
                        .map(|c| if c { Task::Done } else { Task::Open });
                    if task.is_none() && self.features.tasklist_partial {
                        task = take_partial_task(&mut content);
                    }
                    items.push(ListItem { content, task });
                }
                if list.ordered {
                    out.push(Item::Block(Block::OrderedList(OrderedList {
                        meta,
                        items,
                        start: list.start.unwrap_or(1),
                        style: ListStyle::Decimal,
                    })));
                } else {
                    out.push(Item::Block(Block::BulletList(BulletList { meta, items })));
                }
            }
            Node::ThematicBreak(t) => {
                let meta = self.meta_at(ctx, t.position.as_ref());
                out.push(Item::Block(Block::HorizontalRule(HorizontalRule { meta })));
            }
            Node::Html(h) => {
                let meta = self.meta_at(ctx, h.position.as_ref());
                let value = h.value.trim();
                let block = match value
                    .strip_prefix("<!--")
                    .and_then(|rest| rest.strip_suffix("-->"))
                {
                    Some(text) if !text.contains("-->") => Block::Comment(Comment {
                        meta,
                        text: text.to_string(),
                    }),
                    _ => Block::RawBlock(RawBlock {
                        meta,
                        format: "html".to_string(),
                        text: h.value.clone(),
                    }),
                };
                out.push(Item::Block(block));
            }
            Node::Table(table) => {
                let meta = self.meta_at(ctx, table.position.as_ref());
                let model = self.lower_pipe_table(table, ctx);
                out.push(Item::Block(Block::Table(Table {
                    meta,
                    model,
                    attrs: Attrs::new(),
                })));
            }
            Node::FootnoteDefinition(def) => {
                let meta = self.meta_at(ctx, def.position.as_ref());
                let content = self.lower_blocks(&def.children, ctx, document);
                document.footnotes.push(Footnote {
                    meta,
                    label: def.identifier.clone(),
                    content,
                });
            }
            Node::Definition(_) | Node::Yaml(_) | Node::Toml(_) => {}
            Node::TmarkContainer(c) => {
                let block = self.lower_container(c, ctx, document);
                out.push(Item::Block(block));
            }
            Node::TmarkAdmonition(a) => {
                let span = self.span(ctx, a.position.as_ref());
                let meta = self.meta(span);
                let (kind, classes, title) = parse_admonition_info(&a.info);
                let mut attrs = Attrs::new();
                attrs.classes = classes;
                if a.marker.starts_with("???") {
                    attrs.kv.push((
                        "collapsed".to_string(),
                        (!a.marker.ends_with('+')).to_string(),
                    ));
                }
                let title = title.map(|t| self.lower_fragment(&t, span));
                let content = self.lower_content(&a.value, &a.stops, ctx, document);
                out.push(Item::Block(Block::Admonition(Admonition {
                    meta,
                    kind,
                    title,
                    content,
                    attrs,
                })));
            }
            Node::TmarkDefinition(d) => {
                let span = self.span(ctx, d.position.as_ref());
                let content = self.lower_content(&d.value, &d.stops, ctx, document);
                out.push(Item::Definition(content, span));
            }
            other => {
                // Anything else (MDX flow) becomes a paragraph of its source.
                if let Some(position) = other.position() {
                    let span = self.span(ctx, Some(position));
                    let meta = self.meta(span);
                    let text = self.literal_text(span, ctx.slice(Some(position)));
                    out.push(Item::Block(Block::Para(Para {
                        meta,
                        content: vec![text],
                        lead: None,
                    })));
                }
            }
        }
    }

    /// `{include}(path)` alone in a paragraph is a block include.
    fn block_include(&mut self, p: &tmark_markdown::mdast::Paragraph, ctx: &Ctx) -> Option<Block> {
        let (Node::TmarkBrace(head), Node::TmarkArgument(arg)) =
            (p.children.first()?, p.children.get(1)?)
        else {
            return None;
        };
        if p.children.len() != 2 || head.moustache {
            return None;
        }
        let parsed = super::head::parse_role_head(&head.value)?;
        if parsed.name != "include" {
            return None;
        }
        let meta = self.meta_at(ctx, p.position.as_ref());
        let base = parsed
            .kv
            .iter()
            .find(|(k, _)| k == "base")
            .map(|(_, v)| v.clone());
        Some(Block::Include(Include {
            meta,
            path: arg.value.clone(),
            base,
        }))
    }

    /// `$$ … $$ {#eq:x}` on one line: a display math paragraph is a math
    /// block, the attribute list its anchor (spec §Math (display), design C13).
    fn display_math_paragraph(
        &mut self,
        p: &tmark_markdown::mdast::Paragraph,
        ctx: &Ctx,
    ) -> Option<Block> {
        let mut children = p
            .children
            .iter()
            .filter(|n| !matches!(n, Node::Text(t) if t.value.trim().is_empty()));
        let Node::InlineMath(math) = children.next()? else {
            return None;
        };
        if !ctx.slice(math.position.as_ref()).starts_with("$$") {
            return None;
        }
        let attrs = match children.next() {
            None => Attrs::new(),
            Some(Node::TmarkBrace(brace)) if !brace.moustache => {
                let attrs = parse_attrs(&brace.value)?;
                if children.next().is_some() {
                    return None;
                }
                attrs
            }
            Some(_) => return None,
        };
        let meta = self.meta_at(ctx, p.position.as_ref());
        Some(Block::MathBlock(MathBlock {
            meta,
            text: math.value.clone(),
            attrs,
        }))
    }

    /// Compatibility spellings that take a whole paragraph: `\[ … \]`
    /// display math (spec §Math (display)) and the deprecated PyMdownX
    /// snippet `--8<-- "file"` (spec §Includes).
    fn compat_paragraph(
        &mut self,
        p: &tmark_markdown::mdast::Paragraph,
        ctx: &Ctx,
    ) -> Option<Block> {
        let source = ctx.slice(p.position.as_ref()).trim();
        if let Some(inner) = source
            .strip_prefix("\\[")
            .and_then(|s| s.strip_suffix("\\]"))
        {
            let meta = self.meta_at(ctx, p.position.as_ref());
            return Some(Block::MathBlock(MathBlock {
                meta,
                text: inner.trim_matches('\n').to_string(),
                attrs: Attrs::new(),
            }));
        }
        if let Some(rest) = source.strip_prefix("--8<--") {
            let rest = rest.trim();
            let path = rest.trim_matches('"');
            let quoted = rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2;
            if !path.is_empty() && (quoted || !rest.contains(char::is_whitespace)) {
                let span = self.span(ctx, p.position.as_ref());
                self.deprecated(span, "--8<-- \"file\"", "{include}(file)");
                let meta = self.meta(span);
                return Some(Block::Include(Include {
                    meta,
                    path: path.to_string(),
                    base: None,
                }));
            }
        }
        None
    }

    /// A paragraph-initial `{lead}[…]` role, or the `paragraph.lead`
    /// promotion of a short leading strong span (spec §Para).
    fn take_lead(&mut self, content: &mut Vec<Inline>) -> Option<Vec<Inline>> {
        if content.len() < 2 {
            return None;
        }
        // `{lead}[…]` lowers to `Strong` mid-paragraph; at the start it is the
        // lead-in whatever its length. The promotion applies to any strong.
        match &content[0] {
            Inline::Strong(strong) => {
                let short = plain_text(&strong.content).chars().count() < 80;
                let followed_by_text =
                    matches!(&content[1], Inline::Str(s) if s.text.starts_with(' '));
                if short && followed_by_text && self.features.paragraph_lead {
                    let Inline::Strong(strong) = content.remove(0) else {
                        unreachable!()
                    };
                    if let Some(Inline::Str(s)) = content.first_mut() {
                        s.text = s.text.trim_start().to_string();
                    }
                    return Some(strong.content);
                }
                None
            }
            _ => None,
        }
    }

    /// A fenced code block: listing or data directive (spec §Data directives).
    fn lower_fence(&mut self, code: &tmark_markdown::mdast::Code, ctx: &Ctx) -> Block {
        let span = self.span(ctx, code.position.as_ref());
        let meta = self.meta(span);
        let Some(info) = parse_fence_info(code.lang.as_deref(), code.meta.as_deref()) else {
            return Block::CodeBlock(CodeBlock {
                meta,
                text: code.value.clone(),
                lang: None,
                options: Attrs::new(),
            });
        };
        let mut options = Attrs::new();
        options.kv = info.options.clone();
        let node = info
            .node
            .clone()
            .unwrap_or_else(|| registry::default_node_word(&info.lang).word.to_string());
        let listing = |this: &mut Self, lang: Option<String>| {
            let _ = this;
            Block::CodeBlock(CodeBlock {
                meta,
                text: code.value.clone(),
                lang,
                options: options.clone(),
            })
        };
        match node.as_str() {
            "code" => listing(self, Some(info.lang.clone())),
            "table" => match info.lang.as_str() {
                "yaml" | "yml" => match self.lower_yaml_table(&code.value, span) {
                    Ok(model) => Block::Table(Table {
                        meta,
                        model,
                        attrs: options,
                    }),
                    Err(error) => {
                        self.diag(
                            Code::FenceUnknownNodeWord,
                            span,
                            format!("invalid `yaml table`: {error}"),
                        );
                        listing(self, Some(info.lang.clone()))
                    }
                },
                // Grid tables are parsed at milestone 4 (design/11-roadmap.md).
                "grid" => listing(self, Some("grid table".to_string())),
                other => {
                    self.diag(
                        Code::FenceUnknownNodeWord,
                        span,
                        format!("`{other} table` is not a data directive; use `yaml table` or `grid table`"),
                    );
                    listing(self, Some(info.lang.clone()))
                }
            },
            "table-config" => match self.lower_yaml_table_config(&code.value) {
                Ok((columns, settings)) if matches!(info.lang.as_str(), "yaml" | "yml") => {
                    Block::TableConfig(TableConfig {
                        meta,
                        columns,
                        settings,
                    })
                }
                Ok(_) | Err(_) => {
                    self.diag(
                        Code::FenceUnknownNodeWord,
                        span,
                        "`table-config` takes YAML",
                    );
                    listing(self, Some(info.lang.clone()))
                }
            },
            "image" => {
                let mut attrs = options;
                attrs
                    .kv
                    .insert(0, ("generate".to_string(), info.lang.clone()));
                attrs.kv.insert(1, ("code".to_string(), code.value.clone()));
                let image_meta = self.meta(span);
                Block::Para(Para {
                    meta,
                    content: vec![Inline::Image(Image {
                        meta: image_meta,
                        src: String::new(),
                        alt: Vec::new(),
                        attrs,
                    })],
                    lead: None,
                })
            }
            "raw" => match info.lang.as_str() {
                "latex" | "typst" | "html" => Block::RawBlock(RawBlock {
                    meta,
                    format: info.lang.clone(),
                    text: code.value.clone(),
                }),
                other => {
                    self.diag(
                        Code::FenceUnknownNodeWord,
                        span,
                        format!("`{other} raw`: `raw` needs a backend (latex, typst, html)"),
                    );
                    listing(self, Some(info.lang.clone()))
                }
            },
            _ => listing(self, Some(info.lang.clone())),
        }
    }

    /// `::: name {attrs}` … `:::` and the deprecated `/// name` … `///`.
    fn lower_container(
        &mut self,
        c: &tmark_markdown::mdast::TmarkContainer,
        ctx: &Ctx,
        document: &mut Document,
    ) -> Block {
        let span = self.span(ctx, c.position.as_ref());
        let meta = self.meta(span);
        let (name, attrs, valid) = parse_container_info(&c.info);
        let mut attrs = attrs.unwrap_or_default();
        if !valid {
            self.diag(
                Code::AttrNoHost,
                span,
                "malformed attribute list on the fence",
            );
        }
        if c.marker == b'/' {
            self.deprecated(span, "/// name … ///", "::: name … :::");
        }
        if !c.closed {
            self.diag(
                Code::ContainerUnclosed,
                span,
                format!("`::: {name}` is never closed"),
            );
        }
        let content = self.lower_content(&c.value, &c.stops, ctx, document);
        let name = if name == "margin" {
            self.deprecated(span, "::: margin", "::: aside");
            "aside".to_string()
        } else {
            name
        };
        match name.as_str() {
            "figure" => Block::Figure(Figure {
                meta,
                content,
                attrs,
            }),
            "aside" => {
                let side = attrs.get("side").and_then(|s| match s {
                    "left" => Some(tmark_ir::Side::Left),
                    "right" => Some(tmark_ir::Side::Right),
                    "outer" => Some(tmark_ir::Side::Outer),
                    "inner" => Some(tmark_ir::Side::Inner),
                    _ => None,
                });
                let aside_meta = self.meta(span);
                Block::Para(Para {
                    meta,
                    content: vec![Inline::Aside(Aside {
                        meta: aside_meta,
                        content,
                        side,
                    })],
                    lead: None,
                })
            }
            kind if self.is_admonition(kind) => {
                let title = attrs
                    .kv
                    .iter()
                    .position(|(k, _)| k == "title")
                    .map(|at| attrs.kv.remove(at).1)
                    .map(|t| self.lower_fragment(&t, span));
                Block::Admonition(Admonition {
                    meta,
                    kind: kind.to_string(),
                    title,
                    content,
                    attrs,
                })
            }
            _ => {
                self.diag(
                    Code::ContainerUnknown,
                    span,
                    format!("`::: {name}` is not a known container"),
                );
                Block::Div(Div {
                    meta,
                    name,
                    content,
                    attrs,
                })
            }
        }
    }

    /// Pair `:   definition` items with the paragraph before them.
    fn pair_definitions(&mut self, items: Vec<Item>) -> Vec<Block> {
        let mut out: Vec<Block> = Vec::new();
        let mut list: Option<(DefinitionList, Span)> = None;
        for item in items {
            match item {
                Item::Definition(content, span) => {
                    let term = match out.last() {
                        Some(Block::Para(_)) if list.as_ref().map_or(true, |_| true) => {
                            // A paragraph right before the definition is its term,
                            // unless it already belongs to the list as a term.
                            match out.pop() {
                                Some(Block::Para(para)) => Some(para),
                                _ => None,
                            }
                        }
                        _ => None,
                    };
                    match (&mut list, term) {
                        (Some((dl, dl_span)), Some(para)) => {
                            dl.items.push((para.content, vec![content]));
                            *dl_span = dl_span.join(span);
                        }
                        (Some((dl, dl_span)), None) => {
                            if let Some(last) = dl.items.last_mut() {
                                last.1.push(content);
                            }
                            *dl_span = dl_span.join(span);
                        }
                        (None, term) => {
                            let start = term.as_ref().map_or(span, |p| p.meta.span);
                            let meta = self.meta(start.join(span));
                            let term_content = term.map(|p| p.content).unwrap_or_default();
                            list = Some((
                                DefinitionList {
                                    meta,
                                    items: vec![(term_content, vec![content])],
                                },
                                start.join(span),
                            ));
                        }
                    }
                }
                Item::Block(block) => {
                    // A paragraph may be the next term: keep the list open
                    // until something else comes.
                    if let Some((dl, dl_span)) = list.take() {
                        if matches!(block, Block::Para(_)) {
                            list = Some((dl, dl_span));
                        } else {
                            let mut dl = dl;
                            dl.meta.span = dl_span;
                            out.push(Block::DefinitionList(dl));
                        }
                    }
                    out.push(block);
                }
            }
        }
        if let Some((mut dl, dl_span)) = list {
            dl.meta.span = dl_span;
            // A paragraph left after the list was not a term.
            let trailing = matches!(out.last(), Some(Block::Para(_)));
            if trailing {
                let para = out.pop().unwrap();
                out.push(Block::DefinitionList(dl));
                out.push(para);
            } else {
                out.push(Block::DefinitionList(dl));
            }
        }
        out
    }

    /// Turn `Kind: text {#id}` paragraphs into captions attached to a
    /// neighbouring float (design C7).
    fn attach_captions(&mut self, blocks: Vec<Block>) -> Vec<Block> {
        let mut out: Vec<Block> = Vec::with_capacity(blocks.len());
        let mut iter = blocks.into_iter().peekable();
        while let Some(block) = iter.next() {
            let candidate = match &block {
                Block::Para(para) => is_caption(&para.content).is_some(),
                Block::Caption(_) => true,
                _ => false,
            };
            if !candidate {
                out.push(block);
                continue;
            }
            let previous_is_host =
                out.last().is_some_and(is_float) && !matches!(out.last(), Some(Block::Caption(_)));
            let next_is_host = iter.peek().is_some_and(is_float);
            let mut caption = match block {
                Block::Caption(c) => c,
                Block::Para(para) => match self.caption_from(para, None) {
                    Some(c) => c,
                    None => unreachable!("checked by is_caption"),
                },
                _ => unreachable!(),
            };
            if previous_is_host {
                caption.position = CaptionPosition::After;
                out.push(Block::Caption(caption));
            } else if next_is_host {
                caption.position = CaptionPosition::Before;
                out.push(Block::Caption(caption));
            } else {
                self.diag(
                    Code::CaptionNoHost,
                    caption.meta.span,
                    "caption line with no table, figure or listing next to it",
                );
                out.push(Block::Caption(caption));
            }
        }
        out
    }

    /// Build a caption from a `Kind: …` paragraph.
    fn caption_from(&mut self, para: Para, attrs: Option<Attrs>) -> Option<Caption> {
        let (kind, skip) = is_caption(&para.content)?;
        let mut content = para.content;
        if let Some(Inline::Str(first)) = content.first_mut() {
            first.text = first.text[skip..].trim_start().to_string();
            if first.text.is_empty() {
                content.remove(0);
            }
        }
        let attrs = match attrs {
            Some(a) => a,
            None => take_trailing_attrs(&mut content).unwrap_or_default(),
        };
        trim_trailing_space(&mut content);
        Some(Caption {
            meta: para.meta,
            kind,
            content,
            attrs,
            position: CaptionPosition::After,
        })
    }
}

/// `Kind:` at the start of a paragraph: the kind and the bytes to skip.
fn is_caption(content: &[Inline]) -> Option<(CaptionKind, usize)> {
    let Some(Inline::Str(first)) = content.first() else {
        return None;
    };
    for (word, kind) in [
        ("Table:", CaptionKind::Table),
        ("Figure:", CaptionKind::Figure),
        ("Listing:", CaptionKind::Listing),
    ] {
        if let Some(rest) = first.text.strip_prefix(word) {
            if rest.starts_with(' ') || rest.starts_with('\t') {
                return Some((kind, word.len()));
            }
        }
    }
    None
}

/// Blocks a caption can attach to.
fn is_float(block: &Block) -> bool {
    match block {
        Block::Table(_) | Block::Figure(_) | Block::CodeBlock(_) | Block::TableConfig(_) => true,
        Block::Para(para) => para.content.len() == 1 && matches!(para.content[0], Inline::Image(_)),
        _ => false,
    }
}

/// An attribute list left as a trailing literal `{…}` by the inline lowering
/// (block quotes take `{.epigraph}` this way).
fn take_trailing_attrs(content: &mut Vec<Inline>) -> Option<Attrs> {
    let Some(Inline::Str(last)) = content.last() else {
        return None;
    };
    let text = last.text.trim_end();
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    let attrs = parse_attrs(inner)?;
    content.pop();
    trim_trailing_space(content);
    Some(attrs)
}

/// `*[KEY]: expansion` lines (spec §Glossary and acronyms).
fn abbreviations(text: &str) -> Option<Vec<(String, String)>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let rest = line.trim().strip_prefix("*[")?;
        let (key, expansion) = rest.split_once("]:")?;
        if key.is_empty() || key.contains(']') {
            return None;
        }
        out.push((key.to_string(), expansion.trim().to_string()));
    }
    (!out.is_empty()).then_some(out)
}

/// `- [.] item` partial tasks (feature `tasklist.partial`).
fn take_partial_task(content: &mut [Block]) -> Option<Task> {
    let first = content.first_mut()?;
    let inlines = match first {
        Block::Para(p) => &mut p.content,
        Block::Plain(p) => &mut p.content,
        _ => return None,
    };
    let Some(Inline::Str(s)) = inlines.first_mut() else {
        return None;
    };
    let rest = s.text.strip_prefix("[.] ")?;
    s.text = rest.to_string();
    Some(Task::Partial)
}

/// The canonical text of an attribute list, for literal fallbacks.
fn attrs_text(attrs: &Attrs) -> String {
    let mut parts = Vec::new();
    if let Some(id) = &attrs.id {
        parts.push(format!("#{id}"));
    }
    for class in &attrs.classes {
        parts.push(format!(".{class}"));
    }
    for (k, v) in &attrs.kv {
        if v.contains(char::is_whitespace) || v.contains('}') {
            parts.push(format!("{k}=\"{v}\""));
        } else {
            parts.push(format!("{k}={v}"));
        }
    }
    format!("{{{}}}", parts.join(" "))
}
