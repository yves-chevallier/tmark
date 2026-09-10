//! Registries and resolution: counters, labels, bibliography, glossary,
//! index, cross-document inventories; the `Loader` seam; what every `@`
//! refers to. Design: `design/06-registries.md`.
//!
//! Everything here is pure: the only way to another file is `Loader`.

#![forbid(unsafe_code)]

mod bib;
mod collect;
mod counters;
mod inventory;
mod loader;
mod refs;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tmark_ir::{Diagnostic, Document, FileId};

pub use bib::{Bib, BibEntry};
pub use collect::{Host, IndexTable, Label, Labels};
pub use counters::{Counter, Counters};
pub use inventory::{CrossRefs, Inventory, InventoryEntry};
#[cfg(feature = "fs")]
pub use loader::FsLoader;
pub use loader::{Loader, MemoryLoader};
pub use refs::{RefResolution, Resolution};

/// Inputs of a resolution that are not in the document.
#[derive(Clone, Debug, Default)]
pub struct ResolveOptions {
    /// Path of the document, relative to which includes and sources load.
    pub path: PathBuf,
    /// `.bib` files (spec §Bibliography: "passed on the command line").
    pub bibliography: Vec<PathBuf>,
    /// First value of each TeXSmith-numbered series, for a series that
    /// continues across the documents of a build.
    pub start: BTreeMap<String, u32>,
}

/// The registries of a document and the resolution of its references.
#[derive(Debug, Default)]
pub struct Resolved {
    pub counters: Counters,
    pub labels: Labels,
    pub bibliography: Bib,
    /// Glossary and acronym terms (`@gls:term`).
    pub glossary: BTreeMap<String, String>,
    pub index: IndexTable,
    pub crossrefs: CrossRefs,
    /// One entry per `Ref` node and per anchor link, in document order.
    pub refs: Vec<RefResolution>,
    /// Files parsed through includes: their id and path.
    pub files: Vec<(FileId, PathBuf)>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Build the registries of `doc` and resolve its references.
pub fn resolve(doc: &Document, loader: &dyn Loader, options: &ResolveOptions) -> Resolved {
    let mut resolved = Resolved::default();
    let base = options.path.parent().unwrap_or(Path::new("")).to_path_buf();

    // 1. Declare.
    resolved.counters = Counters::declare(doc, &options.start, &mut resolved.diagnostics);

    // 2. Collect definitions, across includes.
    let mut collector = collect::Collector::new(loader, &resolved.counters);
    collector.collect(doc, &options.path);
    resolved.labels = collector.labels;
    resolved.index = collector.index;
    resolved.files = collector.files;
    resolved.diagnostics.extend(collector.diagnostics);

    // 3. Allocate numbers of the TeXSmith-numbered series.
    resolved.counters.allocate(&mut resolved.labels);

    // 4. Sources.
    resolved.bibliography = bib::load(
        doc,
        loader,
        &base,
        &options.bibliography,
        &mut resolved.diagnostics,
    );
    resolved.glossary = collect::glossary(doc);
    resolved.crossrefs = inventory::load(doc, loader, &base, &mut resolved.diagnostics);

    // 5. Resolve every reference.
    refs::resolve_all(doc, &mut resolved);
    resolved
}
