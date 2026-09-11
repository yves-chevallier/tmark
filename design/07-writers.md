# 07 — Writers and source maps

Crate: `tmark-writers`. Input: `&Document`, `&Resolved`, `&WriterOptions`.
Output: `Body { text: String, map: SourceMap, requires: Requires }`.
Spec: the backend columns of the node catalogue, §Raw passthrough, P5.

## What a body is

The part of a target document that a template wraps: for LaTeX, what goes
between `\begin{document}` and `\end{document}` (or into a fragment file to
`\input`); for Typst, the content after the template's `#show` rules; for
HTML, the `<article>` innerHTML; for CommonMark, a full Markdown document in
the chosen profile (this one *is* a document, since Markdown has no
template).

A body never contains a preamble, a package load, a font selection or a
page geometry. Instead it reports what it **requires**:

```rust
pub struct Requires {
    pub packages: BTreeSet<String>,     // LaTeX: "tcolorbox", "siunitx"; Typst: "ctheorems"
    pub fragments: BTreeSet<String>,    // TeXSmith fragment names: "ts-callouts", "ts-code"
    pub shell_escape: bool,             // minted
    pub assets: Vec<AssetRef>,          // images referenced, with the attrs that affect conversion
    pub bibliography: bool,
    pub index: BTreeSet<String>,        // index registries used
    pub counters: BTreeSet<String>,     // user series used
}
```

TeXSmith turns `Requires` into a preamble. The writer does not know what a
fragment contains; it only names the contract (`ts-callouts` provides
`tscallout`). The list of contracts is the `FRAGMENTS` table in
`tmark_ir::registry`, shared with TeXSmith's fragment loader: one
`Fragment { name, provides, packages, shell_escape, description }` per
bundled `ts-*` fragment, `provides` listing macros (`\tskeys`) and
environments (`tscode`) by their contract name. A writer that emits a
contract macro adds the row's `name` to `Requires.fragments` and the row's
`packages` to `Requires.packages`; `shell_escape` is `false` on every
bundled row (minted is decided from `code.engine`). Keystroke labels come
from `registry::KEY_LABELS` (`03-ir.md` §Closed registries), so LaTeX,
Typst and HTML spell `ctrl` the same way.

## The trait

```rust
pub trait Writer {
    fn backend(&self) -> Backend;
    fn write(&self, doc: &Document, res: &Resolved, opts: &WriterOptions) -> Body;
}
```

Four implementations, one module each: `commonmark`, `html`, `latex`,
`typst`. Inside a module, one function per node (`fn header(&mut self, n:
&Header)`), dispatch by `match`. Shared helpers (escaping tables, attribute
formatting, label formatting from counter `ref` templates) live in
`common.rs`. No inheritance between writers: LaTeX and Typst share helpers,
not a base class.

`WriterOptions` is a plain struct: `media: Print | Web`, `profile` (for
CommonMark), `code: { engine, inline_breaks }`, `refs.textual` templates per
medium, `lang`, and `numbering: Backend | Tmark` per series. Anything
requiring knowledge of a template is not an option; it is a fragment
contract.

## Mapping rules that are not obvious

- **Media.** A node with `media=print` is skipped by the HTML writer;
  `media=web` by LaTeX and Typst. Zero width, whitespace collapse as for
  comments.
- **Raw.** `RawInline`/`RawBlock` of another format are dropped silently.
- **References.** `Label` resolutions render the counter's `ref` template
  (`{name} {number}`) as a hyperlink where the backend numbers, and as
  `\ref`/`@label` where the backend numbers; unresolved ones render
  `[?key]` literally in every backend. Textual references (`[text](#id)`)
  apply `refs.textual.print` or `.web`; `{page}` is expanded by the backend
  (`\pageref`), never here.
- **Citations.** `\cite`/`\parencite`/`\textcite` with locators, `#cite`
  in Typst, `<a>` plus a bibliography list in HTML rendered from the entry
  fields with a minimal built-in style (author-year); CSL is TeXSmith's.
- **Zero-width nodes.** Comments, index entries, anchors, counter
  definitions that print nothing, asides: the writer collapses surrounding
  whitespace and removes it before punctuation (spec §Attributes). This is a
  post-pass over the inline sequence, shared by all writers.
- **Language.** `lang` on a span or the document: `\foreignlanguage`,
  `#text(lang:)`, `lang=`. Typographic spacing (French `;` `:`) is applied
  by the backend engines (`babel`, Typst), never by writers.
- **Tables.** The writer computes the column preamble from the semantic
  model (`X` columns → `tabularx`, spans → `multirow`/`multicolumn`, `long`
  → `longtable`); decimal alignment when the feature is on.
- **Code.** `pygments` engine is TeXSmith's (it needs Python); the LaTeX
  writer emits an engine-neutral `\begin{tscode}[lang, title, linenums,
  hl_lines]` contract from the `ts-code` fragment and lets the fragment
  choose the engine. The HTML writer emits `<pre><code class="language-x">`.
- **Math.** Passed through verbatim to LaTeX; the Typst writer converts
  LaTeX math with a small translator (milestone 4; until then `raw` with a
  diagnostic).
- **Includes.** An `Include` node that survived to the writer (TeXSmith did
  not splice it) renders as `\input{stem}` / `#include "stem"` and reports
  the path in `Requires.assets`.

## Source maps

```rust
pub struct SourceMap { pub entries: Vec<(Range<u32>, NodeId)> }   // output byte range → node
```

Every writer pushes an entry when it starts and ends a block node and for
inline nodes that matter to navigation (references, images, captions). The
map plus `Document` spans give output range → source span. Consumers:

- TeXSmith's LaTeX build converts it to `%` line markers or to a SyncTeX
  sidecar so a PDF viewer can jump to the `.md` line.
- The Typst path uses `typst-ide` span→source, then this map (milestone 4).
- The HTML writer additionally emits `data-src="file:start-end"` on block
  elements so a web preview can scroll-sync.

## Tests

- Snapshot per fixture and per backend (`insta`).
- A "requires" assertion per fixture: the set of fragments and packages a
  construct needs is part of its conformance.
- Round-trip of the CommonMark writer through the parser: the `canonical`
  profile must be the fixed point of `04-printer.md` (the CommonMark writer
  *is* the printer; `tmark-writers::commonmark` re-exports `tmark-fmt`).
