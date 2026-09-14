//! Step 2 of the resolution: every definition of the document and of its
//! includes, in document order (design 06 §Resolution algorithm).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tmark_ir::{
    plain_text, walk, Attrs, Block, CaptionKind, Code, Diagnostic, Document, Figure, FileId, Image,
    Inline, NodeId, NodeRef, Span,
};

use crate::counters::Counters;
use crate::loader::{join, Loader};

/// What an anchor sits on; the host decides the counter (spec §Anchor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Host {
    Header,
    Table,
    Figure,
    /// An image labelled inside a `::: figure` container: a subfigure
    /// (spec §Image, Figure). It counts in no series of its own; see
    /// [`Label::subfigure`].
    Subfigure,
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
            Host::Figure | Host::Subfigure => Some("fig"),
            Host::Listing => Some("lst"),
            Host::Equation => Some("eq"),
            Host::Admonition | Host::CounterItem | Host::Anchor => None,
        }
    }
}

/// What makes a label a subfigure (spec §Image, Figure): the images of a
/// `::: figure` container take no number of the `fig` series; their number
/// is the container's, suffixed with a letter in document order.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct Subfigure {
    /// The id of the container's label: its own `{#id}`, else the id of
    /// the `Figure:` caption that captions it. `None` when the container
    /// carries no label, in which case the subfigure has no number either.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The letter appended to the container's number (`a`, `b`, …).
    /// `None` for the lone image of a container: it *is* the figure, and
    /// shares its number without a suffix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub letter: Option<String>,
}

/// The letter of the `n`-th subfigure of a container, 0-based: `a`, `b`, …
/// The writers spell their `(a)` markers with the same function, so that
/// the markers and the resolved numbers agree.
pub fn subfigure_letter(n: usize) -> String {
    char::from_u32('a' as u32 + (n % 26) as u32)
        .unwrap_or('a')
        .to_string()
}

/// A defined label: an `#id`, a counter item, a captioned float.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct Label {
    /// The id as written (`sec:intro`, `fw:boot-loop`, `stock`).
    pub id: String,
    /// The series it counts in (`sec`, `fw`), when it counts.
    pub prefix: Option<String>,
    /// The part after the prefix (`intro`), or the whole id.
    pub key: String,
    pub host: Host,
    pub node: NodeId,
    /// The whole host node.
    pub span: Span,
    /// The id token alone, when the parser recorded it (design 03
    /// §Identity and spans): what rename and go-to-definition select.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_span: Option<Span>,
    /// Position in the series (tmark-numbered series only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<u32>,
    /// The heading text of a header, the caption text of a caption: what
    /// a sibling document shows for the label (`BookLabel::title`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A heading's implicit id (spec §Header): GitHub's slug of the title,
    /// derived here and never stored in the IR; a reference to it is the
    /// hint `ref-implicit-id`. An explicit `{#id}` of the same spelling
    /// replaces it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub implicit: bool,
    /// Set for a [`Host::Subfigure`]: the container it numbers under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subfigure: Option<Subfigure>,
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
    /// The included documents, parsed: their references resolve too.
    pub documents: Vec<(FileId, Document)>,
    pub diagnostics: Vec<Diagnostic>,
    /// The subfigures of the document being walked, by image node: a
    /// per-file map, since node ids are unique within one parse.
    subfigures: HashMap<NodeId, Subfigure>,
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
            documents: Vec::new(),
            diagnostics: Vec::new(),
            subfigures: HashMap::new(),
        }
    }

    /// Collect `doc` (parsed from `path`), then its includes depth-first.
    pub fn collect(&mut self, doc: &Document, path: &Path) {
        self.seen.insert(path.to_path_buf());
        // Which images are subfigures needs the block structure, which the
        // flat walk below does not have; the includes are collected after
        // it, each with its own map.
        self.subfigures = HashMap::new();
        subfigures(&doc.blocks, &mut self.subfigures);
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
                    self.documents.push((id, parsed.document));
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
            Block::Header(h) => {
                let title = Some(plain_text(&h.content));
                if h.attrs.id().is_some() {
                    self.define(&h.attrs, Host::Header, h.meta.id, h.meta.span, title);
                } else {
                    self.define_implicit(h, title);
                }
            }
            Block::Caption(c) => {
                let host = match c.kind {
                    CaptionKind::Table => Host::Table,
                    CaptionKind::Figure => Host::Figure,
                    CaptionKind::Listing => Host::Listing,
                };
                let title = Some(plain_text(&c.content));
                self.define(&c.attrs, host, c.meta.id, c.meta.span, title);
            }
            Block::Table(t) => self.define(&t.attrs, Host::Table, t.meta.id, t.meta.span, None),
            Block::Figure(f) => self.define(&f.attrs, Host::Figure, f.meta.id, f.meta.span, None),
            Block::MathBlock(m) => {
                self.define(&m.attrs, Host::Equation, m.meta.id, m.meta.span, None)
            }
            Block::CodeBlock(c) => {
                self.define(&c.options, Host::Listing, c.meta.id, c.meta.span, None)
            }
            Block::Admonition(a) => {
                self.define(&a.attrs, Host::Admonition, a.meta.id, a.meta.span, None)
            }
            Block::Div(d) => self.define(&d.attrs, Host::Anchor, d.meta.id, d.meta.span, None),
            Block::BlockQuote(q) => {
                self.define(&q.attrs, Host::Anchor, q.meta.id, q.meta.span, None)
            }
            Block::Include(i) => includes.push((i.meta.span, i.path.clone(), i.base.clone())),
            _ => {}
        }
    }

    fn inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Image(i) => self.define(&i.attrs, Host::Figure, i.meta.id, i.meta.span, None),
            Inline::Span(s) => {
                // The span's text is what a numeric reference to it shows
                // (spec §Anchor, `ref-unnumbered`) and what a sibling
                // document sees as its title.
                let title =
                    Some(tmark_ir::walk::plain_text(&s.content)).filter(|t| !t.trim().is_empty());
                self.define(&s.attrs, Host::Anchor, s.meta.id, s.meta.span, title)
            }
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
                    id_span: (!c.key_span.0.is_empty()).then_some(c.key_span.0),
                    number: None,
                    title: None,
                    implicit: false,
                    subfigure: None,
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
    fn define(
        &mut self,
        attrs: &Attrs,
        host: Host,
        node: NodeId,
        span: Span,
        title: Option<String>,
    ) {
        let Some(id) = attrs.id() else { return };
        // An image inside a `::: figure` is a subfigure, whatever the
        // caller passed (spec §Image, Figure).
        let subfigure = self.subfigures.get(&node).cloned();
        let host = if subfigure.is_some() {
            Host::Subfigure
        } else {
            host
        };
        let id_span = attrs.id_span.map(|s| s.0);
        let (prefix, key) = match id.split_once(':') {
            Some((p, k)) if !p.is_empty() && !k.is_empty() => (Some(p.to_string()), k.to_string()),
            _ => (None, id.to_string()),
        };
        // Headings take any of the heading-class prefixes (spec Table
        // "Predeclared counter prefixes": part, chap, sec, app).
        let heading_class = host == Host::Header
            && prefix
                .as_deref()
                .and_then(tmark_ir::registry::prefix)
                .is_some_and(|p| p.heading);
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
            id_span,
            number: None,
            title,
            implicit: false,
            subfigure,
        });
    }

    /// A heading without `{#id}` gets GitHub's slug of its title as an
    /// implicit id (spec §Header), suffixed `-1`, `-2`, … when the id is
    /// taken, in document order. It counts in the heading series like a
    /// bare explicit id (`@boot-sequence` renders "section 2").
    fn define_implicit(&mut self, h: &tmark_ir::Header, title: Option<String>) {
        let slug = github_slug(title.as_deref().unwrap_or_default());
        if slug.is_empty() {
            return;
        }
        let mut id = slug.clone();
        let mut n = 0;
        while self.labels.by_id.contains_key(&id.to_ascii_lowercase()) {
            n += 1;
            id = format!("{slug}-{n}");
        }
        self.insert(Label {
            key: id.clone(),
            id,
            prefix: Host::Header.prefix().map(str::to_string),
            host: Host::Header,
            node: h.meta.id,
            span: h.meta.span,
            id_span: None,
            number: None,
            title,
            implicit: true,
            subfigure: None,
        });
    }

    fn insert(&mut self, label: Label) {
        let key = label.id.to_ascii_lowercase();
        if let Some(existing) = self.labels.by_id.get(&key) {
            // An explicit id takes an implicit one's place silently: the
            // heading keeps its title, the author chose the id.
            if existing.implicit && !label.implicit {
                let node = existing.node;
                if let Some(at) = self.labels.in_order.iter().position(|l| l.node == node) {
                    self.labels.in_order[at] = label.clone();
                }
                self.labels.by_id.insert(key, label);
                return;
            }
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

/// The images every `::: figure` of `blocks` turns into subfigures, by
/// image node (spec §Image, Figure). A container made of image paragraphs
/// only holds subfigures; anything else is a plain float whose images keep
/// their own numbers, which is what the writers render too.
fn subfigures(blocks: &[Block], out: &mut HashMap<NodeId, Subfigure>) {
    for (i, block) in blocks.iter().enumerate() {
        if let Block::Figure(figure) = block {
            // The caption of a container may follow it (spec §Caption,
            // attachment): the writers look for it in the same order.
            let following = match blocks.get(i + 1) {
                Some(Block::Caption(c)) if c.kind == CaptionKind::Figure => c.attrs.id(),
                _ => None,
            };
            container(figure, following, out);
        }
        match block {
            Block::BlockQuote(n) => subfigures(&n.content, out),
            Block::Figure(n) => subfigures(&n.content, out),
            Block::Admonition(n) => subfigures(&n.content, out),
            Block::Div(n) => subfigures(&n.content, out),
            Block::BulletList(n) => {
                for item in &n.items {
                    subfigures(&item.content, out);
                }
            }
            Block::OrderedList(n) => {
                for item in &n.items {
                    subfigures(&item.content, out);
                }
            }
            Block::DefinitionList(n) => {
                for (_, definitions) in &n.items {
                    for definition in definitions {
                        subfigures(definition, out);
                    }
                }
            }
            _ => {}
        }
    }
}

/// The images a `::: figure` container turns into subfigures, in document
/// order (spec §Image, Figure): the images of its paragraphs when every
/// block is a paragraph of images only, empty otherwise — a container with
/// a table, prose or a listing in it is a plain float. The writers lay out
/// and letter exactly this list, so their `(a)` markers and the resolved
/// numbers agree.
pub fn figure_images(figure: &Figure) -> Vec<&Image> {
    let content = match figure.content.split_last() {
        Some((Block::Caption(_), rest)) => rest,
        _ => figure.content.as_slice(),
    };
    let paragraphs: Vec<Vec<&Image>> = content.iter().map(image_paragraph).collect();
    if paragraphs.is_empty() || paragraphs.iter().any(|p| p.is_empty()) {
        return Vec::new();
    }
    paragraphs.into_iter().flatten().collect()
}

/// One container: its images, the label they number under, and their
/// letters.
fn container(figure: &Figure, following: Option<&str>, out: &mut HashMap<NodeId, Subfigure>) {
    let inner = match figure.content.split_last() {
        Some((Block::Caption(c), _)) => c.attrs.id(),
        _ => None,
    };
    let parent = figure
        .attrs
        .id()
        .or(following)
        .or(inner)
        .map(str::to_string);
    let images = figure_images(figure);
    if images.is_empty() {
        return;
    }
    if let [image] = images.as_slice() {
        // A lone image is the figure itself: it shares the container's
        // number, and is the container's own label when it has none.
        if parent.is_some() {
            out.insert(
                image.meta.id,
                Subfigure {
                    parent,
                    letter: None,
                },
            );
        }
        return;
    }
    for (n, image) in images.iter().enumerate() {
        out.insert(
            image.meta.id,
            Subfigure {
                parent: parent.clone(),
                letter: Some(subfigure_letter(n)),
            },
        );
    }
}

/// The images of a paragraph made of images only.
fn image_paragraph(block: &Block) -> Vec<&Image> {
    let Block::Para(p) = block else {
        return Vec::new();
    };
    let only = p.content.iter().all(|i| match i {
        Inline::Image(_) | Inline::SoftBreak(_) | Inline::LineBreak(_) | Inline::Space(_) => true,
        Inline::Str(s) => s.text.trim().is_empty(),
        _ => false,
    });
    if !only {
        return Vec::new();
    }
    p.content
        .iter()
        .filter_map(|i| match i {
            Inline::Image(image) => Some(image),
            _ => None,
        })
        .collect()
}

/// GitHub's heading slug (spec §Header): the plain title lower-cased,
/// every character that is not a letter, a digit, a combining mark, a
/// space, `-` or `_` removed, runs of spaces turned into one `-`. Accents
/// and non-Latin scripts survive. Editors compute go-to-target with the
/// same function. (The text is taken as it is; NFC normalisation is left
/// to the editor that saved the file.)
pub fn github_slug(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut pending_space = false;
    for c in title.trim().chars().flat_map(char::to_lowercase) {
        if c == ' ' {
            pending_space = true;
        } else if c.is_alphanumeric() || c == '-' || c == '_' || is_combining_mark(c) {
            if pending_space {
                out.push('-');
                pending_space = false;
            }
            out.push(c);
        }
    }
    out
}

/// The combining-mark blocks (Unicode general category Mn/Mc, by range:
/// the standard library has no category test).
fn is_combining_mark(c: char) -> bool {
    matches!(
        c as u32,
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F
    )
}

/// Glossary and acronym terms: the terms of `declare.glossary` — whichever
/// of the two spellings declared them (`tmark_ir::frontmatter::GlossaryDecl`;
/// its `style` and `groups` are form, not terms) —, `declare.acronyms`
/// (a mapping term → definition) and the `*[KEY]: …` abbreviations.
pub fn glossary(doc: &Document) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let declare = &doc.front_matter.keys.press.declare;
    for (term, entry) in &declare.glossary.entries {
        out.insert(term.to_ascii_lowercase(), entry.definition().to_string());
    }
    if let serde_json::Value::Object(map) = &declare.acronyms {
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
    for abbr in &doc.abbreviations {
        out.entry(abbr.key.to_ascii_lowercase())
            .or_insert_with(|| abbr.expansion.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::github_slug;

    #[test]
    fn slugs() {
        assert_eq!(github_slug("Boot sequence"), "boot-sequence");
        assert_eq!(github_slug("  Hello,  World!  "), "hello-world");
        assert_eq!(github_slug("Élan vital"), "élan-vital");
        assert_eq!(github_slug("日本語 見出し"), "日本語-見出し");
        assert_eq!(github_slug("a_b-c.d"), "a_b-cd");
        assert_eq!(github_slug("e\u{301}t\u{e9}"), "e\u{301}t\u{e9}");
        assert_eq!(github_slug("!!!"), "");
    }
}
