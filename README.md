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
| `design/13-handoff.md` | Handoff notes, newest first: state of the branch, what each round did, the contract with TeXSmith, review mandates, pitfalls. Start here when taking over. |
| `crates/` | The Rust workspace, one crate per responsibility (`design/01-architecture.md`): IR, vendored tokenizer, lowering, printer, registries, lint, writers, facade, CLI, language server, Python bindings. |
| `editors/vscode/` | The VS Code extension: TextMate grammar and the client of `tmark-lsp`. |

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

Milestones 1 and 2 of `design/11-roadmap.md` are done: the IR, the vendored
tokenizer with the TMark constructs, the lowering, the canonical printer,
the registries and resolution, the lint catalogue, a conformance runner
over `spec/conformance/` (87 fixtures), and the CLI. Milestone 3 (the
language server and the VS Code extension) is implemented and waits for a
person to try the `.vsix`. Milestones 4 and 5 were largely pulled forward
by the TeXSmith migration (branch `texsmith-migration`, not yet merged to
`main`): the LaTeX, Typst and HTML writers with fixture snapshots, the
`yaml table` model, the `mkdocs` printing profile and the web lowering of
a MkDocs page, site-wide resolution, the C31–C42 constructs of the
migration audit, and the Python bindings TeXSmith now runs on. Still open:
the Typst preview in the editor, the parity triage against TeXSmith's
legacy output, critic markup, wiki links and fancy list markers
(`compat-unsupported` today), and the crates.io / PyPI releases. The
current state, milestone by milestone, is `design/11-roadmap.md`; the
notes for whoever continues are `design/13-handoff.md`.

```sh
cargo run -p tmark-cli -- parse spec/conformance/role-aside.md      # IR as JSON
cargo run -p tmark-cli -- check spec/tmark.md                         # parse, resolve, lint
cargo run -p tmark-cli -- lint --fix --diff FILE                      # the safe fixes as a diff (--stdout: the text; neither: in place)
cargo run -p tmark-cli -- fmt --check spec/tmark.md                   # normal form?
cargo run -p tmark-cli -- fmt --profile mkdocs FILE                   # the spellings a Material site renders
cargo run -p tmark-cli -- write FILE --to latex                       # the body for a backend (typst, html; --map for Body as JSON)
cargo run -p tmark-cli -- lower FILE --to web                         # a MkDocs page with its TMark constructs spliced
cargo run -p tmark-cli -- schema ir                                   # also frontmatter, diagnostic, resolved
cargo test --workspace                                               # incl. the CommonMark suite, the fixtures and the writer snapshots
pip install maturin && maturin develop -m crates/tmark-py/Cargo.toml  # the Python package `tmark`, published as `tmark-core` (or: pip install -e crates/tmark-py)
```
