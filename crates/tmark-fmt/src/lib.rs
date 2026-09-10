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
mod out;

use tmark_ir::Document;

pub use edit::{edit, print_node, NodeEdit, Replacement};

/// Which spellings the printer emits. Design `04-printer.md` §Profiles.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    /// The normal form.
    #[default]
    Canonical,
    /// The normal form; the strict profile differs at parse time only
    /// (X1 and X3 off), so it prints like `Canonical`.
    Strict,
    /// PyMdownX spellings for MkDocs sites. Milestone 5: prints like
    /// `Canonical` until then.
    Mkdocs,
}

/// Print a document in its normal form for `profile`.
pub fn format(doc: &Document, profile: Profile) -> String {
    let _ = profile;
    let mut out = out::Out::new();
    block::document(&mut out, doc);
    out.finish()
}
