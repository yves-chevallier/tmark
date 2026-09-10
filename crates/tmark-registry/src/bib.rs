//! The bibliography registry: `.bib` files, inline front-matter entries and
//! DOI shorthands (spec §Bibliography). Keys and raw fields only: no
//! formatting here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use tmark_ir::{Code, Diagnostic, Document};

use crate::loader::Loader;

#[derive(Clone, Debug, PartialEq)]
pub struct BibEntry {
    pub key: String,
    /// `article`, `misc`, …; `doi` for a front-matter DOI shorthand whose
    /// record TeXSmith fetches.
    pub entry_type: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
pub struct Bib {
    pub entries: BTreeMap<String, BibEntry>,
}

impl Bib {
    pub fn get(&self, key: &str) -> Option<&BibEntry> {
        self.entries.get(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
}

pub fn load(
    doc: &Document,
    loader: &dyn Loader,
    base: &Path,
    files: &[PathBuf],
    diagnostics: &mut Vec<Diagnostic>,
) -> Bib {
    let mut bib = Bib::default();
    for file in files {
        let rel = file.to_string_lossy();
        match loader.load(base, &rel) {
            Some(text) => parse_bibtex(&text, &mut bib),
            None => diagnostics.push(Diagnostic::new(
                Code::IncludeMissing,
                doc.front_matter.meta.span,
                format!("bibliography file `{rel}` not found"),
            )),
        }
    }
    inline_entries(doc, &mut bib);
    bib
}

fn parse_bibtex(text: &str, bib: &mut Bib) {
    let Ok(parsed) = biblatex::Bibliography::parse(text) else {
        return;
    };
    for entry in parsed.iter() {
        let mut fields = BTreeMap::new();
        for (name, chunks) in &entry.fields {
            fields.insert(
                name.clone(),
                biblatex::ChunksExt::format_verbatim(chunks.as_slice()),
            );
        }
        bib.entries.insert(
            entry.key.clone(),
            BibEntry {
                key: entry.key.clone(),
                entry_type: entry.entry_type.to_string(),
                fields,
            },
        );
    }
}

/// `sources.bibliography` in the front matter: `key: https://doi.org/…` or
/// `key: {type: misc, title: …, …}`.
fn inline_entries(doc: &Document, bib: &mut Bib) {
    let serde_json::Value::Object(map) = &doc.front_matter.keys.press.sources.bibliography else {
        return;
    };
    for (key, value) in map {
        let entry = match value {
            serde_json::Value::String(url) => BibEntry {
                key: key.clone(),
                entry_type: "doi".to_string(),
                fields: BTreeMap::from([("doi".to_string(), url.clone())]),
            },
            serde_json::Value::Object(fields) => BibEntry {
                key: key.clone(),
                entry_type: fields
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("misc")
                    .to_string(),
                fields: fields
                    .iter()
                    .filter(|(k, _)| k.as_str() != "type")
                    .map(|(k, v)| {
                        let text = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        (k.clone(), text)
                    })
                    .collect(),
            },
            _ => continue,
        };
        bib.entries.insert(key.clone(), entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_keys_and_fields() {
        let mut bib = Bib::default();
        parse_bibtex(
            "@article{ein05,\n  title = {Zur Elektrodynamik},\n  year = 1905\n}\n",
            &mut bib,
        );
        let entry = bib.get("ein05").expect("entry");
        assert_eq!(entry.entry_type, "article");
        assert_eq!(
            entry.fields.get("title").map(String::as_str),
            Some("Zur Elektrodynamik")
        );
    }
}
