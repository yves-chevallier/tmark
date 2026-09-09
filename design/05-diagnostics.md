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

`Code` is an enum in `tmark-ir::diagnostic` with a `&'static str` id and a
doc comment giving the spec section. The CLI prints `file:line:col: severity
code: message`; the LSP maps the fields one to one.

## Who emits what

| Stage | Examples | Owner |
| ----- | -------- | ----- |
| Parse | `attr-no-host`, `role-dangling-head`, `container-unclosed`, `fence-unknown-node-word`, `frontmatter-yaml`, `deprecated` (spelling) | `tmark-syntax` |
| Resolve | `ref-unresolved`, `ref-ambiguous` (key in two registries), `prefix-unknown`, `prefix-host-mismatch` (`{#tbl:x}` on a figure), `label-duplicate`, `citation-shadowed-by-footnote`, `crossref-inventory-missing`, `include-missing` | `tmark-registry` |
| Lint | `hardcoded-number` ("Figure 3" in prose), `position-word` ("above", "below"), `caption-id-off-convention`, `strict-x-construct`, `deprecated-frontmatter-key`, `lead-promotion` (info: sugar promoted), `heading-skip` | `tmark-lint` |

Parse and resolve diagnostics are not optional; they are facts about the
document. Lint rules are a catalogue the user can enable, disable and
configure.

## Rules

```rust
pub trait Rule {
    fn code(&self) -> Code;
    fn default_severity(&self) -> Severity;
    fn check(&self, doc: &Document, res: &Resolved, out: &mut Vec<Diagnostic>);
}
```

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
