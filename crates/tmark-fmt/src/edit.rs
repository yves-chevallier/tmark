//! Local edits: reprint one node and splice it into its span, leaving every
//! other byte untouched (design 04 §Local edits, ADR 0004).

use tmark_ir::{walk, Block, Document, Inline, NodeId, NodeRef, Span};

use crate::escape::Context;
use crate::out::Out;
use crate::{block, inline};

/// What replaces a node.
#[derive(Clone, Debug)]
pub enum Replacement {
    Block(Block),
    Inline(Inline),
}

/// Replace the node `id` of `doc` with `replacement`.
#[derive(Clone, Debug)]
pub struct NodeEdit {
    pub id: NodeId,
    pub replacement: Replacement,
}

/// The span of a node of the document, if it exists.
pub fn span_of(doc: &Document, id: NodeId) -> Option<Span> {
    let mut found = None;
    walk(doc, &mut |node: NodeRef| {
        if node.id() == id {
            found = Some(node.span());
        }
    });
    found
}

/// The canonical text of a replacement, on its own (no trailing newline).
pub fn print(replacement: &Replacement) -> String {
    let mut out = Out::new();
    match replacement {
        Replacement::Block(b) => block::block(&mut out, b),
        Replacement::Inline(i) => {
            inline::inlines(&mut out, std::slice::from_ref(i), Context::default())
        }
    }
    out.finish().trim_end_matches('\n').to_string()
}

/// Apply `edit` to `text`, the source `doc` was parsed from. Returns `text`
/// unchanged when the node is not found.
pub fn edit(text: &str, doc: &Document, edit: NodeEdit) -> String {
    let Some(span) = span_of(doc, edit.id) else {
        return text.to_string();
    };
    let (start, end) = (span.start as usize, span.end as usize);
    if end > text.len() || start > end {
        return text.to_string();
    }
    let mut result = String::with_capacity(text.len());
    result.push_str(&text[..start]);
    result.push_str(&print(&edit.replacement));
    result.push_str(&text[end..]);
    result
}
