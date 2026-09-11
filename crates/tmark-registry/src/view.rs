//! The serialisable view of a [`Resolved`]: what crosses the bindings and
//! what `schema("resolved")` describes (design 09 §Python, ADR 0003). The
//! registries themselves keep their lookup shapes; this is a flat copy.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;
use tmark_ir::{Diagnostic, FileId, NodeId};

use crate::{
    BibEntry, BookLabel, Counter, Inventory, Label, Numbering, RefResolution, Resolution, Resolved,
};

/// A label with its formatted number, when its series numbers it.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct LabelView {
    #[serde(flatten)]
    pub label: Label,
    /// The number as the counter's `format` prints it (`FW-07`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatted: Option<String>,
}

/// One `{index}[…]` entry.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct IndexEntryView {
    /// The registry name; `None` is the default registry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    pub node: NodeId,
    /// The entry path, one plain-text string per level.
    pub path: Vec<String>,
}

/// A file parsed through an include.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct FileView {
    pub id: FileId,
    /// The path the loader was asked for, joined to the including file.
    pub path: String,
}

/// The registries and the resolution of a document, flat and serialisable.
#[derive(Clone, Debug, Default, PartialEq, Serialize, JsonSchema)]
pub struct ResolvedView {
    /// Which series tmark numbered (`ResolveOptions::numbering`).
    pub numbering: Numbering,
    /// The language of the label words; `None` is English.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Every series by prefix, predeclared ones included.
    pub counters: BTreeMap<String, Counter>,
    /// The first free value of every tmark-numbered series
    /// ([`Resolved::next_start`]).
    pub next_start: BTreeMap<String, u32>,
    /// Every definition, in document order (includes after their host).
    pub labels: Vec<LabelView>,
    /// This document's labels for its siblings ([`Resolved::book_labels`]
    /// at the document's path): what the next resolution takes as `book`.
    pub book: Vec<BookLabel>,
    /// One entry per `Ref` node and anchor link, in document order.
    pub refs: Vec<RefResolution>,
    /// The bibliography keys, sorted.
    pub bibliography: Vec<String>,
    /// The bibliography entries by key (raw fields, no formatting).
    pub entries: BTreeMap<String, BibEntry>,
    /// DOIs whose record is not in the bibliography: front-matter DOI
    /// shorthands first, then `@doi:` citations; what TeXSmith fetches.
    pub dois: Vec<String>,
    /// Glossary and acronym terms, lower-cased.
    pub glossary: BTreeMap<String, String>,
    pub index: Vec<IndexEntryView>,
    /// Cross-document inventories by alias.
    pub crossrefs: BTreeMap<String, Inventory>,
    /// Files parsed through includes, in load order.
    pub included: Vec<FileView>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ResolvedView {
    pub fn new(resolved: &Resolved) -> Self {
        let labels = resolved
            .labels
            .in_order
            .iter()
            .map(|label| LabelView {
                formatted: resolved.formatted(label),
                label: label.clone(),
            })
            .collect();
        let mut dois: Vec<String> = resolved
            .bibliography
            .entries
            .values()
            .filter(|e| e.entry_type == "doi")
            .filter_map(|e| e.fields.get("doi").cloned())
            .collect();
        for r in &resolved.refs {
            if let Resolution::Doi { doi } = &r.resolution {
                if !dois.contains(doi) {
                    dois.push(doi.clone());
                }
            }
        }
        let index = resolved
            .index
            .entries
            .iter()
            .flat_map(|(registry, entries)| {
                entries.iter().map(move |(node, path)| IndexEntryView {
                    registry: registry.clone(),
                    node: *node,
                    path: path.clone(),
                })
            })
            .collect();
        ResolvedView {
            numbering: resolved.numbering,
            lang: resolved.lang.clone(),
            counters: resolved.counters.by_prefix.clone(),
            next_start: resolved.next_start(),
            labels,
            book: resolved.book_labels(&resolved.path.to_string_lossy()),
            refs: resolved.refs.clone(),
            bibliography: resolved.bibliography.entries.keys().cloned().collect(),
            entries: resolved.bibliography.entries.clone(),
            dois,
            glossary: resolved.glossary.clone(),
            index,
            crossrefs: resolved.crossrefs.by_alias.clone(),
            included: resolved
                .files
                .iter()
                .map(|(id, path)| FileView {
                    id: *id,
                    path: path.to_string_lossy().into_owned(),
                })
                .collect(),
            diagnostics: resolved.diagnostics.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resolve, MemoryLoader, ResolveOptions};
    use std::path::PathBuf;

    const DOC: &str = "---\npress:\n  declare:\n    counters:\n      fw: {name: Finding, format: \"FW-{n:02d}\"}\n  sources:\n    bibliography:\n      ein05: https://doi.org/10.1002/andp.19053221004\n---\n\n#(fw:boot) Boot. See @fw:boot, @ein05 and @doi:10.1000/x.\n";

    #[test]
    fn view_is_flat_and_numbered() {
        let parsed = tmark_syntax::parse(DOC, FileId(0));
        let resolved = resolve(
            &parsed.document,
            &MemoryLoader::new(),
            &ResolveOptions {
                path: PathBuf::from("doc.md"),
                ..Default::default()
            },
        );
        let view = resolved.view();
        assert_eq!(view.next_start.get("fw"), Some(&2));
        assert_eq!(view.labels.len(), 1);
        assert_eq!(view.labels[0].formatted.as_deref(), Some("FW-01"));
        assert_eq!(view.bibliography, ["ein05"]);
        assert_eq!(
            view.dois,
            ["https://doi.org/10.1002/andp.19053221004", "10.1000/x"]
        );
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["refs"][0]["resolution"]["kind"], "label");
        assert_eq!(json["refs"][0]["resolution"]["number"], "FW-01");
        assert_eq!(json["refs"][1]["resolution"]["kind"], "citation");
        assert_eq!(json["labels"][0]["host"], "counter_item");
        assert_eq!(json["counters"]["fw"]["next"], 2);
        assert!(serde_json::to_value(schemars::schema_for!(ResolvedView)).is_ok());
    }
}
