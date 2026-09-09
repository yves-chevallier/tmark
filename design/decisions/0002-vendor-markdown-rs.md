# ADR 0002 — Vendor the micromark architecture (markdown-rs) rather than depend on or rewrite a CommonMark parser

**Status:** accepted; revisit if upstream gains public construct extension.

**Context.** TMark is CommonMark plus constructs that interleave with inline
tokenisation (roles containing emphasis, attribute lists after links). Three
options: (a) depend on a CommonMark crate and post-process its AST; (b) write
a CommonMark parser; (c) vendor `markdown-rs` (MIT, port of micromark, one
module per construct, positions on every event, GFM/math/front matter
already present) and add constructs inside.

**Decision.** (c). The vendored tree lives in `crates/tmark-syntax/src/md/`
with `VENDORED.md` recording the upstream commit and the list of changed
files.

**Consequences.** CommonMark conformance and its test suite come for free.
Adding a construct is a state machine plus a `Name` variant; most TMark
constructs are handled in lowering without touching the tokenizer. Upstream
merges are manual and rare. Option (a) is rejected because post-hoc
reconstruction of role content is fragile; (b) because it re-derives 6 000
lines the vendored code already provides.
