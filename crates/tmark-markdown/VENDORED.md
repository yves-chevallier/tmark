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
