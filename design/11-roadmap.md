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

Status: `tmark-ir` done (schema committed); `tmark-markdown` vendored with
the TMark constructs, CommonMark suite green; `tmark-syntax` lowers every
construct of `02-syntax.md` except the deferred ones listed in its
implementation notes, with totality property tests; the facade exposes
`parse` and `schema`; the conformance runner
(`crates/tmark/tests/conformance.rs`) checks 31 fixtures (`input` and
`canonical` blocks against the `ir` block, diagnostics on the first input);
`tmark parse`, `tmark fmt` and `tmark schema` work; `tmark-fmt` prints
every node, the canonical block of every fixture is a fixed point, the
editor sample and the whole spec round-trip and format idempotently, and
`edit` splices one node. `tmark fmt` on `spec/tmark.md` is idempotent; the spec itself is not in
normal form (it uses caption-before and bracketed references), and it must
stay that way for now: TeXSmith 0.6 does not implement the caption line
after a table, so the normal form does not yet build "unchanged in
meaning" (checked by converting both versions with `texsmith`: the after-
captions render as text). Closing M1 therefore waits on TeXSmith's caption
support (or on the `mkdocs` profile emitting the shipping spelling). The
parser's performance target is also open (see `02-syntax.md`).

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

Crates: `tmark-writers` (LaTeX, Typst, HTML, CommonMark profiles), the
`typst` crate in `tmark-lsp`.

- Bodies, `Requires`, source maps per `07-writers.md`: the three
  writers, `tmark write` and the fixture snapshots exist (`07-writers.md`
  §Implementation notes (milestone 4)); `Mkdocs` profile, `edit_many` and
  the `FRAGMENTS` and `KEY_LABELS` registries landed ahead of the writers
  in wave 1 of the TeXSmith migration.
- TeXSmith switches its reader to the IR (its side of the work; the
  contract is this repository's schema and `Requires`).
- Typst preview in the editor with click-to-source.
- Done when: the TeXSmith documentation builds to PDF through LaTeX from
  bodies produced here with output equivalent to the HTML-reader path, and
  the preview updates in under a second on a chapter-sized file.

## Migration wave 1 (between M3 and M4, done 2026-09-11)

TeXSmith's `specs/migration/examples-migration.md` §4 items 1–6, 8 and 9,
worktree `fixes`: `lint --fix --stdout|--diff`; `[^key]` / `^[k1,k2]`
citations lowered to `Ref` with a fix (decision X7); the `///` fix routes
`/// latex` to a raw fence and `/// caption` blocks to a caption line;
attribute lists on fence info strings (C28); string authors, the
`deprecated-frontmatter-key` line-edit fix, `press.admonition_style`; the
C20 rows (`[](gls:term)`, `{index}[…]{b}`, `{index:r}`); the `--8<--`
fence body; `compat-unsupported` on the spellings below until M5
implements them. Every TeXSmith example except `tables` (X9, worktree
`tables`) and the ones using M5 constructs passes `check --strict` after
`lint --fix`.

## M5 — Bindings, compatibility, polish

- `tmark-py` wheel and `tmark-wasm`; TeXSmith depends on the wheel.
- PyMdownX compatibility profile completed (critic markup, progress bars,
  wiki links, smart symbols as the spec lists them). Each spelling is
  reported as `compat-unsupported` today (`lower/compat.rs`); implementing
  one means replacing its scan with the construct and updating fixture
  `diag-compat-unsupported`.
- Dialect import (`tmark fmt` on GFM/MyST/Pandoc admonitions and crossrefs).
- Done when: TeXSmith's Python-Markdown extensions are deleted in favour of
  the IR path for PDF, and its MkDocs/Zensical companion emits the `Mkdocs`
  profile.

Status (2026-09-11): `tmark-py` pulled forward for the TeXSmith migration.
The native module `tmark._tmark` and the package `tmark` exist with
`parse`, `format`, `lint`, `fixes`, `resolve`, `edit`, `schema`,
`schema_hash`, `codes`, `fragments`, `registries` (design 09 §Python, the
API as built); the Python `Loader` protocol is wrapped at the boundary;
`maturin develop` and `maturin build` work, the pytest suite under
`crates/tmark-py/tests` is green, `.github/workflows/wheels.yml` builds the
abi3 wheels; `write` renders through `tmark::write` against a shared
`Resolved` handle and `fragments()` serves the `FRAGMENTS` table. Not
started: the wheel on PyPI, `tmark-wasm`, the PyMdownX profile, dialect
import.

## Out of scope for all milestones

Templates, fonts, PDF compilation, DOI resolution, executed fences, site
generator modules. They stay in TeXSmith and consume this core.
