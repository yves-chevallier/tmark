# 11 — Roadmap and definitions of done

Milestones are ordered so that each one is usable on its own and never needs
a later one to be valuable. Do not start a milestone before the previous one
is done, except for spikes that end in an ADR.

## M0 — Foundations (this repository as delivered)

- Spec ported, design documents, ADRs, Cargo skeleton, VS Code grammar and
  its test harness, conformance fixture format with seed fixtures.
- Done when: `cargo build --workspace` succeeds on the skeleton, `npm test`
  passes in `editors/vscode`, and every open question in
  `12-spec-challenges.md` has an owner (spec or code).

## M1 — Parse, print, lint syntactically

Crates: `tmark-ir`, `tmark-syntax`, `tmark-fmt`, `tmark` (facade),
`tmark-cli` (`parse`, `fmt`, `schema`).

Status: **closed (2026-09-11)**. `tmark-ir` done (schema committed);
`tmark-markdown` vendored with the TMark constructs, CommonMark suite
green; `tmark-syntax` lowers every construct of `02-syntax.md` (the
deferred ones are the three `compat-unsupported` spellings of M5), with
totality property tests; the facade exposes `parse` and `schema`; the
conformance runner (`crates/tmark/tests/conformance.rs`) checks 87
fixtures (`input` and `canonical` blocks against the `ir` block,
diagnostics on the first input); `tmark parse`, `tmark fmt` and `tmark
schema` work; `tmark-fmt` prints every node, the canonical block of every
fixture is a fixed point, the editor sample, the spec and TeXSmith's
documentation round-trip and format idempotently, and `edit` /
`edit_many` splice nodes. The last gate fell on the TeXSmith side:
TeXSmith renders the caption line placed *after* its float (commit
`bfad8d7` on its branch `tmark-migration`, `feat(captions): render a
caption line placed after its float`), so `tmark fmt spec/tmark.md`
builds with TeXSmith unchanged in meaning: the LaTeX of the formatted
spec is the LaTeX of the shipping spelling. The parser's performance
target (`02-syntax.md`; the tokenizer is superlinear in the number of
blocks, upstream too) is not an M1 blocker and stays listed under M4's
remaining items.

- Vendor `markdown-rs`; CommonMark spec tests green with the documented
  exceptions.
- Every construct of `02-syntax.md`'s table implemented, with its fixture.
- IR complete per `03-ir.md`, JSON schema generated and committed.
- Printer with the `Canonical` profile; round-trip and idempotence property
  tests green.
- Parse diagnostics.
- Done when: `tmark fmt --check` is a no-op on `spec/tmark.md` after one
  `tmark fmt --write`, and the file still builds with TeXSmith unchanged in
  meaning (compare the LaTeX TeXSmith produces before and after).

## M2 — Resolve and lint

Crates: `tmark-registry`, `tmark-lint`, `tmark-cli` (`lint`, `check`).

- Registries and resolution per `06-registries.md`, including `.bib` and
  inventories through `Loader`.
- Lint rule catalogue of `05-diagnostics.md`, fixes through `edit`.
- Done when: `tmark check spec/tmark.md` reports exactly the known
  unresolved references of the spec (it cites appendices by label) and
  nothing else, and a fixture exists per diagnostic code.

Status: done. `tmark check spec/tmark.md` resolves every reference of the
spec (none is unresolved) and reports only hints (13 position words, 2
hard-coded numbers, all genuine prose). Fixtures exist for every resolve and
lint code that is implemented; open items are listed in the implementation
notes of `05-diagnostics.md` and `06-registries.md`.

## M3 — Language server and editor

Crates: `tmark-lsp`; `editors/vscode` gains the client; the grammar
generator moves to `tmark-ir`.

- Features of `08-lsp.md` up to code actions and document links.
- Front-matter completion from the schema, merged with an external `press`
  schema when configured.
- Done when: the extension, installed from a `.vsix`, gives completion on
  `@` and `{`, formats on save, shows unresolved references, and renames a
  label across a two-file document with an include.

Status (2026-09-10): the server implements diagnostics (parse at once,
resolve and lint after a 150 ms debounce), document symbols, folding,
formatting, semantic tokens (resolution state), completion (`@`, `{`, role
keys, `:::`, fence node words, front-matter keys), definition, references,
rename across an include, hover, document links, quick fixes for
deprecated spellings, `tmark.toml`; the extension bundles the binary and
packages as a `.vsix` (verified with `vsce`, not yet installed in a VS Code
session by a person). The grammar tables come from `tmark-ir`; the
external `press` schema of `tmark.toml` is merged into front-matter
completion; included files get their parse diagnostics published under
their own URI; completion also covers `{.` classes, `{{` paths and
image/include paths. References inside included files resolve and are
reported under their file; the printer round-trips the TeXSmith
documentation (`04-printer.md`). Open: `lint --fix` and the code actions
cover `deprecated` only; range formatting; a real VS Code session. See
`13-handoff.md`.

## M4 — Writers and preview

Crates: `tmark-writers` (LaTeX, Typst, HTML; the CommonMark output is the
`Mkdocs` profile of `tmark-fmt`, review C4), the `typst` crate in
`tmark-lsp`.

- Bodies, `Requires`, source maps per `07-writers.md`.
- TeXSmith switches its reader to the IR (its side of the work; the
  contract is this repository's schema and `Requires`).
- Typst preview in the editor with click-to-source.
- Done when: the TeXSmith documentation builds to PDF through LaTeX from
  bodies produced here with output equivalent to the HTML-reader path, and
  the preview updates in under a second on a chapter-sized file.

Status (2026-09-11, end of the TeXSmith-migration waves): the writers
exist and TeXSmith renders through them; the preview does not exist and
the parity diff is not triaged, so M4 stays **open on those two items**.
What exists:

- `crates/tmark-writers`: `Writer`, `Backend`, `WriterOptions`, `Body`,
  `Requires`, `SourceMap`, the `html`, `latex` and `typst` writers over
  every construct of the catalogue (the coverage table of `07-writers.md`
  §Implementation notes (milestone 4)), the escapers ported from
  TeXSmith's `escaper.py`, the zero-width collapse, `common/abbr.rs` and
  `common/logos.rs`. `tests/fixtures.rs` snapshots every conformance
  fixture per backend (`tests/snapshots/`, 349 files: text plus
  `Requires`); `tests/commonmark.rs` matches 564 of the 652 CommonMark
  examples byte for byte with a golden list; `tests/images.rs` and
  `tests/web.rs` cover icons, converted diagrams and the web lowering.
- The contract with TeXSmith's fragments: `FRAGMENTS` and `KEY_LABELS` in
  `tmark_ir::registry` (read by the writers and served by
  `tmark.fragments()` / `tmark.registries()`), `assets/texsmith.typ`
  (`tmark_writers::TEXSMITH_TYP`, one default per `#ts-…` function the
  Typst writer emits), `\tsdivider` (C36), `\tslogo` (C33), `tsdiv`
  (C32), the `tscode` caption key.
- `tmark write FILE --to latex|typst|html [--media print|web] [--map]`;
  the facade's `tmark::write`.
- The web side of the migration (TeXSmith `web-profile.md`, decision X8):
  `Profile::Mkdocs` in `tmark-fmt` as a spelling table (`04-printer.md`),
  `edit_many` (batch of disjoint local edits, `Replacement::Text`),
  `mkdocs::lower_web(text, doc, res, loader, opts) -> Lowered { text,
  diagnostics, bibliography }` in `tmark-writers`, and `tmark lower FILE
  --to web [--sections] [--citations] [--bib]`.
- Site-wide resolution in `tmark-registry` (`06-registries.md` §Site-wide
  resolution): `ResolveOptions::numbering = All` numbers every
  predeclared series continuously, `ResolveOptions::book` and
  `Resolution::Sibling` link pages of a site, `ResolveOptions::lang`
  localises the predeclared label words, `Resolved::next_start` chains
  documents; `Resolved::view()` is the serialisable capsule the bindings
  return (`schema("resolved")`).
- The C31–C42 constructs (`12-spec-challenges.md`, one fixture each):
  tabs (`:::: tabs` / `::: tab`, `=== "Title"` sugar), `multicolumn` and
  `div` with the `tsdiv` contract, `<div markdown>`, HTML as typed,
  foreign directives (`[TOC]`, dotted `:::`), progress bars (fraction and
  `{: ` colon deprecations with fixes), emoji and icon shortcodes,
  implicit heading ids and `.unnumbered` / `.unlisted`, `^^x^^` under
  `inline.insert`, TeX logos.
- Measured on TeXSmith's side (`~/texsmith/specs/migration/status.md`):
  the `docs/` corpus (144 pages × 2 backends) renders through the tmark
  reader without an error; of the parity corpus's examples 31 / 32 build
  through LaTeX (`emoji-color` needs `lualatex`) and 24 / 26 through Typst
  (`markdown`, `math`: `mitex` 0.2.6 rejects `\begin{aligned}` and
  `\imath`, pre-existing).

Remaining for M4:

1. **Typst preview in `tmark-lsp`** (ADR 0005): nothing started; no
   `typst` dependency in the crate.
2. **The parity triage** on TeXSmith's side (`scripts/parity.py diff`
   between `--reader html` and `--reader tmark`, the allow-list of
   intended differences): the "output equivalent to the HTML-reader
   path" half of the definition of done is unmeasured.
3. **The LaTeX-math translator for Typst**: `TypstMath::Native` is
   accepted and falls back to `mitex` (`writers/src/lib.rs`); `mitex`
   0.2.6 rejects `\begin{aligned}` and `\imath`.
4. **`tmark schema inventory`** (architecture review B4): the inventory
   schema is still TeXSmith's `refs.json` as written.
5. **Node ids**: `03-ir.md` §Identity still says "dense and
   document-ordered", which the three allocation sites of `tmark-syntax`
   do not guarantee; decision X6 of the migration chose to weaken the
   doc to "unique per file". Do one or the other.
6. **Per-row spans on `table-*` diagnostics**: they are reported on the
   whole fence (the YAML reader has no positions); the LSP will want the
   row.
7. **NFC in implicit ids**: spec §Header says the slug is NFC-normalised;
   `Label::implicit` (`tmark-registry/src/collect.rs`) takes the text as
   it is, so a decomposed accent yields another id than a precomposed
   one.
8. `Requires.assets` for generated images (`Image` with `generate=` and
   no `src`): the writers emit nothing for them; decide with TeXSmith's
   assets pass.
9. The parser's performance target (`02-syntax.md`, `reviews/06`).

## Migration wave 1 (between M3 and M4, done 2026-09-11)

TeXSmith's `specs/migration/examples-migration.md` §4 items 1–6, 8 and 9,
worktree `fixes`: `lint --fix --stdout|--diff`; `[^key]` / `^[k1,k2]`
citations lowered to `Ref` with a fix (decision X7); the `///` fix routes
`/// latex` to a raw fence and `/// caption` blocks to a caption line;
attribute lists on fence info strings (C28); string authors, the
`deprecated-frontmatter-key` line-edit fix, `press.admonition_style`; the
C20 rows (`[](gls:term)`, `{index}[…]{b}`, `{index:r}`); the `--8<--`
fence body; `compat-unsupported` on the spellings M5 will implement. Item
7 (the table model, decision X9) landed in worktree `tables`: the model
of `03-ir.md` §Tables, the `table-*` parse and lint codes, the printer's
Python row shapes, the round-trip property test and TeXSmith's table
corpus. Every TeXSmith example passes `check --strict` after `lint
--fix`, `deprecated` warnings aside where the sources are not rewritten
yet.

## M5 — Bindings, compatibility, polish

- `tmark-py` wheel and `tmark-wasm`; TeXSmith depends on the wheel.
- PyMdownX compatibility profile completed (critic markup, wiki links,
  fancy list markers; smart symbols are done). Each remaining spelling
  is reported as `compat-unsupported` today (`lower/compat.rs`);
  implementing one means replacing its scan with the construct and
  updating fixture `diag-compat-unsupported`.
- Dialect import (`tmark fmt` on GFM/MyST/Pandoc admonitions and crossrefs).
- Done when: TeXSmith's Python-Markdown extensions are deleted in favour of
  the IR path for PDF, and its MkDocs/Zensical companion emits the `Mkdocs`
  profile.

Status (2026-09-11, end of the TeXSmith-migration waves): the bindings
were pulled forward and are what TeXSmith's `tmark-migration` branch
runs on; the compatibility profile and the deletion on TeXSmith's side
(phase 5 of its plan) are not started, so M5 stays **open**. What exists:

- `crates/tmark-py`: the native module `tmark._tmark` (PyO3, abi3 from
  Python 3.10, `cdylib` only, `test = false`) under the package `tmark`
  (`python/tmark`, `py.typed`, the stub `_tmark.pyi` generated by
  `scripts/gen_stubs.py` from the docstrings, checked by a pytest). API
  as built (`09-bindings.md` §Python): `parse`, `format`, `lint`,
  `fixes`, `resolve` (with `numbering`, `lang`, `book`, `start`), `write`
  (against a shared `Resolved` handle so a document's slots number
  once), `lower_web`, `edit`, `edit_many`, `schema` (`ir`,
  `frontmatter`, `diagnostic`, `resolved`), `schema_hash` (16 hex digits,
  platform-independent), `codes`, `fragments`, `registries`, `version`
  / `__version__`, the `Loader` protocol wrapped at the boundary with
  the GIL released around the Rust stages, the `Resolved` class.
- The version handshake: the IR JSON root carries `"tmark"`, `resolve`
  and `edit` refuse another version, TeXSmith records `schema_hash()`
  and regenerates its models on drift.
- `.github/workflows/wheels.yml` builds the abi3 wheels (manylinux 2_28
  x86_64 and aarch64, macOS universal2, Windows x64, sdist) on a `v*`
  tag or on demand and publishes through trusted publishing;
  `.github/workflows/ci.yml` builds the module with `maturin develop`
  and runs the pytest suite on every push. TeXSmith installs the crate
  as a path dependency (`vendor/tmark` → this checkout; `uv sync` builds
  the wheel through maturin).
- `compat-unsupported` still covers exactly three spellings: critic
  markup, wiki links and fancy list markers (`lower/compat.rs`, fixture
  `diag-compat-unsupported`); tabs, progress bars, shortcodes, `^^x^^`
  and `[TOC]` left it with the C31–C42 wave.

Remaining for M5:

1. **Critic markup, wiki links, fancy list markers**: the three
   `compat-unsupported` scans become constructs (spec appendix rows, a
   fixture each), and `OrderedList` gains its style.
2. **Dialect import** (`tmark fmt` on GFM/MyST/Pandoc spellings): not
   started. Smart symbols and straight quotes landed with the parity
   triage (finding F6, fixture `inline-smart-symbols`, challenge C45).
3. **Releases**: nothing is on crates.io or PyPI; the wheel workflow has
   run on no tag yet. The wheel does not carry the CLI as a console
   script (`09-bindings.md` says "later").
4. **`tmark-wasm`**: a three-line skeleton.
5. **`citation-shadowed-by-footnote`** is still never emitted
   (`06-registries.md`, wave 1 notes).
6. TeXSmith's phases 4 (parity gate, flip to `--reader tmark`) and 5
   (delete the extensions): the definition of done of this milestone is
   theirs.

## Out of scope for all milestones

Templates, fonts, PDF compilation, DOI resolution, executed fences, site
generator modules. They stay in TeXSmith and consume this core.
