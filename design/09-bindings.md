# 09 — CLI, bindings and distribution

Crates: `tmark-cli`, `tmark-py`, `tmark-wasm`. These are the edges: they own
I/O and processes and contain no language logic. Each is a thin translation
of the facade (`01-architecture.md` §The facade).

## CLI

```
tmark parse  FILE [--compact]           IR as JSON
tmark fmt    FILE… [--profile P] [--check] [--write]
tmark lint   FILE… [--fix] [--strict] [--level CODE=LEVEL]
tmark check  FILE…                       parse + resolve + lint, exit code (alias of lint)
tmark write  FILE --to latex|typst|html [--media print|web] [--map]   (milestone 4)
tmark schema frontmatter|ir             print the JSON schema (`inventory`: milestone 4)
tmark-lsp                                the language server on stdio, a separate binary
```

The language server is its own binary (`crates/tmark-lsp`), so that the CLI
does not carry `lsp-server`; the VS Code extension bundles it. `tmark.toml`
above a file sets its profile and lint levels (`tmark::Config`); flags
override. `clap`, `FsLoader`, exit codes 0/1/2 (ok / findings / usage). Output on stdout, diagnostics on stderr in the
`file:line:col: severity code: message` form. `--json` everywhere for tools.

## Python (`tmark-py`)

PyO3 + maturin, native module `tmark._tmark` re-exported by the Python package `tmark`, wheel published with the CLI binary as
a console script (`tmark`) so `pip install tmark` gives both. API surface,
deliberately small and JSON-shaped (ADR 0003):

```python
tmark.parse(text: str, file: str = "<memory>") -> dict          # Document as JSON + "diagnostics"
tmark.format(text: str, profile: str = "canonical") -> str
tmark.lint(text: str, loader: Loader | None = None) -> list[dict]
tmark.write(doc: dict, backend: str, options: dict, loader: Loader | None = None) -> dict   # {"text", "map", "requires"}
tmark.edit(text: str, doc: dict, node_id: int, replacement: dict) -> str
tmark.schema(name: str) -> dict
```

`Loader` is a Python protocol with `load(from, rel) -> str | None`, wrapped
into a Rust `Loader` at the boundary. Documents cross the boundary as JSON
(`serde_json` → Python objects via `pythonize`), not as PyO3 classes
mirroring every node: TeXSmith generates its typed models from the IR schema
and validates once at the edge. This keeps the binding at a few hundred
lines and avoids a second definition of the IR.

Performance note: JSON across the boundary costs milliseconds per document;
acceptable. If a profile shows otherwise, the fix is a binary encoding
(`serde` + `msgpack`), not PyO3 classes.

## WASM (`tmark-wasm`)

`wasm-bindgen`, same six functions, `MemoryLoader` only. Used by the VS Code
web extension and a playground page. Built with `wasm-pack` into
`editors/vscode/wasm/`. Size budget: under 2 MB gzipped, which excludes the
Typst compiler from the WASM build (preview is desktop-only at milestone 4).

## Versioning

One version for the workspace and the wheel; the VS Code extension pins the
`tmark-lsp` version it bundles. The IR JSON carries `"tmark": "0.x.y"` at the
document root; consumers refuse a major mismatch.

## Distribution

- crates.io: `tmark` (facade) and the libraries; binaries via `cargo install
  tmark-cli`.
- PyPI: `tmark` wheel (manylinux, macOS, Windows) built by maturin in CI.
- VS Code Marketplace: `texsmith.tmark`, with platform-specific packages
  carrying the LSP binary.
- GitHub releases: the CLI archives.
