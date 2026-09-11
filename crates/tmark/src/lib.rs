//! Facade: the one crate downstream users depend on.
//!
//! Design: `design/01-architecture.md` §The facade. `write` arrives with
//! the writers (milestone 4).

#![forbid(unsafe_code)]

mod config;
mod json;

pub use config::Config;
pub use json::{from_json, schema_hash, to_json, VERSION};
pub use tmark_fmt::{
    edit, edit_many, format, print_node, print_node_with, EditError, NodeEdit, Profile, Replacement,
};
pub use tmark_ir as ir;
pub use tmark_ir::{Diagnostic, Document, FileId};
pub use tmark_lint::{lint, Config as LintConfig};
#[cfg(feature = "fs")]
pub use tmark_registry::FsLoader;
pub use tmark_registry::{
    resolve, Host, Label, Loader, MemoryLoader, RefResolution, Resolution, ResolveOptions,
    Resolved, ResolvedView,
};
pub use tmark_syntax::{parse, parse_strict, Parsed};

/// The JSON schema of a public shape, by name: the ones of `tmark_ir::schema`
/// (`"ir"`, `"frontmatter"`, `"diagnostic"`) plus `"resolved"` (the
/// [`ResolvedView`] `resolve` produces).
pub fn schema(name: &str) -> Option<serde_json::Value> {
    match name {
        "resolved" => serde_json::to_value(schemars::schema_for!(ResolvedView)).ok(),
        other => tmark_ir::schema(other),
    }
}

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

/// Splice every fix of the main file into `text`, last first so that
/// earlier spans stay valid; a fix overlapping one already applied is
/// skipped. Returns the new text and the number of fixes applied: what
/// `tmark lint --fix` writes.
pub fn apply_fixes(text: &str, file: FileId, diagnostics: &[Diagnostic]) -> (String, usize) {
    let mut fixes: Vec<&ir::Fix> = diagnostics
        .iter()
        .filter_map(|d| d.fix.as_ref())
        .filter(|f| f.span.file == file)
        .collect();
    fixes.sort_by_key(|f| std::cmp::Reverse(f.span.start));
    let mut out = text.to_string();
    let mut applied = 0;
    let mut limit = text.len() as u32;
    for fix in fixes {
        if fix.span.end > limit {
            continue;
        }
        out.replace_range(
            fix.span.start as usize..fix.span.end as usize,
            &fix.replacement,
        );
        limit = fix.span.start;
        applied += 1;
    }
    (out, applied)
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
