# 09 — CLI, bindings and distribution

Crates: `tmark-cli`, `tmark-py`, `tmark-wasm`. These are the edges: they own
I/O and processes and contain no language logic. Each is a thin translation
of the facade (`01-architecture.md` §The facade).

## CLI

```
tmark parse  FILE [--compact]           IR as JSON
tmark fmt    FILE… [--profile P] [--check] [--write]
tmark lint   FILE… [--fix [--stdout | --diff]] [--strict] [--level CODE=LEVEL]
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

`lint --fix` applies the safe fixes (the `Fix` a diagnostic carries, design
05 §Fixes) and, by default, **rewrites each file in place**, reporting
`file: N fix(es) applied` on stderr; the diagnostics that were fixed are
not printed. `--stdout` prints the fixed text on stdout instead and writes
nothing (one file at a time is the useful case); `--diff` prints a unified
diff (`diff -u` layout, three lines of context) and writes nothing. The
two are exclusive; both require `--fix`. Standard input (`-`) is never
written back.

## Python (`tmark-py`)

PyO3 (abi3, Python ≥ 3.10) + maturin. The native module `tmark._tmark`
is re-exported by the Python package `tmark` (`crates/tmark-py/python/tmark`,
with `py.typed` and a generated stub `_tmark.pyi`); `pyproject.toml` sits
at the crate. The wheel later gains the CLI binary as a console script
(`tmark`) so `pip install tmark` gives both. API surface as built,
deliberately small and JSON-shaped (ADR 0003):

```python
tmark.parse(text: str, file: str = "<memory>", file_id: int = 0, profile: str = "default") -> dict
    # Document JSON with "tmark": "<version>" first and "diagnostics" last
tmark.format(text: str, profile: str = "canonical") -> str
tmark.lint(text: str, file: str = "<memory>", loader: Loader | None = None, options: dict | None = None) -> list[dict]
    # parse + resolve + lint with fixes attached (`tmark check`)
tmark.fixes(text: str, file: str = "<memory>", loader: Loader | None = None, options: dict | None = None) -> str
    # what `tmark lint --fix` writes
tmark.resolve(doc: dict, loader: Loader | None = None, options: dict | None = None, text: str | None = None) -> dict
    # schema("resolved"): numbering, lang, counters (numbers, next), next_start, labels, book,
    # refs, bibliography, entries, dois, glossary, index, crossrefs, included, diagnostics
tmark.edit(text: str, doc: dict, node_id: int, replacement: dict) -> str
tmark.edit_many(text: str, doc: dict, edits: list[dict]) -> str   # [{"node_id", "replacement"}], disjoint spans or ValueError
tmark.write(doc: dict, backend: str, options: dict | None = None, loader: Loader | None = None,
            resolved: Resolved | dict | None = None, resolve_options: dict | None = None) -> dict
    # {"text", "map", "requires"}; backend html | latex | typst
class tmark.Resolved                       # opaque handle in resolve()["handle"]; .view() -> dict, __repr__
tmark.schema(name: str) -> dict            # "ir", "frontmatter", "diagnostic", "resolved"
tmark.schema_hash() -> str                 # 16 hex digits, FNV-1a of schema("ir"), platform-independent
tmark.codes() -> list[dict]                # {"id", "severity", "stage", "doc"} per diagnostic code
tmark.fragments() -> list[dict]            # FRAGMENTS rows: name, provides, packages, shell_escape, description
tmark.registries() -> dict                 # roles, node_words, lang_default_node_words, prefixes,
                                           # admonitions, features, deprecations, key_labels
tmark.version() -> str; tmark.__version__  # the workspace version
```

`options` is one dict for `lint`, `fixes` and `resolve`: `path` (the
document's path, relative to which includes and sources load; `file` when
it is a real path), `bibliography` (`.bib` paths relative to the
document's directory), `start` (prefix → first value; the previous
document's `next_start`), `numbering` (`backend`, the default, or `all`:
design 06 §Site-wide resolution), `lang` (label words: `fr`, `de-CH`),
`book` (the sibling documents' labels: the `book` lists of their own
`resolve` results, locations relativised by the caller), `profile`,
`levels` (code → `off | hint | info | warning | error`; lint rules only,
parse and resolve diagnostics are facts). An unknown key is a
`TypeError`, an unknown code or level a `ValueError`.

`write` is `tmark::write` one to one. `options` maps onto `WriterOptions`,
every key optional: `media` (`print` | `web`), `lang`, `code` {`engine`
(`pygments` | `minted` | `listings` | `verbatim`), `inline_plain`,
`inline_breaks`}, `latex` {`legacy_accents`}, `headings` {`base_level`,
`numbered`}, `refs` {`textual_print`, `textual_web`}, `numbering` (prefix →
`backend` | `tmark`), `typst` {`math` (`mitex` | `native`)}, `source_map`.
Keys are checked against the default options at every level (an unknown
key is a `TypeError` naming its path, `numbering` keys being free) and
the values decoded by serde (a bad enum value is a `ValueError`), so the
accepted set follows the Rust struct without a second table. The result
is the `Body` as JSON: `text`, `map` as `[[start, end, node_id], …]`
over the output bytes (empty unless `source_map`), `requires` with
`packages`, `fragments` (names of the `FRAGMENTS` table), `shell_escape`,
`assets`, `bibliography`, `citations`, `acronyms`, `index`, `counters`.

**The resolution handle.** `Resolved` is not rebuilt from its JSON view,
so `resolve()` returns, next to the view, `"handle"`: an opaque
`tmark.Resolved` (a frozen PyO3 class holding the Rust value; `repr` gives
its label/ref/diagnostic counts, `.view()` the JSON again). `write` takes
`resolved=` as either that handle or the whole `resolve()` dict; TeXSmith
calls `resolve` once per document and `write` once per slot body with the
same handle, so numbering never restarts per slot (the handle is shared,
`write` only reads it). With `resolved=None`, `write` resolves the
document itself through `loader` and `resolve_options` (the `options` of
`resolve`, `start` included) before writing.

`Loader` is a Python protocol (`tmark.Loader`, runtime-checkable) with
`load(from_path: str, rel: str) -> str | None`, wrapped into a Rust
`Loader` at the boundary; `None` is `FsLoader` (the crate has the `fs`
feature on). The Rust stages run with the GIL released (`allow_threads`)
and the wrapper takes it back for each `load` call only; the first Python
exception a loader raises is kept and re-raised once the stage returns.
`from_path` is the including file (or the `{include base=…}` directory)
and `rel` the path as written, so a loader that records what it served
knows every file of a build.

Diagnostics cross as the JSON of `tmark_ir::Diagnostic` (`code` as its
kebab-case id, `span` as `[file, start, end]`, `fix` and `related` when
present) plus `stage` and, for the main file, `path`, `line` and `col`
(1-based, byte column, `LineIndex::line_col`; `None` for another file or
when `resolve` was not given the `text`). Decision X10 of the migration:
same numbers as `tmark-cli`.

Documents cross the boundary as JSON (`serde_json` → Python objects via
`pythonize`), not as PyO3 classes mirroring every node: TeXSmith generates
its typed models from `schema("ir")`, records `schema_hash()` and refuses
to import on drift. The `"tmark"` root key is checked on the way back
(`resolve`, `edit`): a document from another major version (another minor
before 1.0) is a `ValueError` naming both versions. This keeps the binding
at a few hundred lines and avoids a second definition of the IR.

Performance note: JSON across the boundary costs milliseconds per document;
acceptable. If a profile shows otherwise, the fix is a binary encoding
(`serde` + `msgpack`), not PyO3 classes.

### Stubs

The first line of every `#[pyfunction]` docstring in `src/lib.rs` is its
Python signature; `scripts/gen_stubs.py` reads them from the built module
and writes `python/tmark/_tmark.pyi`. One definition (AGENTS.md, SSOT);
`gen_stubs.py --check` is a pytest and a CI step.

### Development install

```sh
uv venv && source .venv/bin/activate
uv pip install maturin pytest
maturin develop -m crates/tmark-py/Cargo.toml     # or: uv pip install -e crates/tmark-py
pytest crates/tmark-py/tests
python crates/tmark-py/scripts/gen_stubs.py       # after changing a signature or a docstring
```

`cargo test --workspace` never needs an interpreter: the crate is
`cdylib`-only with `test = false` (an extension module has no Python to
link a test binary against), the logic it wraps is tested in the facade,
and pyo3's `abi3-py310` falls back to the abi3 configuration when no
`python3` is on the path.

### Building wheels

`.github/workflows/wheels.yml` (PyO3/maturin-action, on a `v*` tag or on
demand): manylinux 2_28 x86_64 and aarch64, macOS `universal2`, Windows
x64, plus the sdist; one abi3 wheel per platform serves every Python from
3.10. The Linux x86_64 wheel is tested with pytest on 3.10 and 3.13 and
the stub check runs; a tag publishes to PyPI through trusted publishing
(`environment: pypi`). Locally: `maturin build --release -m
crates/tmark-py/Cargo.toml -o dist`.

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
