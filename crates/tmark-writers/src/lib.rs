//! HTML, LaTeX and Typst body writers with source maps and `Requires`.
//!
//! Design: `design/07-writers.md`; behaviour catalogue and option names:
//! TeXSmith's `specs/migration/writers-and-passes.md` §1–§2, contract macro
//! names: `specs/migration/fragment-contracts.md` §1 and §3.
//!
//! A body is the part of a target document a template wraps: what goes
//! between `\begin{document}` and `\end{document}`, the content after a
//! Typst template's `#show` rules, the `<article>` innerHTML. It never
//! contains a preamble; it reports what it [`Requires`]. The CommonMark
//! writer is `tmark-fmt`'s `Mkdocs` profile (architecture review C4), not a
//! module here.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use serde::{Deserialize, Serialize};
use tmark_ir::{Document, NodeId};
use tmark_registry::Resolved;

pub mod common;
pub mod html;
pub mod latex;
pub mod mkdocs;
pub mod typst;

pub use html::HtmlWriter;
pub use latex::LatexWriter;
pub use mkdocs::{lower_web, Citations, Lowered, SectionRefs, WebOptions};
pub use typst::TypstWriter;

/// The Typst side of the fragment contracts: a definition for every
/// `#ts-…` function the Typst writer emits (fragment-contracts.md §3 rule
/// 7). TeXSmith writes it next to the `.typ` it builds as `texsmith.typ`;
/// a body compiles with `#import "texsmith.typ": *` (and the `mitex`
/// import when `Requires.packages` names it) in front of it.
pub const TEXSMITH_TYP: &str = include_str!("../assets/texsmith.typ");

/// A target language. Design 07 §The trait.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Html,
    Latex,
    Typst,
}

impl Backend {
    /// `html`, `latex` or `typst`.
    pub fn parse(name: &str) -> Option<Backend> {
        match name {
            "html" => Some(Backend::Html),
            "latex" | "tex" => Some(Backend::Latex),
            "typst" | "typ" => Some(Backend::Typst),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Html => "html",
            Backend::Latex => "latex",
            Backend::Typst => "typst",
        }
    }
}

/// The medium a body is written for (spec §Roles, `media=`). Design 07
/// §Mapping rules: a node restricted to the other medium is skipped.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Media {
    #[default]
    Print,
    Web,
}

/// The LaTeX code engine the `ts-code` fragment will use. The writer learns
/// it only to set [`Requires::shell_escape`] (fragment-contracts.md §5).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodeEngine {
    #[default]
    Pygments,
    Minted,
    Listings,
    Verbatim,
}

/// `code.*` options (writers-and-passes.md §1: `writer.py:633`,
/// `formatter.py:79-82`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CodeOptions {
    pub engine: CodeEngine,
    /// Inline code is plain typewriter text, no highlighting.
    pub inline_plain: bool,
    /// Characters after which an inline code span may break
    /// (`\allowbreak{}`), `code.inline.breaks` (0.6.0).
    pub inline_breaks: String,
}

impl Default for CodeOptions {
    fn default() -> Self {
        CodeOptions {
            engine: CodeEngine::Pygments,
            inline_plain: false,
            inline_breaks: "-".to_string(),
        }
    }
}

/// LaTeX-only knobs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LatexOptions {
    /// The `pylatexenc` accent path of `escape_latex_chars` (`state.py:45`).
    /// Accepted and recorded; the writer keeps UTF-8 text as is (the
    /// engines TeXSmith drives read UTF-8 natively).
    pub legacy_accents: bool,
}

/// Heading levels per slot body (decisions.md X1): the absolute level is
/// `Header.level + base_level - 1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HeadingOptions {
    /// 0 makes `#` a chapter, −1 a part; 1 (the default) a section.
    pub base_level: i8,
    /// `false` writes starred sectioning commands.
    pub numbered: bool,
}

impl Default for HeadingOptions {
    fn default() -> Self {
        HeadingOptions {
            base_level: 1,
            numbered: true,
        }
    }
}

/// Templates of textual references, per medium (spec §Ref: fields `{text}`,
/// `{number}`, `{page}`; `{page}` is expanded by the backend).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RefOptions {
    pub textual_print: String,
    pub textual_web: String,
}

impl Default for RefOptions {
    fn default() -> Self {
        RefOptions {
            textual_print: "{text}".to_string(),
            textual_web: "{text}".to_string(),
        }
    }
}

/// Who numbers a series (design 07 §WriterOptions).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Numbering {
    /// The backend (`\ref`, `@label`).
    Backend,
    /// TMark: the number is in `Resolved` and rendered as text.
    Tmark,
}

/// How the Typst writer renders LaTeX math (writers-and-passes.md §4).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TypstMath {
    /// `#mi(…)` / `#mitex(…)` through the `mitex` package.
    #[default]
    Mitex,
    /// A native translator (post-M5); falls back to `Mitex` today.
    Native,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TypstOptions {
    pub math: TypstMath,
}

/// Everything a writer is told besides the document and its resolution.
/// A plain struct (design 07): anything needing knowledge of a template is
/// a fragment contract, not an option.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WriterOptions {
    pub media: Media,
    /// The document language (front matter `lang`); a span whose `lang`
    /// differs gets `\foreignlanguage` / `#text(lang:)` / `lang=`.
    pub lang: Option<String>,
    pub code: CodeOptions,
    pub latex: LatexOptions,
    pub headings: HeadingOptions,
    pub refs: RefOptions,
    /// Per-series override of who numbers it; absent series follow
    /// `Counter::tmark_numbered`.
    pub numbering: BTreeMap<String, Numbering>,
    pub typst: TypstOptions,
    /// Record output ranges in [`Body::map`]; off leaves the map empty.
    pub source_map: bool,
}

/// An image (or included file) a body references, with the attributes that
/// affect its conversion.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    pub src: String,
    pub node: NodeId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attrs: Vec<(String, String)>,
}

/// What a body needs from its template: packages, fragment contracts,
/// assets, registries. Design 07 §What a body is, plus `citations` and
/// `acronyms` (writers-and-passes.md §1).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Requires {
    /// LaTeX packages structural output needs (`booktabs`, `csquotes`)
    /// plus those the named fragments imply (`Requires::close`); Typst
    /// packages (`@preview/mitex:0.2.7`).
    pub packages: BTreeSet<String>,
    /// Fragment contracts named (`ts-code` provides `tscode`), rows of
    /// `tmark_ir::registry::FRAGMENTS`.
    pub fragments: BTreeSet<String>,
    pub shell_escape: bool,
    pub assets: Vec<AssetRef>,
    pub bibliography: bool,
    /// Cited keys, first seen first.
    pub citations: Vec<String>,
    /// `\tsacr{key}` keys emitted, first seen first.
    pub acronyms: Vec<String>,
    /// Index registries used; `""` is the default registry.
    pub index: BTreeSet<String>,
    /// TMark-numbered series used.
    pub counters: BTreeSet<String>,
}

impl Requires {
    /// Names a fragment contract. The name must be a row of
    /// `tmark_ir::registry::FRAGMENTS` (checked in debug builds; the
    /// fixture snapshots check it in release).
    pub fn fragment(&mut self, name: &str) {
        debug_assert!(
            tmark_ir::registry::fragment(name).is_some(),
            "`{name}` is not a registered fragment contract"
        );
        self.fragments.insert(name.to_string());
    }

    /// Merges the packages (and `shell_escape`) every named fragment
    /// implies, from the registry; a writer calls it once at the end.
    pub fn close(&mut self) {
        for name in &self.fragments {
            if let Some(fragment) = tmark_ir::registry::fragment(name) {
                self.packages
                    .extend(fragment.packages.iter().map(|p| p.to_string()));
                self.shell_escape |= fragment.shell_escape;
            }
        }
    }

    pub fn package(&mut self, name: &str) {
        self.packages.insert(name.to_string());
    }

    pub fn cite(&mut self, key: &str) {
        self.bibliography = true;
        if !self.citations.iter().any(|k| k == key) {
            self.citations.push(key.to_string());
        }
    }

    pub fn acronym(&mut self, key: &str) {
        if !self.acronyms.iter().any(|k| k == key) {
            self.acronyms.push(key.to_string());
        }
    }
}

/// Output byte range → node (design 07 §Source maps). Serialises as
/// `[[start, end, node_id], …]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    pub entries: Vec<(Range<u32>, NodeId)>,
}

impl SourceMap {
    /// The innermost entry covering `offset`, if any.
    pub fn node_at(&self, offset: u32) -> Option<NodeId> {
        self.entries
            .iter()
            .filter(|(range, _)| range.start <= offset && offset < range.end)
            .min_by_key(|(range, _)| range.end - range.start)
            .map(|(_, id)| *id)
    }
}

impl Serialize for SourceMap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.entries.len()))?;
        for (range, id) in &self.entries {
            seq.serialize_element(&(range.start, range.end, id.0))?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for SourceMap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows: Vec<(u32, u32, u32)> = Deserialize::deserialize(deserializer)?;
        Ok(SourceMap {
            entries: rows
                .into_iter()
                .map(|(s, e, id)| (s..e, NodeId(id)))
                .collect(),
        })
    }
}

/// What a writer produces. Design 07.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Body {
    pub text: String,
    #[serde(default)]
    pub map: SourceMap,
    #[serde(default)]
    pub requires: Requires,
}

/// The one abstraction seam of the crate (AGENTS.md: three implementations).
pub trait Writer {
    fn backend(&self) -> Backend;
    fn write(&self, doc: &Document, res: &Resolved, opts: &WriterOptions) -> Body;
}

/// The writer of a backend.
pub fn writer(backend: Backend) -> Box<dyn Writer> {
    match backend {
        Backend::Html => Box::new(HtmlWriter),
        Backend::Latex => Box::new(LatexWriter),
        Backend::Typst => Box::new(TypstWriter),
    }
}

/// Write `doc` for `backend`.
pub fn write(doc: &Document, res: &Resolved, backend: Backend, opts: &WriterOptions) -> Body {
    writer(backend).write(doc, res, opts)
}
