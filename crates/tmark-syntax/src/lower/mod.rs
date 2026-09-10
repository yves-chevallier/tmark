//! Lowering: the mdast of `tmark-markdown` to the TMark IR.
//!
//! Most of TMark is decided here, not in the tokenizer: what a brace group
//! is, which role a head names, where an attribute list attaches, what a
//! fence produces, which paragraph is a caption. Every function here is
//! total: unrecognised input lowers to literal text and a diagnostic.

mod block;
mod inline;
mod table;

pub(crate) mod head;

use std::collections::HashMap;
use std::rc::Rc;

use tmark_ir::{
    frontmatter, registry, Code, Diagnostic, Document, FileId, FrontMatter, Inline, Meta, NodeId,
    Span, Str,
};
use tmark_markdown::{mdast::Node, to_mdast, unist::Position, ParseOptions};

use crate::offset::OffsetMap;
use crate::Parsed;

/// Parse options.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// The strict profile: X1 (`__x__` small caps) and X3 (`~x~` subscript)
    /// are off (spec §Conformance).
    pub strict: bool,
}

/// The text a tree was parsed from, and how its offsets map to the file.
#[derive(Clone)]
pub(crate) struct Ctx<'a> {
    pub text: &'a str,
    pub map: Rc<OffsetMap>,
}

impl<'a> Ctx<'a> {
    /// The source text of a node.
    pub fn slice(&self, position: Option<&Position>) -> &'a str {
        match position {
            Some(p) => &self.text[p.start.offset..p.end.offset],
            None => "",
        }
    }
}

/// Feature switches read from the front matter (spec §Feature registry).
#[derive(Clone, Debug)]
pub(crate) struct Features {
    pub paragraph_lead: bool,
    pub tasklist_partial: bool,
    pub inline_insert: bool,
}

impl Features {
    fn from_front_matter(fm: &FrontMatter) -> Self {
        let get = |name: &str| {
            fm.keys
                .press
                .features
                .get(name)
                .copied()
                .unwrap_or_else(|| registry::feature(name).is_some_and(|f| f.default))
        };
        Features {
            paragraph_lead: get("paragraph.lead"),
            tasklist_partial: get("tasklist.partial"),
            inline_insert: get("inline.insert"),
        }
    }
}

pub(crate) struct Lowerer {
    pub file: FileId,
    pub options: Options,
    pub features: Features,
    next_id: u32,
    pub diagnostics: Vec<Diagnostic>,
    /// Counter prefixes: predeclared plus `declare.counters`.
    pub prefixes: Vec<String>,
    /// Admonition kinds: built-in plus `declare.admonitions`.
    pub admonitions: Vec<String>,
    /// Link reference definitions of the current parse (`[id]: url "title"`).
    pub definitions: HashMap<String, (String, Option<String>)>,
    /// Lowering the body of a `::: figure`: a free caption is the figure's.
    pub in_figure: bool,
}

pub fn parse(text: &str, file: FileId) -> Parsed {
    parse_with(text, file, Options::default())
}

pub fn parse_with(text: &str, file: FileId, options: Options) -> Parsed {
    let mut lowerer = Lowerer {
        file,
        options,
        features: Features {
            paragraph_lead: true,
            tasklist_partial: false,
            inline_insert: false,
        },
        next_id: 0,
        diagnostics: Vec::new(),
        prefixes: registry::PREFIXES
            .iter()
            .map(|p| p.name.to_string())
            .collect(),
        admonitions: registry::ADMONITIONS
            .iter()
            .map(|a| a.name.to_string())
            .collect(),
        definitions: HashMap::new(),
        in_figure: false,
    };
    let ctx = Ctx {
        text,
        map: OffsetMap::identity(),
    };
    let document = match lowerer.tree(text) {
        Ok(tree) => lowerer.lower_root(&tree, &ctx),
        // Only the MDX constructs can fail, and they are off; be total anyway.
        Err(message) => {
            let span = Span::new(file, 0, text.len() as u32);
            lowerer.diag(Code::FrontmatterYaml, span, message.reason.to_string());
            let meta = lowerer.meta(span);
            let inline = Inline::Str(Str {
                meta: lowerer.meta(span),
                text: text.to_string(),
            });
            Document {
                file,
                blocks: vec![tmark_ir::Block::Para(tmark_ir::Para {
                    meta,
                    content: vec![inline],
                    lead: None,
                })],
                ..Document::default()
            }
        }
    };
    Parsed {
        document,
        diagnostics: lowerer.diagnostics,
    }
}

impl Lowerer {
    pub fn tree(&self, text: &str) -> Result<Node, tmark_markdown::message::Message> {
        to_mdast(text, &ParseOptions::tmark())
    }

    /// Allocate the next node id (dense, document order: call before
    /// lowering the children).
    pub fn meta(&mut self, span: Span) -> Meta {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        Meta::new(id, span)
    }

    pub fn span(&self, ctx: &Ctx, position: Option<&Position>) -> Span {
        match position {
            Some(p) => Span::new(
                self.file,
                ctx.map.translate(p.start.offset) as u32,
                ctx.map.translate(p.end.offset) as u32,
            ),
            None => Span::new(self.file, 0, 0),
        }
    }

    /// A span for a byte range of the current text.
    pub fn span_of(&self, ctx: &Ctx, start: usize, end: usize) -> Span {
        Span::new(
            self.file,
            ctx.map.translate(start) as u32,
            ctx.map.translate(end) as u32,
        )
    }

    pub fn meta_at(&mut self, ctx: &Ctx, position: Option<&Position>) -> Meta {
        let span = self.span(ctx, position);
        self.meta(span)
    }

    pub fn diag(&mut self, code: Code, span: Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(code, span, message));
    }

    pub fn deprecated(&mut self, span: Span, spelling: &str, canonical: &str) {
        self.diag(
            Code::Deprecated,
            span,
            format!("`{spelling}` is deprecated, write `{canonical}`"),
        );
    }

    pub fn is_prefix(&self, name: &str) -> bool {
        self.prefixes.iter().any(|p| p.eq_ignore_ascii_case(name))
    }

    pub fn is_admonition(&self, name: &str) -> bool {
        self.admonitions.iter().any(|a| a == name)
    }

    /// A literal `Str` with an explicit text (for synthesised pieces).
    pub fn literal_text(&mut self, span: Span, text: impl Into<String>) -> Inline {
        Inline::Str(Str {
            meta: self.meta(span),
            text: text.into(),
        })
    }

    fn lower_root(&mut self, root: &Node, ctx: &Ctx) -> Document {
        let children: &[Node] = root.children().map_or(&[], Vec::as_slice);
        let root_span = self.span(ctx, root.position());
        let doc_meta = self.meta(root_span);
        let _ = doc_meta;

        // Front matter.
        let mut front_matter = FrontMatter {
            meta: Meta::default(),
            raw: String::new(),
            keys: Default::default(),
            extra: serde_json::Value::Null,
            deprecated: Vec::new(),
        };
        let mut rest = children;
        if let Some(Node::Yaml(yaml)) = children.first() {
            let span = self.span(ctx, yaml.position.as_ref());
            let meta = self.meta(span);
            let raw = ctx.slice(yaml.position.as_ref()).to_string();
            match frontmatter::parse(&yaml.value) {
                Ok(mut fm) => {
                    fm.meta = meta;
                    fm.raw = raw;
                    for key in &fm.deprecated {
                        self.diag(
                            Code::DeprecatedFrontmatterKey,
                            span,
                            format!(
                                "top-level `{key}` is deprecated, move it under its `press` group"
                            ),
                        );
                    }
                    front_matter = fm;
                }
                Err(error) => {
                    self.diag(Code::FrontmatterYaml, span, error.message.clone());
                    front_matter.meta = meta;
                    front_matter.raw = raw;
                }
            }
            rest = &children[1..];
        }
        self.features = Features::from_front_matter(&front_matter);
        self.prefixes
            .extend(front_matter.keys.press.declare.counters.keys().cloned());
        self.admonitions
            .extend(front_matter.keys.press.declare.admonitions.keys().cloned());

        self.collect_definitions(rest);
        let mut document = Document {
            file: self.file,
            front_matter,
            ..Document::default()
        };
        document.blocks = self.lower_blocks(rest, ctx, &mut document);
        document
    }

    /// Link reference definitions of a parse, for reference links.
    pub fn collect_definitions(&mut self, nodes: &[Node]) {
        self.definitions.clear();
        for node in nodes {
            if let Node::Definition(def) = node {
                self.definitions
                    .insert(def.identifier.clone(), (def.url.clone(), def.title.clone()));
            }
        }
    }

    /// Parse and lower a fragment of text that has no source of its own
    /// (an image alt, a YAML cell, a title): every span points at `anchor`.
    pub fn lower_fragment(&mut self, text: &str, anchor: Span) -> Vec<Inline> {
        let Ok(tree) = self.tree(text) else {
            return vec![self.literal_text(anchor, text)];
        };
        let map = OffsetMap::nested(vec![(0, anchor.start as usize)], OffsetMap::identity());
        let ctx = Ctx { text, map };
        let mut document = Document::default();
        let saved = std::mem::take(&mut self.definitions);
        let blocks = self.lower_blocks(
            tree.children().map_or(&[], Vec::as_slice),
            &ctx,
            &mut document,
        );
        self.definitions = saved;
        match blocks.into_iter().next() {
            Some(tmark_ir::Block::Para(para)) => para.content,
            Some(tmark_ir::Block::Plain(plain)) => plain.content,
            _ => Vec::new(),
        }
    }

    /// Parse and lower re-parsed content (a container or admonition body)
    /// whose `stops` map it back to `ctx`.
    pub fn lower_content(
        &mut self,
        value: &str,
        stops: &[(usize, usize)],
        ctx: &Ctx,
        document: &mut Document,
    ) -> Vec<tmark_ir::Block> {
        let Ok(tree) = self.tree(value) else {
            return Vec::new();
        };
        let map = OffsetMap::nested(stops.to_vec(), ctx.map.clone());
        let inner = Ctx { text: value, map };
        let saved = std::mem::take(&mut self.definitions);
        let children: &[Node] = tree.children().map_or(&[], Vec::as_slice);
        self.collect_definitions(children);
        let blocks = self.lower_blocks(children, &inner, document);
        self.definitions = saved;
        blocks
    }
}

pub(crate) use tmark_ir::plain_text;

/// Source text with its backslash escapes decoded (`\+` → `+`), for text
/// that the tokenizer took raw and the lowering gives back as literal: what
/// CommonMark would have produced had the construct not been recognised.
pub(crate) fn decode_escapes(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next.is_ascii_punctuation() {
                    out.push(next);
                    chars.next();
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}
