//! The counter registry: predeclared prefixes plus `declare.counters`
//! (spec §Counters), and the numbers of the TeXSmith-numbered series.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;
use tmark_ir::registry::{self, Scope};
use tmark_ir::{Code, Diagnostic, Document};

use crate::collect::Labels;

/// One series. Spec §Counters, "Fields".
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct Counter {
    pub prefix: String,
    /// Label word (`Figure`, `Finding`); none for `gls` and `doi`.
    pub name: Option<String>,
    /// Python-style format over `n`, `prefix`, `key`; `None` means `"{n}"`.
    pub format: Option<String>,
    pub start: u32,
    pub scope: Option<Scope>,
    /// Template of a reference (`"{name} {number}"`, `"{number}"`).
    pub reference: String,
    /// Numbered by TeXSmith (user series) rather than by the backend.
    pub tmark_numbered: bool,
    /// Numbers allocated in document order, by key.
    pub numbers: BTreeMap<String, u32>,
    /// The next value the series allocates: what the following document of
    /// a build passes as `ResolveOptions::start`.
    next: u32,
}

impl Counter {
    /// The value the next label of a TeXSmith-numbered series takes.
    pub fn next(&self) -> u32 {
        self.next
    }

    /// The formatted number of a key, when this series numbers it.
    pub fn label(&self, key: &str) -> Option<String> {
        let n = *self.numbers.get(key)?;
        Some(format_number(
            self.format.as_deref().unwrap_or("{n}"),
            n,
            &self.prefix,
            key,
        ))
    }
}

/// `"FW-{n:02d}"` with `n`, `prefix` and `key`: the subset of Python's
/// format language the spec uses (zero-padded widths on `n`).
pub fn format_number(format: &str, n: u32, prefix: &str, key: &str) -> String {
    let mut out = String::new();
    let mut rest = format;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let field = &rest[open + 1..open + close];
        let (name, spec) = field.split_once(':').unwrap_or((field, ""));
        match name {
            "n" => {
                let width = spec
                    .trim_start_matches('0')
                    .trim_end_matches('d')
                    .parse::<usize>()
                    .unwrap_or(0);
                if spec.starts_with('0') {
                    out.push_str(&format!("{n:0width$}"));
                } else {
                    out.push_str(&format!("{n:width$}"));
                }
            }
            "prefix" => out.push_str(prefix),
            "key" => out.push_str(key),
            _ => {
                out.push('{');
                out.push_str(field);
                out.push('}');
            }
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

/// All series, by prefix (lower case).
#[derive(Debug, Default)]
pub struct Counters {
    pub by_prefix: BTreeMap<String, Counter>,
}

impl Counters {
    pub fn get(&self, prefix: &str) -> Option<&Counter> {
        self.by_prefix.get(&prefix.to_ascii_lowercase())
    }

    pub fn is_declared(&self, prefix: &str) -> bool {
        self.get(prefix).is_some()
    }

    /// Step 1 of the resolution: predeclared entries, then the front
    /// matter's declarations (which may override fields of a predeclared
    /// entry but not shadow a role name).
    pub fn declare(
        doc: &Document,
        start: &BTreeMap<String, u32>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut counters = Counters::default();
        for p in registry::PREFIXES {
            counters.by_prefix.insert(
                p.name.to_string(),
                Counter {
                    prefix: p.name.to_string(),
                    name: p.label.map(str::to_string),
                    format: None,
                    start: 1,
                    scope: p.scope,
                    reference: p.reference.unwrap_or("{number}").to_string(),
                    tmark_numbered: false,
                    numbers: BTreeMap::new(),
                    next: 1,
                },
            );
        }
        let fm = &doc.front_matter;
        for (prefix, decl) in &fm.keys.press.declare.counters {
            let key = prefix.to_ascii_lowercase();
            if registry::role(&key).is_some() {
                diagnostics.push(Diagnostic::new(
                    Code::PrefixUnknown,
                    fm.meta.span,
                    format!("counter prefix `{prefix}` shadows the role of the same name"),
                ));
                continue;
            }
            let existing = counters.by_prefix.get(&key).cloned();
            let predeclared = existing.is_some();
            let mut counter = existing.unwrap_or(Counter {
                prefix: key.clone(),
                name: None,
                format: None,
                start: 1,
                scope: None,
                reference: String::new(),
                tmark_numbered: true,
                numbers: BTreeMap::new(),
                next: 1,
            });
            if let Some(name) = &decl.name {
                counter.name = Some(name.clone());
            }
            if let Some(format) = &decl.format {
                counter.format = Some(format.clone());
            }
            if let Some(s) = decl.start {
                counter.start = s;
            }
            if let Some(scope) = decl.scope {
                counter.scope = Some(scope);
            }
            // Spec §Counters, `ref`: defaults to `{number}` with a format,
            // `{name} {number}` otherwise.
            counter.reference = match &decl.reference {
                Some(r) => r.clone(),
                None if predeclared => counter.reference.clone(),
                None if counter.format.is_some() => "{number}".to_string(),
                None => "{name} {number}".to_string(),
            };
            counter.next = counter.start;
            counters.by_prefix.insert(key, counter);
        }
        // Theorem kinds declared with a counter of their own.
        for decl in fm.keys.press.declare.admonitions.values() {
            if let Some(prefix) = &decl.counter {
                let key = prefix.to_ascii_lowercase();
                counters.by_prefix.entry(key.clone()).or_insert(Counter {
                    prefix: key,
                    name: decl.name.clone(),
                    format: None,
                    start: 1,
                    scope: Some(Scope::Chapter),
                    reference: "{name} {number}".to_string(),
                    tmark_numbered: true,
                    numbers: BTreeMap::new(),
                    next: 1,
                });
            }
        }
        for (prefix, first) in start {
            if let Some(counter) = counters.by_prefix.get_mut(&prefix.to_ascii_lowercase()) {
                counter.next = *first;
            }
        }
        counters
    }

    /// Step 3: number the labels of the TeXSmith-numbered series in
    /// document order.
    pub fn allocate(&mut self, labels: &mut Labels) {
        for label in labels.in_order.iter_mut() {
            let Some(prefix) = &label.prefix else {
                continue;
            };
            let Some(counter) = self.by_prefix.get_mut(&prefix.to_ascii_lowercase()) else {
                continue;
            };
            if !counter.tmark_numbered || counter.numbers.contains_key(&label.key) {
                continue;
            }
            let n = counter.next;
            counter.next += 1;
            counter.numbers.insert(label.key.clone(), n);
            label.number = Some(n);
        }
        // Mirror the numbers into the by-id map.
        for label in &labels.in_order {
            if let Some(entry) = labels.by_id.get_mut(&label.id.to_ascii_lowercase()) {
                entry.number = label.number;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_python() {
        assert_eq!(format_number("FW-{n:02d}", 7, "fw", "x"), "FW-07");
        assert_eq!(format_number("{prefix}-{n}", 12, "req", "x"), "req-12");
        assert_eq!(format_number("{n}", 3, "", ""), "3");
        assert_eq!(format_number("{key}", 3, "", "boot"), "boot");
    }
}
