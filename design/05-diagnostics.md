# 05 — Diagnostics and lint

Crates: `tmark-ir` (the `Diagnostic` type), `tmark-syntax` (syntactic
diagnostics), `tmark-registry` (resolution diagnostics), `tmark-lint` (the
rule catalogue). Spec: P4 "diagnostics are loud", §Tooling roadmap item 4.

## One type

```rust
pub struct Diagnostic {
    pub code: Code,               // stable identifier, e.g. "ref-unresolved"
    pub severity: Severity,       // Error | Warning | Info | Hint
    pub span: Span,
    pub message: String,          // one sentence, no trailing period, names the construct
    pub fix: Option<Fix>,         // a NodeEdit or a text edit the LSP can apply
    pub related: Vec<(Span, String)>,   // e.g. the other definition of a duplicate key
}
```

`Code` is an enum in `tmark-ir::diagnostic` with a `&'static str` id, a
doc comment giving the spec section (also `Code::doc()`, for tools that
list the catalogue) and a `Code::stage()` (parse, resolve, lint). The CLI
prints `file:line:col: severity code: message`; the LSP maps the fields one
to one.

**Positions.** A span is bytes; nothing stores a line or a column. When a
tool prints one, `line` and `col` are **1-based, and `col` counts bytes**
from the start of the line (`LineIndex::line_col` plus one on each): the
convention of `tmark-cli`, of the Python binding's `line`/`col` fields and
of TeXSmith's renderer, so that both print identical lines for identical
findings (migration decision X10). The LSP is the exception: it speaks
UTF-16 code units (`LineColUtf16`, design 08 §Positions). Revisit
characters versus bytes when an editor complains; change both sides or
neither.

## Who emits what

| Stage | Examples | Owner |
| ----- | -------- | ----- |
| Parse | `attr-no-host`, `role-dangling-head`, `container-unclosed`, `fence-unknown-node-word`, `frontmatter-yaml`, `deprecated` (spelling), `compat-unsupported` (a PyMdownX spelling recognised but not implemented yet: literal text plus a warning), `container-orphan` (a `tab` outside `tabs`, hint), `parse-internal` (the tokenizer failed: the text is one paragraph, an error) | `tmark-syntax` |
| Parse | `attr-no-host`, `role-dangling-head`, `container-unclosed`, `fence-unknown-node-word`, `frontmatter-yaml`, `deprecated` (spelling), `parse-internal` (the tokenizer failed: the text is one paragraph, an error), `table-yaml`, `table-unknown-key`, `table-columns`, `table-align`, `table-shape`, `table-row-width`, `table-span`, `table-column-unknown` (the `yaml table` schema) | `tmark-syntax` |
| Resolve | `ref-unresolved`, `ref-ambiguous` (key in two registries), `prefix-unknown`, `prefix-host-mismatch` (`{#tbl:x}` on a figure), `label-duplicate`, `citation-shadowed-by-footnote`, `crossref-inventory-missing`, `include-missing`, `ref-implicit-id` (hint: a reference to a heading's implicit id), `ref-unnumbered` (a numeric reference to an anchor with no counter: it renders the anchor's text) | `tmark-registry` |
| Lint | `hardcoded-number` ("Figure 3" in prose), `position-word` ("above", "below"), `caption-id-off-convention`, `strict-x-construct`, `deprecated-frontmatter-key`, `lead-promotion` (info: sugar promoted), `heading-skip`, `table-placement`, `table-width`, `table-width-sum`, `directive-foreign`, `icon-web-only`, `feature-off` (hints) | `tmark-lint` |
| Parse | `attr-no-host`, `role-dangling-head`, `container-unclosed`, `fence-unknown-node-word`, `frontmatter-yaml`, `deprecated` (spelling), `compat-unsupported` (a PyMdownX spelling recognised but not implemented yet: literal text plus a warning), `parse-internal` (the tokenizer failed: the text is one paragraph, an error) | `tmark-syntax` |
| Parse | `attr-no-host`, `role-dangling-head`, `container-unclosed`, `fence-unknown-node-word`, `frontmatter-yaml`, `deprecated` (spelling), `parse-internal` (the tokenizer failed: the text is one paragraph, an error), `table-yaml`, `table-unknown-key`, `table-columns`, `table-align`, `table-shape`, `table-row-width`, `table-span`, `table-column-unknown` (the `yaml table` schema) | `tmark-syntax` |
| Resolve | `ref-unresolved`, `ref-ambiguous` (key in two registries), `prefix-unknown`, `prefix-host-mismatch` (`{#tbl:x}` on a figure), `label-duplicate`, `citation-shadowed-by-footnote`, `crossref-inventory-missing`, `include-missing` | `tmark-registry` |
| Lint | `hardcoded-number` ("Figure 3" in prose), `position-word` ("above", "below"), `caption-id-off-convention`, `strict-x-construct`, `deprecated-frontmatter-key`, `lead-promotion` (info: sugar promoted), `heading-skip`, `table-placement`, `table-width`, `table-width-sum` | `tmark-lint` |

Parse and resolve diagnostics are not optional; they are facts about the
document. Lint rules are a catalogue the user can enable, disable and
configure.

## Rules

```rust
pub struct Context<'a> { pub doc: &'a Document, pub resolved: &'a Resolved, pub text: &'a str }

pub trait Rule {
    fn code(&self) -> Code;
    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>);
}
```

The severity comes from `Code::default_severity`, overridden by the
configuration.

One file per rule under `crates/tmark-lint/src/rules/`, registered in a
`const RULES: &[&dyn Rule]`. A rule is pure and stateless. A rule that needs
to look at the text (position words) walks `Str` nodes; a rule that needs
registries reads `Resolved`.

Configuration: `[lint]` table in `tmark.toml` (workspace) or the front
matter's `features` key when the spec names a feature. Format:
`code = "off" | "info" | "warning" | "error"`.

## Fixes

A diagnostic may carry a `Fix`. Fixes are produced by the emitter (it knows
the replacement), applied by `tmark-fmt::edit` (it knows how to print). The
CLI applies them with `tmark lint --fix`; the LSP exposes them as code
actions. A fix is only offered when it is safe: rewriting a deprecated
spelling is safe, guessing a label for an unresolved reference is not.

## Severities by default

- Error: front-matter YAML errors, an unknown key under a namespace the spec
  validates (`declare`, `sources`, `features`), an X-class construct under the
  strict profile, a `yaml table` the schema rejects (every `table-*` code
  but `table-width-sum`).
- Warning: unresolved and ambiguous references, unknown prefixes, duplicate
  labels, deprecated spellings, missing inventories.
- Info: sugar the formatter will rewrite, lead-in promotions.
- Hint: style rules (position words, hard-coded numbers).

`tmark check` exits non-zero on errors, and on warnings with `--strict`.

## Implementation notes (milestone 2)

- `tmark-lint` exposes `lint(doc, resolved, text, config)` and a `Rule`
  trait whose `check` receives a `Context { doc, resolved, text }`: the
  source text is there for rules that look at spellings (lead promotion).
- Rules shipped: `hardcoded-number`, `position-word`,
  `caption-id-off-convention`, `heading-skip`, `lead-promotion`.
  `strict-x-construct` waits for the strict profile's parse-time reporting
  (milestone 5); `deprecated-frontmatter-key` and `role-unknown` are emitted
  by the parser, not by a rule.
- `Config` maps a `Code` to `off | hint | info | warning | error`; the CLI
  takes `--level code=level` and the `[lint]` table of `tmark.toml`
  (`tmark::Config`, milestone 3).

## Implementation notes (milestone 3)

- Fixes exist for `deprecated`: `tmark::fixes` finds the node whose span is
  the diagnostic's and reprints it canonically (`tmark_fmt::print_node`).
  The language server offers them as quick fixes; `tmark lint --fix`
  applies them last-first. No other code carries a fix yet; the spec
  conformance audit (`reviews/02`) lists deprecation rows that emit no
  diagnostic at all (D5–D8).
- `ref-unresolved` is reported on the key token (`RefItem.key_span`), not
  on the whole `@[…]` group.
- `Code` gained `ALL` and `from_id` (the CLI and the LSP parse codes).
- `tmark check FILE… [.bib…] [--strict] [--level …]` prints parse, resolve
  and lint diagnostics and exits 1 on errors (warnings under `--strict`);
  `tmark lint` is its alias until `--fix` lands with the LSP fixes.
- Conformance fixtures gained a `## resolution` section for the resolve
  and lint stages; one fixture per diagnostic code lives under
  `spec/conformance/diag-*.md` and `lint-*.md`.

## Implementation notes (migration wave 1)

- `deprecated-frontmatter-key` carries a fix: the whole YAML island with
  every deprecated key moved to its canonical place, as a *line edit*
  (`tmark_ir::yaml_edit::move_key`: the key's block is cut, dedented,
  renamed when the spelling changes, and re-indented at the end of its
  target mapping, which is created when missing). Nothing else in the
  island moves, which is what the printer's byte-for-byte copy of the
  front matter asks for. A key with a flow value (`press: {…}`) on the
  path gives no fix. Every deprecated key's diagnostic carries the same
  replacement, so `lint --fix` moves them all in one pass (overlapping
  fixes after the first are skipped). The message names the target
  (`` `counters` is deprecated, write `press.declare.counters` ``);
  `frontmatter::deprecated_key_target` is the one table.
- `compat-unsupported` (warning, parse stage, `tmark-syntax/src/lower/compat.rs`)
  replaces silence for the PyMdownX spellings milestone 5 will implement.
  The wave that introduced it covered content tabs (`=== "Title"`
  paragraphs), critic markup, progress bars (`[=n% "label"]`), wiki links
  (`[[…]]`), emoji and icon shortcodes (`:smile:`, `:material-…:`, not
  inside a word), `^^…^^` without `inline.insert`, lower-case fancy list
  markers (`a.`, `iv.`, `#.`, `1)`) and `[TOC]` at a paragraph start; what
  is left today is the wiki link and the fancy list marker, each other
  spelling having become a construct. It is a new code rather than
  `strict-x-construct` because these are not X-class deviations under a
  profile: they are constructs of the compatibility appendix that every
  profile will accept once implemented, and the strict profile must keep
  reporting X1/X3 separately. A spelling escaped at its first character
  (`\[TOC]`, `\:smile:`, a paragraph starting with `\`) is the author's
  literal text and is not reported. The scans are narrow on purpose (a
  miss is the old behaviour, a false positive is a wrong warning on
  prose): upper-case list markers (`I. M. Pei`) and shortcodes glued to a
  word (`a:b:`) are left alone. Fixture `diag-compat-unsupported`.
- `deprecated` on `[^key]` / `^[k1,k2]` citations, `/// latex`,
  `/// caption` blocks, `[](gls:term)`, `{index}[…]{b}`, `{index:r}[…]`
  and the `--8<--` fence body all carry the generic node-reprint fix
  (`tmark::fixes`), so the fix for a spelling lives in the printer once.
## Implementation notes (wave 1, tables — decision X9)

The checks of `texsmith.extensions.tables.schema` are split by whether the
IR can hold the offending shape (design 03 §Tables):

- Parse time (`tmark-syntax::lower::table_yaml`, the port of `parse_table`
  and `build_matrix`), on the fence span, all errors: `table-yaml` (not
  YAML, or not a mapping: the fence stays a `CodeBlock` with its info
  string), `table-unknown-key` (Python `extra="forbid"` at every level),
  `table-columns` (missing, not a list, fewer than two, a bad descriptor,
  a group without name or columns), `table-align`, `table-shape` (a value
  of the wrong type: rows, `long`, widths, cells, separators, named rows),
  `table-row-width` (extra or missing cells, a list longer than its group),
  `table-span` (collisions, a value under a row span, spans past the last
  column or row, a separator inside a span), `table-column-unknown`
  (named-row mode). The document still parses: the `Table` carries a
  best-effort model and its `source`, which the printer writes back as
  typed, and the lint rules skip it.
- Lint (`tmark-lint`), on tables the model holds faithfully:
  `table-placement` (error, `^[hHtbpT!]+$`), `table-width` (error: empty,
  or a percentage outside (0, 100], on the table, a column or a
  table-config entry), `table-width-sum` (warning: column percentages over
  100; TeXSmith has no such check, the layout scales silently).
- One fixture per code under `spec/conformance/diag-table-*.md`; the
  accepted shapes have `fence-yaml-table-{named,settings,spans,config}.md`.
- Diagnostics are reported on the whole fence: the YAML reader has no
  positions. A per-row span is a candidate once the LSP shows the need.


## Implementation notes (C31–C42 wave)

- `compat-unsupported` now covers only what is still unimplemented:
  wiki links and fancy list markers (`lower/compat.rs`; fixture
  `diag-compat-unsupported`). Content tabs, progress bars, emoji and icon
  shortcodes, `^^x^^` and `[TOC]` left it with their constructs, and
  critic markup with challenge C49 (fixtures `critic-*`): the five
  spellings lower, so reporting them would be a warning on working
  syntax.
- `deprecated` for the attribute colon `{: …}` and the progress fraction
  `[=a/b "…"]` carries its own text fix (`Lowerer::deprecated_with_fix`):
  their spans are not a node's (the brace on a host, the head before the
  bar's attributes), so the generic node-reprint fix of `tmark::fixes`
  could not find them. On a fence info string the whole fence is the span
  and the reprint is the fix.
- `container-orphan` (parse, hint): the orphan `tab` is wrapped in a
  `tabs` of its own, so the document still renders; consecutive orphans
  form one set.
- `ref-implicit-id` (resolve, hint) is reported on the whole reference
  (the `@key` or the link), not on the key token, and only when the label
  it resolved to is implicit.
- `ref-unnumbered` (resolve, warning) is reported on the whole reference
  too, for `@key` and the empty link `[](#key)` only: `[text](#key)` is
  the textual form the message recommends. The host has no counter when
  it is an anchor-only host (a span, a `Div`, a block quote) that no
  declared series numbered, or a sub-figure of a container that has no
  label. The writers show the anchor's text (a span's content) or its id
  in place of the number.
- `directive-foreign` (lint, hint) fires per dotted directive on the
  `RawBlock{format=markdown}`; `[TOC]` is the same node and silent.
  `icon-web-only` fires per icon span. `feature-off` fires per `^^x^^` run
  found in a `Str` whose text is its source byte for byte: the escaped
  spelling the printer writes back (`\^\^x\^\^`) is literal on purpose.
