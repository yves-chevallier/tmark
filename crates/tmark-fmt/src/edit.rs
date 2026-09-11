//! Local edits: reprint one node and splice it into its span, leaving every
//! other byte untouched (design 04 §Local edits, ADR 0004). `edit_many`
//! splices a batch of disjoint edits in one pass.

use std::fmt;

use tmark_ir::{find, Block, Document, Inline, NodeId, NodeRef, Span};

use crate::escape::Context;
use crate::out::Out;
use crate::{block, inline, Profile};

/// What replaces a node.
#[derive(Clone, Debug)]
pub enum Replacement {
    Block(Block),
    Inline(Inline),
    /// Literal text, spliced as is: what a lowering that has no IR node for
    /// its output (an HTML wrapper on a MkDocs site) splices.
    Text(String),
}

/// Replace the node `id` of `doc` with `replacement`.
#[derive(Clone, Debug)]
pub struct NodeEdit {
    pub id: NodeId,
    pub replacement: Replacement,
}

/// Why a batch of edits cannot be applied. The text is left untouched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditError {
    /// No node of the document has this id.
    NotFound(NodeId),
    /// The node's span is not inside the text (a node of an included file,
    /// or a document parsed from another text).
    OutOfRange(NodeId),
    /// Two edits touch the same bytes (one node inside the other, or the
    /// same node twice).
    Overlap(NodeId, NodeId),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EditError::NotFound(id) => write!(f, "node {} not found", id.0),
            EditError::OutOfRange(id) => write!(f, "node {} is outside the text", id.0),
            EditError::Overlap(a, b) => write!(f, "edits of nodes {} and {} overlap", a.0, b.0),
        }
    }
}

impl std::error::Error for EditError {}

/// The span of a node of the document, if it exists.
pub fn span_of(doc: &Document, id: NodeId) -> Option<Span> {
    find(doc, id).map(|node| node.span())
}

/// The canonical text of a replacement, on its own (no trailing newline).
pub fn print(replacement: &Replacement) -> String {
    match replacement {
        Replacement::Block(b) => print_node(NodeRef::Block(b)),
        Replacement::Inline(i) => print_node(NodeRef::Inline(i)),
        Replacement::Text(t) => t.clone(),
    }
}

/// The canonical text of one node of a document, on its own (no trailing
/// newline): what a fix for a deprecated spelling replaces the node's span
/// with (design 05 §Fixes).
pub fn print_node(node: NodeRef<'_>) -> String {
    print_node_with(node, Profile::Canonical)
}

/// `print_node` under a profile (design 04 §Profiles). A node printed on
/// its own has no document: under `Mkdocs` a bare `@key` without a colon
/// is a citation, and `#{prefix:key}` needs a predeclared prefix.
pub fn print_node_with(node: NodeRef<'_>, profile: Profile) -> String {
    let mut out = Out::with_profile(profile);
    match node {
        NodeRef::Block(b) => block::block(&mut out, b),
        NodeRef::Inline(i) => {
            inline::inlines(&mut out, std::slice::from_ref(i), Context::default())
        }
    }
    out.finish().trim_end_matches('\n').to_string()
}

/// Apply `edit` to `text`, the source `doc` was parsed from. Returns `text`
/// unchanged when the node is not found or lies outside the text.
pub fn edit(text: &str, doc: &Document, edit: NodeEdit) -> String {
    edit_many(text, doc, vec![edit]).unwrap_or_else(|_| text.to_string())
}

/// Apply every edit to `text`, the source `doc` was parsed from, in one
/// pass: spans are spliced from the end of the text, so earlier offsets
/// stay valid. The spans must be disjoint; an unknown node, a span outside
/// the text or two overlapping spans are an error and nothing is applied.
pub fn edit_many(text: &str, doc: &Document, edits: Vec<NodeEdit>) -> Result<String, EditError> {
    let mut located: Vec<(usize, usize, NodeEdit)> = Vec::with_capacity(edits.len());
    for edit in edits {
        let span = span_of(doc, edit.id).ok_or(EditError::NotFound(edit.id))?;
        let (start, end) = (span.start as usize, span.end as usize);
        if span.file != doc.file
            || start > end
            || end > text.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            return Err(EditError::OutOfRange(edit.id));
        }
        located.push((start, end, edit));
    }
    located.sort_by_key(|(start, end, _)| (*start, *end));
    for pair in located.windows(2) {
        let (_, end, first) = &pair[0];
        let (start, _, second) = &pair[1];
        if start < end || (start == end && first.id == second.id) {
            return Err(EditError::Overlap(first.id, second.id));
        }
    }
    let mut result = text.to_string();
    for (start, end, edit) in located.into_iter().rev() {
        result.replace_range(start..end, &print(&edit.replacement));
    }
    Ok(result)
}
