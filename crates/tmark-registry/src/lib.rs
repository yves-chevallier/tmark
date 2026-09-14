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
mod view;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tmark_ir::{Diagnostic, Document, FileId};

pub use bib::{Bib, BibEntry};
pub use collect::{figure_images, subfigure_letter, Host, IndexTable, Label, Labels, Subfigure};
pub use counters::{Counter, Counters};
pub use inventory::{CrossRefs, Inventory, InventoryEntry};
#[cfg(feature = "fs")]
pub use loader::FsLoader;
pub use loader::{join, Loader, MemoryLoader};
pub use refs::{RefResolution, Resolution};
pub use view::{IndexEntryView, LabelView, ResolvedView};

/// Which series tmark allocates numbers for (design 06 §Site-wide
/// resolution).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Numbering {
    /// The TeXSmith-numbered series only (user counters, theorem kinds
    /// with a counter of their own); `sec`, `fig`, `tbl`, `lst`, `eq`,
    /// `thm` are the backend's (spec §Counters, "Numbered by").
    #[default]
    Backend,
    /// Every series that numbers something, for a medium with no backend
    /// to number them (the web). Continuous `document` scope: chapter and
    /// section scopes do not reset (web-profile open question 1).
    All,
}

/// A label of a sibling document of the same book or site, as
/// [`Resolved::book_labels`] publishes it and [`ResolveOptions::book`]
/// takes it back (design 06 §Site-wide resolution).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BookLabel {
    /// The label id as written (`fw:boot`, `sec:intro`); matched
    /// case-insensitively.
    pub key: String,
    /// The series it counts in, when it counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// The formatted number (`FW-10`, `3`), when the series numbered it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    pub kind: Host,
    /// The heading or caption text, for a header or caption host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Where the label lives: the document's location plus `#key`
    /// (`findings.md#fw:boot`). Opaque to tmark; the caller relativises.
    pub location: String,
}

/// Inputs of a resolution that are not in the document.
#[derive(Clone, Debug, Default)]
pub struct ResolveOptions {
    /// Path of the document, relative to which includes and sources load.
    pub path: PathBuf,
    /// `.bib` files (spec §Bibliography: "passed on the command line").
    pub bibliography: Vec<PathBuf>,
    /// First value of each series tmark numbers, for a series that
    /// continues across the documents of a build ([`Resolved::next_start`]).
    pub start: BTreeMap<String, u32>,
    /// Which series tmark numbers.
    pub numbering: Numbering,
    /// Language of the label words (`Figure`, `Abbildung`); a BCP 47 tag.
    /// Defaults to the front matter's `lang`, then English.
    pub lang: Option<String>,
    /// Labels of the other documents of the book or site: a key defined
    /// in none of this document's registries resolves to
    /// [`Resolution::Sibling`] when one of them has it.
    pub book: Vec<BookLabel>,
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
    /// The included documents themselves (their labels are in `labels`,
    /// their references in `refs`; an editor reads their nodes for hover).
    pub included: Vec<(FileId, Document)>,
    pub diagnostics: Vec<Diagnostic>,
    /// The path the document was resolved as ([`ResolveOptions::path`]).
    pub path: PathBuf,
    /// Which series were numbered ([`ResolveOptions::numbering`]).
    pub numbering: Numbering,
    /// The language the label words are in (the option, else the front
    /// matter's `lang`); `None` is English.
    pub lang: Option<String>,
    /// The sibling labels by lower-cased key ([`ResolveOptions::book`]).
    siblings: BTreeMap<String, BookLabel>,
}

impl Resolved {
    /// The first free value of every series tmark numbered: what the
    /// next document of a build passes as [`ResolveOptions::start`] so
    /// that numbering continues across documents.
    pub fn next_start(&self) -> BTreeMap<String, u32> {
        self.counters
            .by_prefix
            .values()
            .filter(|c| c.tmark_numbered)
            .map(|c| (c.prefix.clone(), c.next()))
            .collect()
    }

    /// This document's labels as the other documents of the book see
    /// them: what they pass as [`ResolveOptions::book`]. `location` is the
    /// document's location for the caller's purposes (a page path, a URL);
    /// each label's location is `location#key`.
    pub fn book_labels(&self, location: &str) -> Vec<BookLabel> {
        self.labels
            .in_order
            .iter()
            .map(|label| BookLabel {
                key: label.id.clone(),
                prefix: label.prefix.clone(),
                number: self.formatted(label),
                kind: label.host,
                title: label.title.clone(),
                location: format!("{location}#{}", label.id),
            })
            .collect()
    }

    /// Whether a numeric reference to `label` has no number to show (spec
    /// §Anchor, `ref-unnumbered`): its host is an anchor only (a span, a
    /// `Div`, a block quote) and no declared series numbered it, or it is
    /// a sub-figure of a container that has no label of its own. The
    /// backend-numbered hosts (headings, floats, equations, theorems) are
    /// numbered even though `formatted` is `None` for them.
    pub fn unnumbered(&self, label: &Label) -> bool {
        match label.host {
            Host::Anchor => self.formatted(label).is_none(),
            Host::Subfigure => label
                .subfigure
                .as_ref()
                .and_then(|s| s.parent.as_deref())
                .and_then(|parent| self.labels.get(parent))
                .map_or(true, |parent| self.unnumbered(parent)),
            _ => false,
        }
    }

    /// The formatted number of a label, when its series numbered it. A
    /// subfigure's is its container's number and its letter (`2a`); the
    /// container's own number is never a subfigure's, so one level of
    /// indirection is enough.
    pub(crate) fn formatted(&self, label: &Label) -> Option<String> {
        let Some(subfigure) = &label.subfigure else {
            return self.own_number(label);
        };
        let parent = self.labels.get(subfigure.parent.as_deref()?)?;
        let number = self.own_number(parent)?;
        Some(match &subfigure.letter {
            Some(letter) => format!("{number}{letter}"),
            None => number,
        })
    }

    /// The number the label's own series allocated it.
    fn own_number(&self, label: &Label) -> Option<String> {
        label
            .prefix
            .as_deref()
            .and_then(|p| self.counters.get(p))
            .and_then(|c| c.label(&label.key))
    }

    /// The sibling label of a key, when the book has one.
    pub fn sibling(&self, key: &str) -> Option<&BookLabel> {
        self.siblings.get(&key.to_ascii_lowercase())
    }

    /// The JSON-shaped view of the resolution (design 09 §Python).
    pub fn view(&self) -> ResolvedView {
        ResolvedView::new(self)
    }
}

/// Build the registries of `doc` and resolve its references.
pub fn resolve(doc: &Document, loader: &dyn Loader, options: &ResolveOptions) -> Resolved {
    let mut resolved = Resolved {
        path: options.path.clone(),
        numbering: options.numbering,
        lang: options
            .lang
            .clone()
            .or_else(|| doc.front_matter.keys.lang.clone()),
        ..Default::default()
    };
    let base = options.path.parent().unwrap_or(Path::new("")).to_path_buf();
    // The first label of a key wins, as for local labels.
    for label in &options.book {
        resolved
            .siblings
            .entry(label.key.to_ascii_lowercase())
            .or_insert_with(|| label.clone());
    }

    // 1. Declare.
    resolved.counters = Counters::declare(
        doc,
        &options.start,
        options.numbering,
        resolved.lang.as_deref(),
        &mut resolved.diagnostics,
    );

    // 2. Collect definitions, across includes.
    let mut collector = collect::Collector::new(loader, &resolved.counters);
    collector.collect(doc, &options.path);
    resolved.labels = collector.labels;
    resolved.index = collector.index;
    resolved.files = collector.files;
    resolved.included = collector.documents;
    resolved.diagnostics.extend(collector.diagnostics);

    // 3. Allocate numbers of the series tmark numbers.
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

    // 5. Resolve every reference, in the document and in its includes.
    refs::resolve_all(doc, &mut resolved);
    let included = std::mem::take(&mut resolved.included);
    for (_, included_doc) in &included {
        refs::resolve_all(included_doc, &mut resolved);
    }
    resolved.included = included;
    resolved
}
