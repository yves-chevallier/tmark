//! Facade: the one crate downstream users depend on.
//!
//! Design: `design/01-architecture.md` §The facade. Milestone 1 exposes
//! parsing; `resolve`, `lint`, `format` and `write` arrive with their crates.

#![forbid(unsafe_code)]

pub use tmark_ir as ir;
pub use tmark_ir::{Diagnostic, Document, FileId};
pub use tmark_syntax::{parse, parse_strict, Parsed};
