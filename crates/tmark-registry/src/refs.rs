//! Step 5: what every reference refers to (spec §Registries (lookup),
//! §Ref, §Cite, §Glossary reference, §Cross-document references).

use schemars::JsonSchema;
use serde::Serialize;
use tmark_ir::{walk, Code, Diagnostic, Inline, NodeId, NodeRef, Span, Target};

use crate::Resolved;

/// What a key refers to. Serialises with a `kind` tag (`label`, `citation`,
/// `glossary`, `doi`, `external`, `ambiguous`, `unresolved`).
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resolution {
    /// A label or counter item of this document (or an include).
    Label {
        target: NodeId,
        prefix: Option<String>,
        /// Formatted number, for TeXSmith-numbered series.
        number: Option<String>,
    },
    Citation {
        key: String,
    },
    Glossary {
        term: String,
    },
    Doi {
        doi: String,
    },
    External {
        alias: String,
        label: String,
        page: Option<u32>,
    },
    /// A key present in two registries (`ref-ambiguous`).
    Ambiguous,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct RefResolution {
    pub node: NodeId,
    /// The key token (the whole node for an anchor link, or when the item
    /// came from JSON without a key span).
    pub span: Span,
    /// The key as written.
    pub key: String,
    pub resolution: Resolution,
}

pub fn resolve_all(doc: &tmark_ir::Document, resolved: &mut Resolved) {
    let mut found: Vec<(NodeId, Span, String)> = Vec::new();
    walk(doc, &mut |node: NodeRef| {
        if let NodeRef::Inline(inline) = node {
            match inline {
                Inline::Ref(r) => {
                    for item in &r.items {
                        let span = if item.key_span.0.is_empty() {
                            r.meta.span
                        } else {
                            item.key_span.0
                        };
                        found.push((r.meta.id, span, item.key.clone()));
                    }
                }
                Inline::Link(l) => {
                    if let Target::Anchor(id) = &l.target {
                        found.push((l.meta.id, l.meta.span, id.clone()));
                    }
                }
                _ => {}
            }
        }
    });
    for (node, span, key) in found {
        let resolution = resolve_one(&key, resolved);
        if resolution == Resolution::Unresolved {
            resolved.diagnostics.push(Diagnostic::new(
                Code::RefUnresolved,
                span,
                format!("`@{key}` does not resolve to a label, a citation key, a glossary term or an inventory"),
            ));
        }
        resolved.refs.push(RefResolution {
            node,
            span,
            key,
            resolution,
        });
    }
}

/// Resolution order (spec §Registries): declared prefix (`doi` and `gls`
/// included), then bibliography; a key in two registries is ambiguous.
fn resolve_one(key: &str, resolved: &mut Resolved) -> Resolution {
    let lower = key.to_ascii_lowercase();
    // Cross-document: `alias:prefix:key` with a declared alias.
    if let Some((alias, rest)) = lower.split_once(':') {
        if let Some(inventory) = resolved.crossrefs.by_alias.get(alias) {
            return match inventory.refs.get(rest) {
                Some(entry) => Resolution::External {
                    alias: alias.to_string(),
                    label: entry.label.clone(),
                    page: entry.page,
                },
                None => Resolution::Unresolved,
            };
        }
    }
    if let Some(doi) = lower.strip_prefix("doi:") {
        return Resolution::Doi {
            doi: key[key.len() - doi.len()..].to_string(),
        };
    }
    if let Some(term) = lower.strip_prefix("gls:") {
        return if resolved.glossary.contains_key(term) {
            Resolution::Glossary {
                term: term.to_string(),
            }
        } else {
            Resolution::Unresolved
        };
    }
    let head = lower.split_once(':').map(|(h, _)| h);
    let label = match head {
        Some(prefix) if resolved.counters.is_declared(prefix) => resolved.labels.get(&lower),
        Some(_) => None,
        None => resolved.labels.get(&lower),
    };
    let citation = resolved.bibliography.contains(key);
    match (label, citation) {
        (Some(label), true) => {
            let span = label.span;
            resolved.diagnostics.push(Diagnostic::new(
                Code::RefAmbiguous,
                span,
                format!("`{key}` is both a label and a bibliography key"),
            ));
            Resolution::Ambiguous
        }
        (Some(label), false) => {
            let number = label
                .prefix
                .as_deref()
                .and_then(|p| resolved.counters.get(p))
                .and_then(|c| c.label(&label.key));
            Resolution::Label {
                target: label.node,
                prefix: label.prefix.clone(),
                number,
            }
        }
        (None, true) => Resolution::Citation {
            key: key.to_string(),
        },
        (None, false) => Resolution::Unresolved,
    }
}
