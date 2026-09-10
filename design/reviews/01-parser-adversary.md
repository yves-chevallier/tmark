# Review 01 — Parser adversary (handoff mandate 1)

Scope: `crates/tmark-markdown` (vendored fork) and `crates/tmark-syntax`
(lowering), at commit `f3443a6`, attacked with the CommonMark edge cases
the TMark constructs interleave with. Every finding was reproduced with
`target/debug/examples/dump FILE` (IR + parse diagnostics) and, where
noted, `tmark parse|fmt|check`. Scratch inputs live under
`/tmp/claude-1000/-home-ycr-tmark/2cbd51a1-bfb7-47e4-884e-62793ff9a596/scratchpad/parser/`.
Nothing was modified under `crates/`, `spec/` or `editors/`. Findings
already listed in `reviews/02-spec-conformance.md` (D1–D34) are not
repeated.

(report in progress — skeleton written first, findings appended as verified)

## Summary

## Findings

