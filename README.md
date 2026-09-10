# TMark

TMark is a Markdown dialect for technical and academic writing: CommonMark
structure, the Python-Markdown and PyMdownX conventions MkDocs users know, and
a small set of constructs that close the gaps prose tooling has never closed
(references, citations, numbering, captions, rich tables, controlled escape
hatches to LaTeX, Typst and HTML). The language is defined by its
intermediate representation, not by any concrete syntax; every construct has
one canonical spelling and zero or more sugar spellings that normalise to it.

This repository is the home of the **language and its core toolchain**:

| Path | What it is |
| ---- | ---------- |
| `spec/tmark.md` | The language specification (draft 3). Normative. |
| `spec/conformance/` | Conformance fixtures: one file per construct, sugar → canonical → IR. |
| `design/` | Implementation specification: architecture, crate boundaries, parser, IR, printer, diagnostics, registries, writers, language server, bindings, testing, roadmap. |
| `design/decisions/` | Architecture decision records. Settled questions; do not reopen without a new ADR. |
| `crates/` | The Rust workspace, one crate per responsibility (skeleton; see `design/01-architecture.md`). |
| `editors/vscode/` | The VS Code extension: TextMate grammar today, language-server client tomorrow. |

The core is **pure**: text in, tree or text out, no I/O, no network, no
external tool, no template. Everything that touches the world (includes,
executed fences, image conversion, DOI lookups, templates, PDF builds, site
generators) lives in [TeXSmith](https://github.com/yves-chevallier/texsmith),
which consumes this core through its Python bindings.

## Reading order

1. `design/00-overview.md` — goals, non-goals, principles, the product boundary.
2. `spec/tmark.md` — the language. Read §Document model and §Lexical grammar first, then the node catalogue.
3. `design/01-architecture.md` — crates, dependency graph, data flow.
4. The design document of the part you implement, then its ADRs.
5. `design/11-roadmap.md` — what to build first and what "done" means at each milestone.

`AGENTS.md` states the working rules for anyone, human or agent, implementing
from this repository.

## Status

Milestones 1 and 2 of `design/11-roadmap.md` are implemented: the IR, the
vendored tokenizer with the TMark constructs, the lowering, the canonical
printer, the registries and resolution, the lint catalogue, a conformance
runner over `spec/conformance/`, and a CLI (`parse`, `fmt`, `check`, `lint`,
`schema`). Milestone 1 formally waits on TeXSmith's support of the caption
line after a table. The VS Code
extension ships a working TextMate grammar and a grammar test harness
(`editors/vscode/README.md`).

```sh
cargo run -p tmark-cli -- parse spec/conformance/role-aside.md   # IR as JSON
cargo run -p tmark-cli -- check spec/tmark.md                      # parse, resolve, lint
cargo run -p tmark-cli -- fmt --check spec/tmark.md                # normal form?
cargo test --workspace                                            # incl. the CommonMark suite and the fixtures
```
