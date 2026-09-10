# Review 01 — Parser adversary (handoff mandate 1)

Run by the implementing agent as a compact pass after two reviewer agents
died on usage limits (the second one left only a skeleton). Scope:
`crates/tmark-markdown` (vendored markdown-rs 1.0.0 with the TMark
constructs) and `crates/tmark-syntax`, at HEAD after the sub-span work.
Method: a scratch program (`scratchpad/parser/adv`) parses 34 adversarial
inputs and checks, for each: no panic in `parse` or `format`; every node,
diagnostic and sub-span (`key_span`, `id_span`, counter `key_span`) is in
bounds and on a `char` boundary; every sub-span's source slice equals the
key or id it names; `format` is idempotent and `parse(format(doc))` is
structurally equal to `doc`. Findings already in `reviews/02` (D1–D34)
and `reviews/03` (U1–U11) are not repeated.

## Summary

Two of the 34 inputs failed for parser reasons (both fixed in this pass),
two for printer reasons already reported as U2 and U4, and the round-trip
corpus of the printer critic surfaced a panic in the vendored tokenizer
that is an upstream markdown-rs bug (guarded, not fixed). The Unicode
class, CRLF, tab, empty-group, unbalanced-bracket, soft-break, deep
nesting and only-a-sigil cases all pass: spans on boundaries, sub-spans
exact, no panic. The cost of a 1 MB pathological file is the tokenizer's
superlinear behaviour (`reviews/06`), not a TMark construct.

## Findings

### P1. Tokenizer panic: unclosed fence in a list item, then a list of another kind (severity: panic; upstream)

```md
- ```h
1. i
```

Also with a blank line between, with `* `, `1)`, `1. ::: h` then `- i`,
and with a TMark container (`::: h`) in place of the code fence. `to_mdast`
panics with `internal error: entered unreachable code: mismatched
(non-jsx): ListUnordered / ListOrdered` (`to_mdast.rs:2129`); `to_html`
renders the same input. Reproduced with crates.io `markdown` 1.0.0
unmodified (`scratchpad/perf/upstream`): the list-item resolver injects
list boundaries that cross when the first item's flow is closed after the
second item has opened. Not fixed in the tokenizer (the document/flow exit
ordering in `construct/document.rs` needs a change that should go
upstream). Guarded: `Lowerer::tree` catches the panic and `parse` returns
the text as one paragraph with `parse-internal` (error); `tmark fmt` and
the language server refuse to format such a file. Fixture:
`diag-parse-internal.md`. Found by the printer critic (`reviews/03`,
`panic/c1.md`).

### P2. Bracketed item whose "key" is not a key repeated its suffix (severity: wrong IR; fixed)

`[@a; -@b, p. 3]`: `b` is one letter, so it is not a reference key (spec
grammar: two characters at least) and the whole item `-b, p. 3` becomes
the key, as designed; but the suffix `p. 3` was kept as well, so the
printer wrote `@[a; -b, p. 3, p. 3]` and every print added one more. The
fallback now clears the suffix (`head.rs`, unit test).

### P3. `$$ y $$ {#eq:b}` on one line: `id_span` pointed into the previous math block (severity: wrong sub-span; fixed)

The one-line display-math paragraph path (`display_math_paragraph`)
parsed its attribute list without relocating the sub-span; and the
multi-line path looked for the list on the *first* line of the math node
while it sits after the closing `$$`. Both fixed (`attrs_base_last_line`);
covered by `crates/tmark-syntax/tests/subspans.rs`.

### P4. One-letter reference keys are never keys (severity: spec question)

`@a`, `[@a]`, `@[a; b]` are literal text (or whole-item keys) because the
spec's key grammar `[A-Za-z][\w:.-]*[A-Za-z0-9]` needs two characters.
Bibliography keys of one letter are rare; document it or relax the
grammar. Not changed.

### Passed (for the record)

Links inside groups inside links; attributes after an image inside
emphasis; `@` after `(`, `[`, `«`, `—`, `¿`, CJK, emoji, a combining mark;
`@` in code spans and link destinations (literal); `#[` and `#(` at line
start in lists and quotes; equal-length nested `:::` in a list item; tabs
in an admonition body; a CRLF file with heading id, reference, index
entry, role and container; tilde fences with `yaml table`; `{}`,
`{{}}`, `[]`, `{sc}[]`, `{index}[][][]`, `@[]`, `#[]`, `#()`; a role head
split across a soft break; unbalanced `[`, `(`, `{` and an unterminated
`@[`; attribute values with `\"`, `}` and `\}` (parse side; the printer's
handling is U4); front matter without its closing `---`; documents that
are only `@`, `{`, `#[`, `@[`, `#(`; keys with non-ASCII letters; 60
nested containers; 1 MB of `@`, 150 000 references, 500 000 `{` (slow,
see `reviews/06`, but total); a caption with nothing to attach to;
counter items inside a heading; Pandoc citations; `@a.`/`@a:`/`@a-`
trailing punctuation; DOI keys; attributes on a block quote; index paths
with `main=true` and the deprecated registry suffix; abbreviation
definitions (now one span per line); footnotes and inline notes; `\|` in
a table cell (parse side).

## Not covered

Fuzzing with random bytes beyond the existing `proptest` totality test;
the MDX constructs (off); nested containers deeper than 60; the
`tmark_admonition` construct with tabs *and* nested fences; property
checks on `format` of hand-built IR (the printer critic did those).
