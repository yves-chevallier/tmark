# ADR 0004 — Spans plus local edits, not a lossless concrete syntax tree

**Status:** accepted (restates spec §Round-trip and source spans).

**Context.** Formatters and refactoring tools either keep a lossless CST
(rust-analyzer style) or an AST with source spans and reprint locally
(Pandoc style with positions).

**Decision.** AST with a span on every node, a canonical printer for whole
documents, and `edit(text, doc, id, node)` that reprints one node and
splices it into its span. The front matter is copied byte for byte.

**Consequences.** `tmark fmt` normalises sugar, fence lengths, marker
characters, attribute order, caption position, redundant whitespace; it
preserves soft breaks, comments, raw passthroughs, escapes and unrecognised
text. Tools that change one node touch nothing else. A CST is not needed and
would double the size of `tmark-syntax`.
