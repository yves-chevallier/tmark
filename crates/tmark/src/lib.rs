//! Facade: the one crate downstream users depend on.
//!
//! Design: `design/01-architecture.md` §The facade. `write` arrives with
//! the writers (milestone 4).

#![forbid(unsafe_code)]

mod config;

pub use config::Config;
pub use tmark_fmt::{edit, format, NodeEdit, Profile, Replacement};
pub use tmark_ir as ir;
pub use tmark_ir::schema;
pub use tmark_ir::{Diagnostic, Document, FileId};
pub use tmark_lint::{lint, Config as LintConfig};
#[cfg(feature = "fs")]
pub use tmark_registry::FsLoader;
pub use tmark_registry::{
    resolve, Host, Label, Loader, MemoryLoader, RefResolution, Resolution, ResolveOptions, Resolved,
};
pub use tmark_syntax::{parse, parse_strict, Parsed};

/// Every diagnostic of a file: parse, resolve and lint, in that order.
/// Includes and sources load through `loader`, relative to
/// `options.path`.
pub fn check(
    text: &str,
    file: FileId,
    loader: &dyn Loader,
    options: &ResolveOptions,
    lint_config: &LintConfig,
) -> (Document, Vec<Diagnostic>) {
    let parsed = parse(text, file);
    let resolved = resolve(&parsed.document, loader, options);
    let mut diagnostics = parsed.diagnostics;
    diagnostics.extend(resolved.diagnostics.iter().cloned());
    diagnostics.extend(lint(&parsed.document, &resolved, text, lint_config));
    (parsed.document, diagnostics)
}
