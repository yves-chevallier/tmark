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
`edit` splices one node. Remaining for M1: the fixed-point check of
`tmark fmt --check` on `spec/tmark.md` itself (the spec is not written in
normal form yet; see `04-printer.md`), and the parser's performance target.

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

## M3 — Language server and editor

Crates: `tmark-lsp`; `editors/vscode` gains the client; the grammar
generator moves to `tmark-ir`.

- Features of `08-lsp.md` up to code actions and document links.
- Front-matter completion from the schema, merged with an external `press`
  schema when configured.
- Done when: the extension, installed from a `.vsix`, gives completion on
  `@` and `{`, formats on save, shows unresolved references, and renames a
  label across a two-file document with an include.

## M4 — Writers and preview

Crates: `tmark-writers` (LaTeX, Typst, HTML, CommonMark profiles), the
`typst` crate in `tmark-lsp`.

- Bodies, `Requires`, source maps per `07-writers.md`; `Mkdocs` profile.
- TeXSmith switches its reader to the IR (its side of the work; the
  contract is this repository's schema and `Requires`).
- Typst preview in the editor with click-to-source.
- Done when: the TeXSmith documentation builds to PDF through LaTeX from
  bodies produced here with output equivalent to the HTML-reader path, and
  the preview updates in under a second on a chapter-sized file.

## M5 — Bindings, compatibility, polish

- `tmark-py` wheel and `tmark-wasm`; TeXSmith depends on the wheel.
- PyMdownX compatibility profile completed (critic markup, progress bars,
  wiki links, smart symbols as the spec lists them).
- Dialect import (`tmark fmt` on GFM/MyST/Pandoc admonitions and crossrefs).
- Done when: TeXSmith's Python-Markdown extensions are deleted in favour of
  the IR path for PDF, and its MkDocs/Zensical companion emits the `Mkdocs`
  profile.

## Out of scope for all milestones

Templates, fonts, PDF compilation, DOI resolution, executed fences, site
generator modules. They stay in TeXSmith and consume this core.
