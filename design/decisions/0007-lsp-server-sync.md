# ADR 0007 — `lsp-server` (synchronous) rather than `tower-lsp`

**Status:** accepted.

**Context.** The language server's work is CPU-bound parsing over a map of
open documents. `tower-lsp` brings an async runtime; `lsp-server`
(rust-analyzer's) is a synchronous message loop.

**Decision.** `lsp-server` + `lsp-types`, one main loop, one worker thread
for resolve and lint, snapshots for requests.

**Consequences.** No async in the workspace. Simpler debugging, smaller
binary. If a feature ever needs many concurrent I/O calls, that feature is
probably TeXSmith's.
