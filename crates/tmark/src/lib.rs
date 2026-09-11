//! Facade: the one crate downstream users depend on.
//!
//! Design: `design/01-architecture.md` §The facade. `write` arrives with
//! the writers (milestone 4).

#![forbid(unsafe_code)]

mod config;

pub use config::Config;
pub use tmark_fmt::{
    edit, edit_many, format, print_node, EditError, NodeEdit, Profile, Replacement,
};
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

/// Parse for a profile: the strict profile switches the X-class
/// constructs off at parse time (spec §Conformance and deviations); the
/// others parse alike. The one place that maps a profile to a parser.
pub fn parse_with(text: &str, file: FileId, profile: Profile) -> Parsed {
    match profile {
        Profile::Strict => parse_strict(text, file),
        Profile::Canonical | Profile::Mkdocs => parse(text, file),
    }
}

/// The stages after parsing: resolve, then lint. Returns the resolution
/// (navigation and completion read it) and the diagnostics of both stages
/// in order. Includes and sources load through `loader`, relative to
/// `options.path`.
pub fn analyse(
    doc: &Document,
    text: &str,
    loader: &dyn Loader,
    options: &ResolveOptions,
    lint_config: &LintConfig,
) -> (Resolved, Vec<Diagnostic>) {
    let resolved = resolve(doc, loader, options);
    let mut diagnostics = resolved.diagnostics.clone();
    diagnostics.extend(lint(doc, &resolved, text, lint_config));
    (resolved, diagnostics)
}

/// Attach a `Fix` to every `deprecated` diagnostic whose span is exactly a
/// node of `doc`: the node reprinted in its canonical spelling (design 05
/// §Fixes: "rewriting a deprecated spelling is safe"). Diagnostics that
/// already carry a fix, or whose span is not a node (front-matter keys),
/// are left alone.
pub fn fixes(doc: &Document, diagnostics: &mut [Diagnostic]) {
    for d in diagnostics
        .iter_mut()
        .filter(|d| d.code == ir::Code::Deprecated && d.fix.is_none())
    {
        let mut node = None;
        ir::walk(doc, &mut |n: ir::NodeRef| {
            if node.is_none() && n.span() == d.span {
                node = Some(n);
            }
        });
        if let Some(node) = node {
            d.fix = Some(ir::Fix {
                span: d.span,
                replacement: print_node(node),
            });
        }
    }
}

/// Every diagnostic of a file: parse, resolve and lint, in that order,
/// with fixes attached.
pub fn check(
    text: &str,
    file: FileId,
    profile: Profile,
    loader: &dyn Loader,
    options: &ResolveOptions,
    lint_config: &LintConfig,
) -> (Document, Vec<Diagnostic>) {
    let parsed = parse_with(text, file, profile);
    let (_, analysis) = analyse(&parsed.document, text, loader, options, lint_config);
    let mut diagnostics = parsed.diagnostics;
    diagnostics.extend(analysis);
    fixes(&parsed.document, &mut diagnostics);
    (parsed.document, diagnostics)
}
