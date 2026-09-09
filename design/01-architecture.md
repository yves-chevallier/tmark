# 01 — Architecture

## Crates

One crate per responsibility. Arrows are `Cargo` dependencies; nothing points
upward. A crate may only use the public API of the crates below it.

```
                        tmark-cli   tmark-lsp   tmark-py   tmark-wasm      (edges: I/O, processes)
                              \        |   |      /          /
                               \       |   |     /          /
                                 tmark  (facade: parse · fmt · lint · write · resolve)
                                 /     |      |        \
                        tmark-writers  |   tmark-lint   tmark-fmt
                        (commonmark,   |      |            |
                         html, latex,  |      |            |
                         typst)        |      |            |
                                 \     |      |           /
                                  tmark-registry (counters, labels, bib, glossary, index, crossrefs, Loader)
                                          |
                                     tmark-syntax  (parser: text → IR + diagnostics)
                                       /      \
                          tmark-markdown      tmark-ir     (nodes, spans, front matter types, closed registries, JSON schema)
                          (vendored micromark: tokenizer constructs, events, mdast)
```

| Crate | Responsibility | Depends on | Design doc |
| ----- | -------------- | ---------- | ---------- |
| `tmark-ir` | Node types, spans, attributes, front-matter types, the closed registries (roles, node words, predeclared prefixes, deprecations), serde and `schemars` derivations. No logic beyond constructors, `walk`, `map`. | `serde`, `schemars` | `03-ir.md` |
| `tmark-markdown` | Vendored markdown-rs (micromark architecture) with the TMark tokenizer constructs added. Events and mdast only; knows nothing of the IR. | `unicode-id` | `02-syntax.md`, ADR 0002 |
| `tmark-syntax` | Text → `Document` + `Vec<Diagnostic>`: drives `tmark-markdown` and lowers its mdast to the IR. Never fails. | `tmark-ir`, `tmark-markdown` | `02-syntax.md` |
| `tmark-registry` | Builds the registries from the front matter, the document and, through `Loader`, included files and external sources (`.bib`, `refs.json`). Resolves `@` and `#`. | `tmark-ir`, `tmark-syntax` | `06-registries.md` |
| `tmark-fmt` | Canonical printer and profiles; local edit splicing. | `tmark-ir`, `tmark-syntax` (for idempotence tests) | `04-printer.md` |
| `tmark-lint` | Rule catalogue over IR + registries; produces `Diagnostic`s. | `tmark-ir`, `tmark-registry` | `05-diagnostics.md` |
| `tmark-writers` | `Writer` trait and four implementations: `commonmark`, `html`, `latex`, `typst`. Emit a body plus a source map. | `tmark-ir`, `tmark-registry` | `07-writers.md` |
| `tmark` | Facade. The one crate downstream users depend on. Re-exports and a handful of pipeline functions. | all of the above | this document |
| `tmark-cli` | `tmark parse|fmt|lint|write|check`. Filesystem `Loader`. | `tmark` | `09-bindings.md` |
| `tmark-lsp` | Language server over stdio. Filesystem `Loader`, document store, incremental re-parse. | `tmark` | `08-lsp.md` |
| `tmark-py` | PyO3 module `tmark` for TeXSmith. | `tmark` | `09-bindings.md` |
| `tmark-wasm` | `wasm-bindgen` build for the web (VS Code web, playground). | `tmark` | `09-bindings.md` |

The four bottom crates (`ir`, `syntax`, `registry`, `fmt`) are the milestone
1 deliverable; a user can parse, format and lint with nothing else.

## Data flow

```
text ──parse──▶ Document + diagnostics(parse)
                    │
                    ├──resolve(loader)──▶ Registries + diagnostics(resolve)
                    │                          │
                    ├──lint(registries)────────┴──▶ diagnostics(lint)
                    │
                    ├──print(profile)──▶ text (normal form)
                    │
                    └──write(backend, registries, options)──▶ Body { text, source_map }
```

`Document` is immutable after parsing. Passes that transform a document
(TeXSmith's includes, executed fences, asset conversion) produce a *new*
`Document`; node ids and spans survive the transformation so the source map
still points at the author's text (`03-ir.md` §Identity).

## The facade

`tmark` exposes exactly these entry points, all pure:

```rust
pub fn parse(text: &str, file: FileId) -> Parsed;              // Document + diagnostics
pub fn resolve(doc: &Document, loader: &dyn Loader) -> Resolved; // Registries + diagnostics
pub fn lint(doc: &Document, res: &Resolved) -> Vec<Diagnostic>;
pub fn format(doc: &Document, profile: Profile) -> String;
pub fn write(doc: &Document, res: &Resolved, backend: Backend, opts: &WriterOptions) -> Body;
pub fn edit(text: &str, doc: &Document, edit: NodeEdit) -> String; // local splice
```

Anything else a binding needs is a composition of these. Bindings do not
reach into lower crates.

## Boundaries that are traits

Only three traits cross crate boundaries, and each has at least two
implementations in this repository:

| Trait | Where | Implementations |
| ----- | ----- | --------------- |
| `Loader` | `tmark-registry` | `MemoryLoader` (tests, WASM), `FsLoader` (CLI, LSP, PyO3) |
| `Writer` | `tmark-writers` | `CommonMark`, `Html`, `Latex`, `Typst` |
| `Rule` | `tmark-lint` | every lint rule |

Everything else is a plain function or a plain data type. If a fourth trait
seems necessary, write the ADR first.

## Dependencies policy

Allowed in the core: `serde`, `serde_json`, `schemars`, `unicode-*` crates,
`memchr`, a YAML parser (`serde_yaml` successor of the day), a BibTeX parser
(`biblatex`), and the vendored CommonMark machinery (ADR 0002). Edges may add
`clap`, `lsp-server`/`lsp-types`, `pyo3`, `wasm-bindgen`, `typst` (milestone
4). Nothing async in the core. No `regex` in the parser hot path: constructs
are hand-written state machines like the rest of the CommonMark core.

## Repository layout

```
spec/            language spec and conformance fixtures
design/          this documentation, ADRs
crates/          the workspace
editors/vscode/  the VS Code extension (grammar now, LSP client at milestone 3)
```

Generated artifacts live next to their consumer and carry a header naming
their generator: `editors/vscode/syntaxes/*.json` (from `tmark-ir` registries
at milestone 3; from `scripts/build_grammar.py` until then), the JSON schema of
the front matter (`crates/tmark-ir/schema/frontmatter.json`), the Python stubs
in TeXSmith.
