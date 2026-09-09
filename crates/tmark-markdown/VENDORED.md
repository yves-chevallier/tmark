# Vendored markdown-rs

Upstream: https://github.com/wooorm/markdown-rs, tag `1.0.0`, commit
`1506572`, MIT (see `LICENSE-markdown-rs`). Vendored per ADR 0002.

What was dropped: `benches/`, `fuzz/`, `generate/`, `mdast_util_to_markdown/`,
the MDX tests that need `swc` (`tests/mdx_*.rs`, `tests/serde.rs`,
`tests/test_utils/`). The MDX constructs stay in the source, off by default,
so that upstream merges stay mechanical.

What is changed: listed below, one line per file, kept current by whoever
edits the fork.

- `Cargo.toml`: package renamed `tmark-markdown`, lib `tmark_markdown`.
- `tests/*.rs`: crate path `markdown::` → `tmark_markdown::`.
- `Cargo.toml`: `[lints.clippy]` allows for lints newer than upstream's toolchain.
- `src/lib.rs`: `#![deny(clippy::pedantic)]` removed (newer clippy versions keep adding pedantic lints).
- `src/configuration.rs`: `Constructs` gains the `tmark_*` flags, `Constructs::tmark()` and `ParseOptions::tmark()`; the `Debug` expectations in its unit test list the new flags.
- `src/event.rs`, `src/state.rs`, `src/tokenizer.rs` (`LabelKind`), `src/construct/mod.rs`, `src/util/mod.rs`: registrations for the TMark constructs.
- `src/construct/text.rs`, `src/construct/flow.rs`: dispatch to the TMark constructs (all `Nok` when their flag is off).
- `src/construct/attention.rs`: `=`, `^`, `+` and single `~` sequences under `tmark_attention`.
- `src/construct/label_end.rs`: `TmarkGroup` and `TmarkSpan` label kinds.
- `src/mdast.rs`, `src/to_mdast.rs`: the `Tmark*` nodes and their builders.
- New files: `src/construct/tmark_*.rs`, `src/construct/partial_tmark_body.rs`, `src/util/tmark.rs`.
- `Cargo.toml`: `doctest = false` (the doc examples import `markdown::`).
- `src/construct/raw_flow.rs`: an attribute list after a closing math fence (`$$ {#eq:x}`) is accepted under `tmark_brace` and recorded as the fence meta.
- `src/lib.rs`: `pub use util::tmark;` re-exports the TMark predicates for `tmark-syntax`.
- `src/to_mdast.rs`: the meta of a closing math fence is attached to the node below the content buffer.
