# 06 — Performance investigation (handoff mandate 6)

Run by the implementing agent, not by a separate reviewer (the reviewer
agents were killed by usage limits). No `perf` or `cargo flamegraph` on the
machine; measurements are phase timings with `std::time::Instant`, release
profile, minimum of 5–10 runs, other builds running concurrently (numbers
are upper bounds). Scratch projects: `scratchpad/perf/once` (phases through
the workspace crates) and `scratchpad/perf/upstream` (crates.io `markdown`
1.0.0, `to_mdast` with GFM options, on the same files).

## Measurements

| File | Size | Phase | Min | ms/MB |
| ---- | ---- | ----- | --- | ----- |
| `design/03-ir.md` | 11.6 KB | tokenizer + mdast (fork, TMark options) | 1.8 ms | 155 |
| | | `tmark_syntax::parse` (tokenizer + lowering) | 1.8 ms | 155 |
| | | `tmark::analyse` (resolve + lint, no includes) | 0.1 ms | |
| | | `tmark::format` | 0.1 ms | |
| `spec/tmark.md` | 75 KB | tokenizer + mdast (fork) | 11.1 ms | 147 |
| | | `parse` | 12.6 ms | 167 |
| | | `analyse` | 0.8 ms | |
| | | `format` | 0.8 ms | |
| | | upstream `markdown` 1.0.0 `to_mdast` GFM | 13 ms | 172 |
| | | upstream `to_html` GFM | 13 ms | 172 |
| spec ×14 | 1.06 MB | tokenizer + mdast (fork) | 519 ms | 491 |
| | | `parse` | 553 ms | 523 |
| | | `analyse` | 12 ms | |
| | | `format` | 11 ms | |
| | | upstream `to_mdast` GFM | 498 ms | 471 |
| | | upstream `to_html` GFM | 502 ms | 475 |

The `bench` example (`cargo run --release -p tmark-syntax --example bench`)
agrees: 507–551 ms for the 1 MB file, GFM and TMark options alike.

## Findings

1. **No fork overhead.** The vendored crate with the TMark constructs on
   costs the same as crates.io `markdown` 1.0.0 with GFM options on the
   same bytes (519 vs 498 ms on 1 MB, 11 vs 13 ms on 75 KB). The TMark
   constructs and the lowering add about 6 % (`parse` minus tokenizer).
2. **The tokenizer is superlinear.** 14× the bytes cost 47× the time
   (11.1 → 519 ms), in upstream as in the fork. Per-megabyte figures
   therefore depend on the file: ~160 ms/MB for chapter-sized files,
   ~500 ms/MB at 1 MB. The "700 ms/MB" of `02-syntax.md` was measured on
   the 1 MB synthetic file and overstates the cost of real documents by
   three to four times. `to_html` shows the same curve, so the position
   bookkeeping of `to_mdast` is not the cause; the cost is in the
   tokenizer or its resolvers (the `subtokenize`/`resolve` passes walk the
   whole event list; a quadratic term there is the likely suspect, not
   verified without a profiler).
3. **Release profile.** The workspace has no `[profile.*]` section, so the
   default release profile (opt-level 3, no debug assertions, no overflow
   checks) applies to the vendored crate. No logging on the hot path.
4. **Resolve, lint and format are negligible**: about 2 % of a parse.

## Recommendation for M3

(a) Whole-file re-parse on the main loop is fine: a 100 KB chapter costs
about 15–20 ms per keystroke, well under the 150 ms debounce of the
analysis; a 1 MB file costs 0.5 s per keystroke, which lags the server's
answers but never blocks the editor (separate process). Keep the design.
Revisit with (b) parse on the worker with cancellation only if users edit
files above ~300 KB, and (c) profile the tokenizer's resolvers upstream
before touching the vendored crate; a fix there is upstreamable.

## The criterion benchmark of `10-testing.md`

It does not exist. To add it: `criterion` as a dev-dependency of
`tmark-syntax`, `benches/parse.rs` with two inputs (`spec/tmark.md` and
the conformance corpus concatenated to 1 MB), `cargo bench -p tmark-syntax`,
and a CI step comparing against the committed baseline
(`--save-baseline`/`--baseline`, fail over 20 %). Report both sizes, since
the curve is not linear.
