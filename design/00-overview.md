# 00 — Overview

## What TMark is

TMark is the Markdown dialect specified in `../spec/tmark.md`. The core
toolchain in this repository gives that language a parser, an intermediate
representation (IR), a canonical printer and formatter, a linter, registries
for references, writers to CommonMark, HTML, LaTeX and Typst *bodies*, a
language server, and bindings for Python and the web.

The language is **CommonMark plus four syntactic families** (attributes,
roles, container directives, data directives) **plus two sigils** (`#`
defines, `@` refers), on top of the Python-Markdown / PyMdownX conventions.
The IR, not the syntax, is the definition of the language (spec §The IR is the
definition). Everything here follows from that sentence.

## Goals

1. **One parser, one IR, one printer.** Parse any accepted spelling into the
   same IR; print the IR back as the one canonical spelling; `parse(print(ir))
   == ir` modulo spans.
2. **Editor-grade.** Spans on every node, diagnostics with positions,
   sub-100 ms re-parse of a chapter-sized file, a language server that gives
   completion, lint, formatting, navigation and outline.
3. **Backend bodies, not documents.** Writers emit the body a template wraps:
   a LaTeX fragment between `\begin{document}` and `\end{document}`, a Typst
   fragment, an HTML fragment, a CommonMark document in a chosen profile.
   Preambles, fonts, packages, templates belong to TeXSmith.
4. **Source maps.** Every writer records which output range came from which
   node, so a PDF viewer or a browser can jump back to the `.md` line.
5. **Embeddable.** A Rust library, a CLI, a Python wheel (PyO3), a WASM build.
   No runtime dependency on Python, Node or a TeX distribution in the core.

## Non-goals

- Templates, preambles, fonts, packages, build orchestration, PDF
  compilation, site generators, plugins that run code, network access. All of
  that is TeXSmith.
- User-defined syntax. Roles, node words and counter prefixes are closed
  registries with declared entries (spec P4, P6, §Feature registry).
- A lossless concrete syntax tree. Spans plus local edits (ADR 0004).
- HTML *input*. Importing a site is TeXSmith's HTML reader; this core only
  reads TMark text.
- Rendering Markdown for the web beyond an HTML fragment writer. Site
  generators keep their own pipeline; `tmark fmt --profile mkdocs` gives them
  Markdown they understand.

## Principles (see `../AGENTS.md`)

Pure core. Spec is the source of truth. One definition per fact. KISS and
YAGNI. SOLID where two concrete users exist. DRY across languages.

## The product boundary

```
              text (.md / .tm)                          site generators
                    │                                        ▲
  ┌─────────────────▼──────────────────┐   CommonMark        │
  │  tmark  (this repo, Rust, pure)    │──────────────────────┘
  │  parse · IR · fmt · lint ·         │
  │  registries · writers · lsp        │   IR (JSON / PyO3)
  └─────────────────┬──────────────────┘──────────────┐
                    │ LaTeX / Typst / HTML body        │
  ┌─────────────────▼──────────────────────────────────▼──────┐
  │  texsmith  (Python)                                       │
  │  IR passes with I/O (includes, exec, assets, DOI, cross-  │
  │  document inventories) · templates · fragments · fonts ·  │
  │  LaTeX and Typst builds · MkDocs / Zensical adapters      │
  └───────────────────────────────────────────────────────────┘
```

Two rules keep the boundary honest:

- **Purity test.** If a function needs a file, a clock, a process or a
  socket, it is either behind the `Loader` trait (`06-registries.md`) or it is
  in TeXSmith.
- **Body test.** If a writer needs to know the template, the font, the
  package list or the page geometry, the concern is TeXSmith's. A writer
  only needs the IR, the resolved registries and the writer options declared
  in `07-writers.md`.

## Vocabulary

| Term | Meaning |
| ---- | ------- |
| Construct | A syntactic form the parser recognises (a role, an attribute list, a fence…). |
| Node | An IR value. Node names are those of the spec's node catalogue. |
| Span | Byte range in a source file, plus the file id. Every node has one. |
| Canonical | The one spelling the printer emits for a node. |
| Sugar | Any other accepted spelling. Normalises to the canonical IR. |
| Profile | A printer configuration selecting which sugar and which X-class constructs are emitted (`canonical`, `strict`, `mkdocs`). |
| Registry | A named table the reference system resolves against (counters, bibliography, glossary, index, cross-document). |
| Loader | The only I/O seam: a trait that returns the text of a path. |
| Body | A writer's output: the part of a document a template wraps. |
| Source map | Output range → node id, emitted with every body. |
