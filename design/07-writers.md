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

## Implementation notes (milestone 4)

State of `crates/tmark-writers` at the end of the first M4 writers pass
(branch `wt/writers`), for whoever continues. Specification of the
behaviour: TeXSmith's `specs/migration/writers-and-passes.md` (§1 layout
and options, §2 the LaTeX catalogue, §4 Typst math), the contract names of
`specs/migration/fragment-contracts.md` (§1, §3, §5) and `decisions.md`
(X1–X4). Read those before this section.

### What exists

- `lib.rs`: `Writer`, `Backend` (`html`, `latex`, `typst`), `Media`,
  `WriterOptions { media, lang, code {engine, inline_plain,
  inline_breaks}, latex {legacy_accents}, headings {base_level, numbered},
  refs {textual_print, textual_web}, numbering, typst {math}, source_map
  }`, `Body { text, map, requires }`, `Requires` (design fields plus
  `citations`, `acronyms`), `SourceMap` (serialises as `[[start, end,
  node], …]`), `write(doc, res, backend, opts)`, `writer(backend)`. All
  serde-derived: `tmark-py` and `tmark write --map` hand the JSON over.
- `common/`: `out.rs` (the printer's `Out` plus `begin`/`end` map entries,
  `blank_line` never doubles), `zero.rs` (the zero-width collapse, shared),
  `media.rs`, `refs.rs` (template rendering, `[?key]`, label word
  capitalisation, `Resolved` lookups), `text.rs` (dash/quote/script tables,
  `KEY_LABELS`, `slugify`, `acronym_key`, ASCII fold), `abbr.rs`
  (whole-word acronym substitution in `Str`, see below), `fragments.rs`
  (the contract names as constants — **TODO** switch to
  `tmark_ir::registry::FRAGMENTS` when `wt/registry` lands; the names are
  the `press.fragments` spellings and must stay identical).
- `html/`: the `<article>` innerHTML, one function per node, `data-src`
  only when `source_map` is on, labels numbered in document order (the
  web has no backend counter; sub-figure images take no number),
  footnotes and an author-year bibliography appended, `<abbr>` for
  acronyms. 564 of the 652 CommonMark examples render byte for byte
  (`tests/commonmark.rs`, golden list asserted).
- `latex/`: `mod.rs` (blocks), `inline.rs`, `figure.rs`, `table.rs`,
  `escape.rs` (the `escaper.py` tables and order; no math scan of `Str`).
- `typst/`: `mod.rs`, `inline.rs`, `table.rs`, `math.rs` (mitex),
  `escape.rs`.
- Facade `tmark::write` and re-exports; CLI `tmark write FILE --to
  latex|typst|html [--media print|web] [--map]` (`--map` prints the whole
  `Body` as JSON).
- Tests: `tests/fixtures.rs` snapshots every `spec/conformance` fixture
  per backend, text plus `Requires`, under `tests/snapshots/`
  (`INSTA_UPDATE=always cargo test -p tmark-writers` regenerates; read
  the diff). Unit tests for the escapers, tables layout, widths, math,
  zero-width, slugs.

### Construct coverage

| Construct | LaTeX | Typst | HTML |
| --- | --- | --- | --- |
| Paragraphs, inline text, `\tslead` | done | done | done |
| Headings (levels, `*`, `\label`, slug) | done | done | done (id only when explicit) |
| Lists, tasks (`tstasklist`), definition lists | done | done | done (tightness approximated) |
| Code (`tscode`, `\tscodeinline`, `Div{code}` X3) | done | done (`#ts-code`, `#raw`) | done |
| Pipe and model tables | done | done | done |
| Figures, sub-figures, captions, `\captionof` in a box | done | done | done |
| Footnotes | done (`\par` joins paragraphs) | done | done |
| References, citations, glossary, DOI, external | done | done | done |
| Index (`\tsindex`) | done | done (`#ts-index`, no-op in `texsmith.typ`) | dropped (zero-width) |
| Acronyms (`\tsacr`, key rule, `Requires.acronyms`) | done | done | done |
| Counters | done | done | done |
| Asides, admonitions, keystrokes | done | done | done |
| Math (verbatim; `equation` + `\label` for an anchored block) | done | done (mitex) | done (MathJax delimiters) |
| Links, anchors, textual template | done | done (`{page}` → `#ts-page`, not in the contract yet) | done |
| Raw, comments, zero-width collapse, media | done | done | done |
| `Div` dispatch (`epigraph`, `code`, `tsdiv`) | done | done | done |
| `Include`, `\tsdivider` | done | done | done |
| Scripts, emoji (`\tsscript`, `\tsemoji`) | done, untested on a corpus | done | plain spans |
| Progress bars (`\tsprogress[thin]{0.45}{label}`) | done | done (`#ts-progress`) | done (`<progress>` in a `.progress` span) |
| `multicolumn`/`div` containers | via `tsdiv` | via `#ts-div` | `<div class="multicolumn">`, `<div class="…">` |
| Tabs (`tsdiv{tab}[title=…]` in sequence) | done | done | done (`tabbed-set` / `tabbed-labels` / `tabbed-block`) |
| TeX logos (`typography.tex-logos`) | done (`\LaTeX{}`, `\tslogo{…}`) | done (`#ts-logo`) | done (`<span class="tex-logo">`) |
| `.unnumbered` / `.unlisted` headings | done (`\section*`, `\addcontentsline` kept for unnumbered only) | done (`numbering: none`, `outlined: false`) | classes |
| Implicit heading ids | `\label` only when referenced | same | none (the site slugs) |
| Foreign directives, icon spans | dropped | dropped | dropped / `<span class="icon">` |

### Decisions taken here (not in the notes)

- Blocks are separated by exactly one blank line in every backend; the
  legacy `\n` join with per-emitter trailing newlines is not reproduced
  (parity normalisation collapses blank runs anyway).
- `\ref` and `\hyperref` use the label *as defined* (`Labels::get(key).id`),
  because TMark matches keys case-insensitively and LaTeX does not.
- A backend-numbered reference renders the `ref` template with
  `{number}` → `\ref{key}` and a `~` between name and number
  (`Figure~\ref{fig:x}`); a TMark-numbered one renders the text inside
  `\hyperref[key]{…}`. A capitalised prefix capitalises the label word;
  a lower-case one keeps the declared word.
- Citations: a bracketed group → `\cite{k1,k2}` (locators as
  `\cite[pre][post]{k}`), a bare `@key` → `\textcite{key}` (and
  `ts-bibliography` in `Requires.fragments` for the fallback),
  `-@key` → `\citeyear`. Typst: `#cite(<k>, form: "prose")` for bare.
- An `Image` alone in a paragraph is a figure; its alt is the caption when
  no caption line follows (legacy `render_images`), and the short caption
  when one does and the alt is not longer. Inside `tscallout`/`tscode`
  the figure is `center` + `\captionof{figure}`.
- `::: figure` with several images → `subfigure` (package `subcaption`),
  `cols` per row; Typst uses a `grid`.
- The acronym substitution lives in the writers (`common/abbr.rs`): the
  parser fills `Document.abbreviations` but emits no `Abbr` node, so
  `Str` runs are split at whole-word keys (`*[X]:` and
  `press.declare.acronyms`). When the parser emits `Abbr`, the helper
  finds nothing and can be deleted. `examples/abbr` and
  `examples/glossary` render `\tsacr` with this.
- `Div{name=latex|typst|html}` is rendered as raw text of its paragraphs
  by the matching backend: a shim for the `/// latex` slash blocks the
  parser still lowers to a container (deprecation `slash-raw-block`, on
  the `wt/fixes` list). Delete when the parser lowers them to `RawBlock`.
- `WriterOptions.latex.legacy_accents` is accepted and ignored (the
  `pylatexenc` path has no Rust twin; engines read UTF-8).
- The Greek subscript entries map to `\beta` etc. as the legacy table did
  (`escaper.py:223`, latent bug reproduced for parity; fix both sides).
- HTML `id`s on headings only when written; auto-slugs are LaTeX/Typst
  labels only (open question b: `python-slugify` on the plain text).
- `Requires.packages` lists what *structural* output needs (`ulem`,
  `csquotes`, `booktabs`, `tabularx`, `longtable`, `multirow`, `float`,
  `graphicx`, `caption`, `subcaption`, `enumitem`, `babel`, `glossaries`,
  `imakeidx`); contract packages come from the `FRAGMENTS` table on
  TeXSmith's side.

### What is next

1. Switch `common/fragments.rs` to `tmark_ir::registry::FRAGMENTS` and
   `KEY_LABELS` once `wt/registry` merges; add a test that every
   fragment name the writers emit is a row of the table.
2. Run the TeXSmith parity harness (`scripts/parity.py --reader tmark`)
   and triage: expected differences are `\item{}` → `\item`, the
   zero-width spacing, `\clearpage` → `\tsdivider`, `\index` →
   `\tsindex`, `\acrshort` → `\tsacr`, `\marginnote` → `\tsaside`,
   `\keystroke` → `\tskeys`, `callout` → `tscallout`, `code` → `tscode`,
   blank-line runs, heading slugs of headings containing inline markup.
3. `Requires.assets` for generated images (`Image` with empty `src` and
   `generate=`): the writers emit nothing for them today; decide with the
   assets pass whether the writer should still list them.
4. Listing captions: `tscode` receives `caption={…}`; the fragment
   contract of fragment-contracts.md §5 does not list that key yet — add
   it there or drop it here.
5. Typst: `#ts-page(<id>)` for `{page}` in a textual template and
   `#ts-task`, `#ts-acr`, `#ts-code`, `#ts-index`, `#ts-anchor`-less
   anchors (`#metadata(none) <id>`) need their `texsmith.typ`
   definitions on TeXSmith's side; nothing here compiles a `.typ` yet.
6. Typst preview in the LSP (ADR 0005) and the source-map consumers
   (`%` line markers, SyncTeX sidecar) are untouched.
7. The CommonMark failures are IR-level: list tightness is not in the IR
   (an `Item`/`List` `tight` flag would fix ~15 examples), URL
   percent-encoding and entity decoding, raw HTML blocks dropped, tabs
   in indented code. None is a writer bug.
8. Table validation (X9, `wt/tables`): once nested per-group cells and
   mapping rows parse, `examples/tables` renders through the model path;
   today those fences fall back to code blocks in the parser.

### Pitfalls

- `Out::scratch()` for anything rendered aside (captions, titles, cell
  text): `render_inlines` swaps the buffer and loses map entries inside;
  push the enclosing node's `begin`/`end` at the outer level.
- `blank_line` is idempotent; `ensure_newline` is what a nested list
  wants after `\item text`.
- `zero::collapse` clones the inline sequence; it is applied at every
  `inlines()` call, so nested content is collapsed at its own level.
- `in_box` (LaTeX) is what decides `\captionof`; `in_cell` decides
  `\newline` for a hard break.
- `render_blocks_inline` joins paragraphs with `\par ` (LaTeX) and
  `#parbreak()` (Typst); a footnote with a list inside renders the list
  environment inline, which LaTeX accepts.
- The fixture snapshot names are `<fixture>@<backend>`; a new fixture
  needs `INSTA_UPDATE=always` once, then review the three new files.

### Decisions of the C31–C42 wave

- `Div{tabs}` is transparent on the paged backends: its `tab` children
  render in sequence through the generic `tsdiv` / `#ts-div` contract,
  `title=` forwarded as a key (spec §Tabs: "the title in bold, then the
  content" is the fragment's default). The HTML writer mirrors Material's
  `tabbed` markup without the radio inputs.
- TeX logos are a split of `Str` runs like the acronym substitution
  (`common/logos.rs`), so code, math, raw text, destinations and attribute
  values never see them. `\TeX{}`, `\LaTeX{}` and `\LaTeXe{}` are the
  kernel's; the other words are `\tslogo{Name}`, a new macro of the
  `ts-typesetting` contract (`FRAGMENTS`), `#ts-logo("Name")` in Typst.
  The feature is read from the document's front matter, not from
  `WriterOptions`.
- A heading without `{#id}` gets a `\label` / `<label>` only when a
  reference of the document resolves to its implicit id
  (`common::refs::referenced_implicit_id`); the earlier unconditional
  `\label{slug}` is gone, so unreferenced headings carry no label. HTML
  keeps writing ids only when explicit: the site's slugifier owns the rest.
- `ProgressBar` values are fractions with at most four decimals
  (`text::trim_float`); the label defaults to the percentage.
