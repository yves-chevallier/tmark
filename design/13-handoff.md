# 13 — Handoff notes (milestone 3, first pass)

Written by the agent that implemented most of M3 on top of M1–M2, for the
agent that takes over. Read `AGENTS.md`, then this file, then
`11-roadmap.md`, then `design/reviews/`. Everything below is opinion from
the inside of the work: verify it, do not trust it.

## State of the repository

- `main`, workspace green: `cargo test --workspace`, `cargo clippy
  --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`,
  `npm test` in `editors/vscode`. Not pushed; the owner pushes.
- New since M2: `tmark-lsp` (server library plus binary, `crates/tmark-lsp`),
  the VS Code client (`editors/vscode/src/extension.js`, esbuild bundle,
  `bin/tmark-lsp` bundled by `npm run bundle:server`, `.vsix` verified
  with `vsce`), `tmark::Config` (`tmark.toml`), `tmark::parse_with`,
  `tmark::analyse`, `tmark::fixes`, `tmark lint --fix`, sub-spans in the IR
  (`SubSpan`: `RefItem.key_span`, `Attrs.id_span`, `CounterItem.key_span`),
  `tmark_ir::structural_json` (the one fixture normaliser), `find` and
  `nodes_at`, `plain_text` in `tmark-ir`, `Severity::as_str/parse`, the
  `fs` feature as a real opt-in, the grammar tables exported from
  `tmark-ir` (`examples/registries.rs` → `scripts/registries.json`).
- Reviews under `design/reviews/`: `02-spec-conformance.md`,
  `04-ir-review.md`, `05-architecture.md` were run and acted on (see
  below). **Mandates 1 (parser adversary), 3 (printer critic) and 6
  (performance) were not run**: the reviewer agents died on usage limits
  twice. Run them before M4 touches `tmark-fmt` and `tmark-markdown`; the
  prompts are in the previous version of this file (`git show
  f3443a6:design/13-handoff.md`).

## What the reviews said and what was done

From `04-ir-review.md` (all five changes landed): C1–C3 sub-spans, C4
`structural_json` + `SUGAR_FIELDS`, C5 `LineIndex` fixes (snap inside a
multibyte character, clamp before the line break; `line_end` added). Also
done from its side findings: each `AbbrDef` owns its line; `print_node`
exposed by `tmark-fmt`. Not done: node ids are not pre-order/dense
(`03-ir.md` §Identity overclaims; either fix the three allocation sites in
`tmark-syntax` or weaken the doc); `#id` on fence info strings is dropped
(spec question, `reviews/02` D2); `edit()` is single-shot and returns a
`String`.

From `05-architecture.md`: A1 (feature gate), A2 (documented in `01`), A5
(dependency comments), A6 (severity words), C3 (`analyse`, `parse_with`),
A4 (grammar tables from `tmark-ir`) done. Not done: A3 (the caption-kind ↔
prefix mapping is now `CaptionKind::prefix()` in `tmark-ir` but
`tmark-lint::caption_id`, `collect.rs` and the heading-class list still
spell their own copies — small, do it); B4 (inventory schema, `tmark
schema inventory`, M4); C4 (drop `tmark-writers → tmark-fmt`, decide at
M4); the `ResolveOptions.bibliography` relativisation duplicated in the
CLI and `Config` (`pathdiff`/`relative_to`); `has_press_key` re-scans the
front matter in the LSP instead of reading `Document.front_matter`.

From `02-spec-conformance.md`: nothing fixed in code, by mandate. Its
ranking for M3 is the to-do list of the next pass; C18–C24 were added to
`12-spec-challenges.md`. The one I checked myself: D13 (anchors with an
undeclared prefix do not resolve) is the spec's lookup rule, not a bug —
now C24.

## What M3 still lacks (against `11-roadmap.md` §M3 and `08-lsp.md`)

1. **A person installing the `.vsix` and trying it.** Everything is tested
   over the in-memory connection and the binary over stdio; nobody has
   opened VS Code. Expect small things: activation on `.md` files without
   `press` (the client sends them all; the server stays quiet — check
   that VS Code does not show "TMark" errors for a README), the output
   channel, the restart command.
2. **`press` schema merge** into front-matter completion (`Config.press_schema`
   is parsed, unused). `completion::front_matter` walks `tmark::schema("frontmatter")`;
   merge the external JSON schema's `properties` under `press` and offer
   its keys.
3. **Diagnostics for included files** (`08-lsp.md` says per file). The
   analysis has the spans with `FileId(n)` and `Resolved.files`; publish a
   `PublishDiagnostics` per included path (`convert::path_to_uri`).
4. **Outline and folding** for `Para([Aside])` and generated images
   (IR review §2–3).
5. **Completion** of `{.` classes seen in the document, image paths
   (needs `Loader::list`, or `std::fs::read_dir` at the edge), `{{` paths
   from the front matter.
6. **Range formatting** (whole-document diff) and incremental text sync if
   the 1 MB case matters.
7. **Fixes beyond `deprecated`**: the audit's D5–D8 rows emit nothing;
   `caption-id-off-convention` could offer the conventional prefix;
   `deprecated-frontmatter-key` could move the key under `press`.
8. **Editor polish**: per-platform download of the binary (only the
   bundled or PATH binary today), `LICENSE` file for `vsce`, a changelog
   entry when released, the `TMARK_DEV` variable in `launch.json` is
   unused.

## Plan of attack for M4 (writers and preview)

Design: `07-writers.md`, with the architecture review's C4 amendment
(no CommonMark writer: `Profile::Mkdocs` in `tmark-fmt`; `tmark-writers`
holds `Writer`, `Body`, `Requires`, `SourceMap`, `html`, `latex`, `typst`).
Suggested order: HTML writer first (the CommonMark suite compares HTML,
`10-testing.md` §2, and the LSP preview can show it), then LaTeX against
TeXSmith's output on its docs, then Typst with the in-process preview
(ADR 0005). Before starting: run the three missing reviews; the printer
critic's over-escaping count decides whether `tmark fmt` is ready to
touch TeXSmith's corpus, and the parser adversary decides whether the
totality property test is enough.

## Pitfalls learned this pass

- **The shell.** `cargo` is not on `PATH` in the agent's non-interactive
  shell: `export PATH=$HOME/.cargo/bin:$PATH`. zsh expands a bare `=====`
  as a command. The `rtk` hook filters command output aggressively (test
  results and clippy findings vanish); prefix with `rtk proxy` to see
  everything, and read `~/.local/share/rtk/tee/*.log` when in doubt.
- **`rustfmt` versus Python patch scripts, again.** Every `cargo fmt` moves
  the anchors; patch, then format, then verify with a build — never
  format between writing a patch script and running it. Two commits in
  this pass were amended because a patch silently failed after a format.
- **`serde_json` `preserve_order`.** `Value::Object` is an `IndexMap`;
  `Map::remove` swaps the last key into the hole, `shift_remove` keeps
  order. `structural_json` depends on this to leave the fixtures' key
  order untouched.
- **`lsp_types::Uri` has interior mutability**; clippy refuses it as a map
  key (`mutable_key_type`). Key maps by `uri.as_str()` and keep the `Uri`
  in the value; `WorkspaceEdit.changes` is built at the end by `collect`.
- **Node ids restart in every included file.** `Resolution::Label { target }`
  alone does not identify a label; look labels up by id (`Labels::get`).
- **`walk`'s `NodeRef` lifetime** is now tied to the document
  (`walk<'a>(doc: &'a Document, f: impl FnMut(NodeRef<'a>))`), which is what
  lets `find` and `nodes_at` return nodes. Closures that collected
  `NodeRef`s before could not.
- **The counter item deprecation message** says "write `#(prefix:key)`"
  while the canonical spelling (and the fix) is `{counter}(prefix:key)`.
  Harmless, but pick one.
- **Reviewer agents and usage limits.** Six parallel reviewers were killed
  by a session limit, then three by an "out of credits" error on the
  second attempt. Launch them two at a time, and write the report file
  early and incrementally so a kill loses less.

## Commands

```sh
export PATH=$HOME/.cargo/bin:$PATH
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
cargo run -q -p tmark-ir --example schema && cargo run -q -p tmark-ir --example registries && git diff --exit-code
cargo build -p tmark-syntax --example dump && python3 scripts/fixture-ir.py   # then review the diff
cargo run -q -p tmark-cli -- check spec/tmark.md
cargo run -q -p tmark-cli -- lint --fix FILE
cargo run -q -p tmark --example fixes -- FILE                                  # what --fix would do
cargo run --release -q -p tmark-syntax --example bench -- spec/tmark.md
(cd editors/vscode && npm install && npm test && npm run build:grammar && npm run bundle:server && npm run package)
code --install-extension editors/vscode/vscode-tmark-0.1.0.vsix
```
