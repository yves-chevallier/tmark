# ADR 0005 — Typst is the preview backend, LaTeX the press backend

**Status:** accepted for milestone 4.

**Context.** Editor preview needs sub-second turnaround and click-to-source.
LaTeX compiles in seconds to minutes and needs a distribution; Typst is a
Rust library with span-level introspection (`typst-ide`).

**Decision.** `tmark-lsp` embeds the `typst` crate to render the Typst body
to SVG pages for the editor webview. Click-to-source goes Typst span → source
map → TMark span. LaTeX remains the backend for print-quality output through
TeXSmith, where SyncTeX plus the same source map gives PDF-to-source.

**Consequences.** The Typst writer and the source map are on the critical
path of milestone 4. The WASM build excludes Typst (size). Preview fidelity
is "Typst's rendering of the same IR", not "the LaTeX PDF"; the spec's P5
symmetry makes that acceptable.
