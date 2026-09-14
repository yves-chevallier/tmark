//! Canonical printer, profiles, local edits.
//!
//! Design: `design/04-printer.md`. The printer emits exactly one spelling per
//! node (the "Canonical" column of the spec's node catalogue), so that
//! `parse(format(doc)) == doc` and `format` is idempotent. Prose is never
//! re-wrapped; the front matter is copied byte for byte.

#![forbid(unsafe_code)]

mod attrs;
mod block;
mod edit;
mod escape;
mod inline;
mod mkdocs;
mod out;

use tmark_ir::Document;

pub use edit::{edit, edit_many, print_node, print_node_with, EditError, NodeEdit, Replacement};
pub use inline::blocks_sigil;

/// Which spellings the printer emits. Design `04-printer.md` §Profiles.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    /// The normal form.
    #[default]
    Canonical,
    /// The normal form; the strict profile differs at parse time only
    /// (X1 and X3 off), so it prints like `Canonical`.
    Strict,
    /// The PyMdownX and TeXSmith 0.6 spellings a MkDocs site renders
    /// (`mkdocs.rs`); every other construct prints as `Canonical`.
    Mkdocs,
}

/// Print a document in its normal form for `profile`.
pub fn format(doc: &Document, profile: Profile) -> String {
    let mut out = out::Out::for_document(profile, doc);
    block::document(&mut out, doc);
    out.finish()
}
