//! Pre-order traversal of a document.
//!
//! Design: `design/03-ir.md` §Walking and mapping: "matching on the enum is
//! the visitor". Only `walk` is provided; `map` waits for its first user.

use crate::node::{Block, Document, Inline, Row};
use crate::span::{NodeId, Span};

/// A borrowed node handed to a walker.
#[derive(Copy, Clone, Debug)]
pub enum NodeRef<'a> {
    Block(&'a Block),
    Inline(&'a Inline),
}

impl NodeRef<'_> {
    pub fn id(self) -> NodeId {
        match self {
            NodeRef::Block(b) => b.meta().id,
            NodeRef::Inline(i) => i.meta().id,
        }
    }

    pub fn span(self) -> Span {
        match self {
            NodeRef::Block(b) => b.meta().span,
            NodeRef::Inline(i) => i.meta().span,
        }
    }
}

/// Visits every block and inline of the document in pre-order: the body,
/// then the footnote definitions. Table cells, captions, admonition titles,
/// image alt texts and index paths are descended into.
pub fn walk(doc: &Document, f: &mut impl FnMut(NodeRef)) {
    walk_blocks(&doc.blocks, f);
    for footnote in &doc.footnotes {
        walk_blocks(&footnote.content, f);
    }
}

pub fn walk_blocks(blocks: &[Block], f: &mut impl FnMut(NodeRef)) {
    for block in blocks {
        walk_block(block, f);
    }
}

pub fn walk_inlines(inlines: &[Inline], f: &mut impl FnMut(NodeRef)) {
    for inline in inlines {
        walk_inline(inline, f);
    }
}

fn walk_block(block: &Block, f: &mut impl FnMut(NodeRef)) {
    f(NodeRef::Block(block));
    match block {
        Block::Para(n) => {
            if let Some(lead) = &n.lead {
                walk_inlines(lead, f);
            }
            walk_inlines(&n.content, f);
        }
        Block::Plain(n) => walk_inlines(&n.content, f),
        Block::Header(n) => walk_inlines(&n.content, f),
        Block::BlockQuote(n) => walk_blocks(&n.content, f),
        Block::BulletList(n) => {
            for item in &n.items {
                walk_blocks(&item.content, f);
            }
        }
        Block::OrderedList(n) => {
            for item in &n.items {
                walk_blocks(&item.content, f);
            }
        }
        Block::DefinitionList(n) => {
            for (term, definitions) in &n.items {
                walk_inlines(term, f);
                for definition in definitions {
                    walk_blocks(definition, f);
                }
            }
        }
        Block::Table(n) => {
            for row in n.model.rows.iter().chain(&n.model.footer) {
                if let Row::Data(row) = row {
                    for cell in &row.cells {
                        walk_inlines(&cell.content, f);
                    }
                }
            }
        }
        Block::Caption(n) => walk_inlines(&n.content, f),
        Block::Figure(n) => walk_blocks(&n.content, f),
        Block::Admonition(n) => {
            if let Some(title) = &n.title {
                walk_inlines(title, f);
            }
            walk_blocks(&n.content, f);
        }
        Block::Div(n) => walk_blocks(&n.content, f),
        Block::CodeBlock(_)
        | Block::HorizontalRule(_)
        | Block::TableConfig(_)
        | Block::MathBlock(_)
        | Block::RawBlock(_)
        | Block::Include(_)
        | Block::Comment(_) => {}
    }
}

fn walk_inline(inline: &Inline, f: &mut impl FnMut(NodeRef)) {
    f(NodeRef::Inline(inline));
    match inline {
        Inline::Emph(n) => walk_inlines(&n.content, f),
        Inline::Strong(n) => walk_inlines(&n.content, f),
        Inline::Strikeout(n) => walk_inlines(&n.content, f),
        Inline::Underline(n) => walk_inlines(&n.content, f),
        Inline::Highlight(n) => walk_inlines(&n.content, f),
        Inline::Subscript(n) => walk_inlines(&n.content, f),
        Inline::Superscript(n) => walk_inlines(&n.content, f),
        Inline::SmallCaps(n) => walk_inlines(&n.content, f),
        Inline::Quoted(n) => walk_inlines(&n.content, f),
        Inline::Link(n) => walk_inlines(&n.content, f),
        Inline::Note(n) => walk_blocks(&n.content, f),
        Inline::Image(n) => walk_inlines(&n.alt, f),
        Inline::IndexEntry(n) => {
            for level in &n.path {
                walk_inlines(level, f);
            }
        }
        Inline::Aside(n) => walk_blocks(&n.content, f),
        Inline::Span(n) => walk_inlines(&n.content, f),
        Inline::Str(_)
        | Inline::Space(_)
        | Inline::SoftBreak(_)
        | Inline::LineBreak(_)
        | Inline::Code(_)
        | Inline::Math(_)
        | Inline::Ref(_)
        | Inline::CounterItem(_)
        | Inline::Keystroke(_)
        | Inline::Var(_)
        | Inline::Abbr(_)
        | Inline::Comment(_)
        | Inline::RawInline(_) => {}
    }
}
