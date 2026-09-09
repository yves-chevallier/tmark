# ADR 0001 — The core is pure

**Status:** accepted.

**Context.** TeXSmith mixes parsing, template rendering, asset conversion,
network access and build orchestration in one Python package. That makes the
language impossible to embed in an editor, hard to test, and tied to a TeX
distribution.

**Decision.** Every crate under `crates/` except the edges (`tmark-cli`,
`tmark-lsp`, `tmark-py`, `tmark-wasm`) is pure: no I/O, no processes, no
clock, no network, no global state. The single seam is the `Loader` trait,
which returns text for a path and nothing else.

**Consequences.** Features needing the world (DOI lookup, executed fences,
image conversion, templates, PDF builds) live in TeXSmith and operate on the
IR through the bindings. The core runs in WASM unchanged. Tests need no
fixtures on disk beyond `MemoryLoader`.
