//! Text to `Document` plus diagnostics: drives the vendored micromark
//! tokenizer (`tmark-markdown`) and lowers its tree to the TMark IR.
//!
//! Design: `design/02-syntax.md`. The lowering never fails: unrecognised
//! input is literal text plus a diagnostic.

#![forbid(unsafe_code)]

mod lower;
mod offset;

use tmark_ir::{Diagnostic, Document, FileId};

/// The result of parsing one file.
#[derive(Debug)]
pub struct Parsed {
    /// The document, always produced.
    pub document: Document,
    /// Syntactic diagnostics, in document order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse a TMark file.
///
/// `file` identifies the text in every span of the result. Parsing never
/// fails: text that is not recognised stays literal (spec §Roles, P4).
pub fn parse(text: &str, file: FileId) -> Parsed {
    lower::parse(text, file)
}

/// Parse under the strict profile: X1 (`__x__` small caps) and the
/// Appendix-PyMdownX sugar are off (spec §Conformance).
pub fn parse_strict(text: &str, file: FileId) -> Parsed {
    lower::parse_with(text, file, lower::Options { strict: true })
}
