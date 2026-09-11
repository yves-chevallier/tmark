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
| Parse | `attr-no-host`, `role-dangling-head`, `container-unclosed`, `fence-unknown-node-word`, `frontmatter-yaml`, `deprecated` (spelling), `parse-internal` (the tokenizer failed: the text is one paragraph, an error) | `tmark-syntax` |
| Resolve | `ref-unresolved`, `ref-ambiguous` (key in two registries), `prefix-unknown`, `prefix-host-mismatch` (`{#tbl:x}` on a figure), `label-duplicate`, `citation-shadowed-by-footnote`, `crossref-inventory-missing`, `include-missing` | `tmark-registry` |
| Lint | `hardcoded-number` ("Figure 3" in prose), `position-word` ("above", "below"), `caption-id-off-convention`, `strict-x-construct`, `deprecated-frontmatter-key`, `lead-promotion` (info: sugar promoted), `heading-skip` | `tmark-lint` |

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
  strict profile.
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
