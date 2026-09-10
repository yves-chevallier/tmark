//! Step 2 of the resolution: every definition of the document and of its
//! includes, in document order (design 06 §Resolution algorithm).

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use tmark_ir::{
    plain_text, walk, Block, CaptionKind, Code, Diagnostic, Document, FileId, Inline, NodeId,
    NodeRef, Span,
};

use crate::counters::Counters;
use crate::loader::{join, Loader};

/// What an anchor sits on; the host decides the counter (spec §Anchor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Header,
    Table,
    Figure,
    Listing,
    Equation,
    /// A theorem-like admonition; the counter is the declared one.
    Admonition,
    /// A counter item, `#(prefix:key)`.
    CounterItem,
    /// A span or another element with no counter: an anchor only.
    Anchor,
}

impl Host {
    /// The conventional prefix of the host, when it has a counter.
    pub fn prefix(self) -> Option<&'static str> {
        match self {
            Host::Header => Some("sec"),
            Host::Table => Some("tbl"),
            Host::Figure => Some("fig"),
            Host::Listing => Some("lst"),
            Host::Equation => Some("eq"),
            Host::Admonition | Host::CounterItem | Host::Anchor => None,
        }
    }
}

/// A defined label: an `#id`, a counter item, a captioned float.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    /// The id as written (`sec:intro`, `fw:boot-loop`, `stock`).
    pub id: String,
    /// The series it counts in (`sec`, `fw`), when it counts.
    pub prefix: Option<String>,
    /// The part after the prefix (`intro`), or the whole id.
    pub key: String,
    pub host: Host,
    pub node: NodeId,
    pub span: Span,
    /// Position in the series (TeXSmith-numbered series only).
    pub number: Option<u32>,
}

/// All labels, by lower-cased id and in document order.
#[derive(Debug, Default)]
pub struct Labels {
    pub by_id: BTreeMap<String, Label>,
    pub in_order: Vec<Label>,
}

impl Labels {
    pub fn get(&self, id: &str) -> Option<&Label> {
        self.by_id.get(&id.to_ascii_lowercase())
    }
}

/// Index entries by registry name (`None` is the default registry).
#[derive(Debug, Default)]
pub struct IndexTable {
    pub entries: BTreeMap<Option<String>, Vec<(NodeId, Vec<String>)>>,
}

pub struct Collector<'a> {
    loader: &'a dyn Loader,
    counters: &'a Counters,
    seen: HashSet<PathBuf>,
    next_file: u32,
    pub labels: Labels,
    pub index: IndexTable,
    pub files: Vec<(FileId, PathBuf)>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> Collector<'a> {
    pub fn new(loader: &'a dyn Loader, counters: &'a Counters) -> Self {
        Collector {
            loader,
            counters,
            seen: HashSet::new(),
            next_file: 1,
            labels: Labels::default(),
            index: IndexTable::default(),
            files: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Collect `doc` (parsed from `path`), then its includes depth-first.
    pub fn collect(&mut self, doc: &Document, path: &Path) {
        self.seen.insert(path.to_path_buf());
        let mut includes: Vec<(Span, String, Option<String>)> = Vec::new();
        walk(doc, &mut |node: NodeRef| match node {
            NodeRef::Block(block) => self.block(block, &mut includes),
            NodeRef::Inline(inline) => self.inline(inline),
        });
        for (span, rel, base) in includes {
            let from = match &base {
                Some(base) => join(path, &format!("{base}/")),
                None => path.to_path_buf(),
            };
            let target = join(&from, &rel);
            if self.seen.contains(&target) {
                continue;
            }
            match self.loader.load(&from, &rel) {
                Some(text) => {
                    let id = FileId(self.next_file);
                    self.next_file += 1;
                    self.files.push((id, target.clone()));
                    let parsed = tmark_syntax::parse(&text, id);
                    self.diagnostics.extend(parsed.diagnostics);
                    self.collect(&parsed.document, &target);
                }
                None => self.diagnostics.push(Diagnostic::new(
                    Code::IncludeMissing,
                    span,
                    format!("included file `{rel}` not found"),
                )),
            }
        }
    }

    fn block(&mut self, block: &Block, includes: &mut Vec<(Span, String, Option<String>)>) {
        match block {
            Block::Header(h) => self.define(h.attrs.id(), Host::Header, h.meta.id, h.meta.span),
            Block::Caption(c) => {
                let host = match c.kind {
                    CaptionKind::Table => Host::Table,
                    CaptionKind::Figure => Host::Figure,
                    CaptionKind::Listing => Host::Listing,
                };
                self.define(c.attrs.id(), host, c.meta.id, c.meta.span);
            }
            Block::Table(t) => self.define(t.attrs.id(), Host::Table, t.meta.id, t.meta.span),
            Block::Figure(f) => self.define(f.attrs.id(), Host::Figure, f.meta.id, f.meta.span),
            Block::MathBlock(m) => {
                self.define(m.attrs.id(), Host::Equation, m.meta.id, m.meta.span)
            }
            Block::CodeBlock(c) => {
                self.define(c.options.id(), Host::Listing, c.meta.id, c.meta.span)
            }
            Block::Admonition(a) => {
                self.define(a.attrs.id(), Host::Admonition, a.meta.id, a.meta.span)
            }
            Block::Div(d) => self.define(d.attrs.id(), Host::Anchor, d.meta.id, d.meta.span),
            Block::BlockQuote(q) => self.define(q.attrs.id(), Host::Anchor, q.meta.id, q.meta.span),
            Block::Include(i) => includes.push((i.meta.span, i.path.clone(), i.base.clone())),
            _ => {}
        }
    }

    fn inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Image(i) => self.define(i.attrs.id(), Host::Figure, i.meta.id, i.meta.span),
            Inline::Span(s) => self.define(s.attrs.id(), Host::Anchor, s.meta.id, s.meta.span),
            Inline::CounterItem(c) => {
                // Spec §CounterItem: "An undeclared prefix warns."
                if !self.counters.is_declared(&c.prefix) {
                    self.diagnostics.push(Diagnostic::new(
                        Code::PrefixUnknown,
                        c.meta.span,
                        format!("counter prefix `{}` is not declared", c.prefix),
                    ));
                }
                let id = format!("{}:{}", c.prefix, c.key);
                self.insert(Label {
                    id,
                    prefix: Some(c.prefix.clone()),
                    key: c.key.clone(),
                    host: Host::CounterItem,
                    node: c.meta.id,
                    span: c.meta.span,
                    number: None,
                });
            }
            Inline::IndexEntry(e) => {
                let path = e.path.iter().map(|group| plain_text(group)).collect();
                self.index
                    .entries
                    .entry(e.registry.clone())
                    .or_default()
                    .push((e.meta.id, path));
            }
            _ => {}
        }
    }

    /// An `#id` on a host. The prefix, when present, must agree with the
    /// host (`prefix-host-mismatch`); a bare id takes the host's counter.
    fn define(&mut self, id: Option<&str>, host: Host, node: NodeId, span: Span) {
        let Some(id) = id else { return };
        let (prefix, key) = match id.split_once(':') {
            Some((p, k)) if !p.is_empty() && !k.is_empty() => (Some(p.to_string()), k.to_string()),
            _ => (None, id.to_string()),
        };
        // Headings take any of the heading-class prefixes (spec Table
        // "Predeclared counter prefixes": part, chap, sec, app).
        let heading_class = host == Host::Header
            && prefix.as_deref().is_some_and(|p| {
                ["part", "chap", "sec", "app"]
                    .iter()
                    .any(|h| p.eq_ignore_ascii_case(h))
            });
        let prefix = match (&prefix, host.prefix()) {
            (Some(p), Some(_)) if heading_class => Some(p.clone()),
            (Some(p), Some(conventional)) if !p.eq_ignore_ascii_case(conventional) => {
                // A predeclared prefix on the wrong host is a mismatch; a user
                // series numbers the host in a custom series (spec §Anchor).
                if tmark_ir::registry::prefix(p).is_some() && p != "gls" && p != "doi" {
                    self.diagnostics.push(Diagnostic::new(
                        Code::PrefixHostMismatch,
                        span,
                        format!("`#{id}` uses the `{p}` prefix on a {conventional} host"),
                    ));
                }
                Some(p.clone())
            }
            (Some(p), _) => Some(p.clone()),
            (None, conventional) => conventional.map(str::to_string),
        };
        self.insert(Label {
            id: id.to_string(),
            prefix,
            key,
            host,
            node,
            span,
            number: None,
        });
    }

    fn insert(&mut self, label: Label) {
        let key = label.id.to_ascii_lowercase();
        if let Some(existing) = self.labels.by_id.get(&key) {
            let mut d = Diagnostic::new(
                Code::LabelDuplicate,
                label.span,
                format!("`#{}` is defined twice", label.id),
            );
            d.related
                .push((existing.span, "first definition".to_string()));
            self.diagnostics.push(d);
            return;
        }
        self.labels.by_id.insert(key, label.clone());
        self.labels.in_order.push(label);
    }
}

/// Glossary and acronym terms: `declare.glossary`, `declare.acronyms`
/// (mappings term → definition) and the `*[KEY]: …` abbreviations.
pub fn glossary(doc: &Document) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let declare = &doc.front_matter.keys.press.declare;
    for value in [&declare.glossary, &declare.acronyms] {
        if let serde_json::Value::Object(map) = value {
            for (term, def) in map {
                let text = match def {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Object(o) => o
                        .get("name")
                        .or_else(|| o.get("description"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    other => other.to_string(),
                };
                out.insert(term.to_ascii_lowercase(), text);
            }
        }
    }
    for abbr in &doc.abbreviations {
        out.entry(abbr.key.to_ascii_lowercase())
            .or_insert_with(|| abbr.expansion.clone());
    }
    out
}
