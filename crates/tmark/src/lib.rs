//! Facade: the one crate downstream users depend on.
//!
//! Design: `design/01-architecture.md` §The facade. Milestone 1 exposes
//! `parse`, `format` and `edit`; `resolve`, `lint` and `write` arrive with
//! their crates.

#![forbid(unsafe_code)]

pub use tmark_fmt::{edit, format, NodeEdit, Profile, Replacement};
pub use tmark_ir as ir;
pub use tmark_ir::schema;
pub use tmark_ir::{Diagnostic, Document, FileId};
pub use tmark_syntax::{parse, parse_strict, Parsed};
