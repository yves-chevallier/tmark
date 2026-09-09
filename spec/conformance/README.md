# Conformance fixtures

One file per construct, named after the spec section it covers
(`inline-smallcaps.md`, `role-aside.md`, `caption-table.md`…). A fixture is
a Markdown file made of fenced sections, so it is readable on GitHub and
parsed by the test runner (`crates/tmark/tests/conformance.rs`) with no
other format to learn.

## Format

````markdown
# <title>                       one H1: the construct, with the spec section in the first paragraph

## input                         one or more fenced `md` blocks; every one must parse to the same IR
```md
…sugar spelling…
```
```md
…another accepted spelling…
```

## canonical                     exactly one fenced `md` block: what `tmark fmt` prints
```md
…
```

## ir                            one fenced `json` block: the Document, spans and ids omitted
```json
…
```

## diagnostics                   optional; one line per expected diagnostic: `code @ L:C-L:C` (1-based, on the canonical input)
```text
deprecated @ 1:1-1:20
```

## latex / typst / html          optional; the body each writer must produce for the canonical input
```latex
…
```
````

Rules:

- The `ir` block is compared modulo `id` and `span` fields. Everything else
  is exact.
- Each `input` block is parsed independently and must equal the `ir`.
- The `canonical` block, parsed, must equal the `ir`, and printed again must
  equal itself (idempotence).
- Diagnostics are matched by code and span; the message is free.
- Backend blocks are `insta` snapshots: the runner fails when they differ
  and `cargo insta review` updates them.
- A fixture may include a front matter in its inputs when the construct
  depends on declarations (counters, features).

## Coverage

Every row of the construct table in `design/02-syntax.md`, every node of
`design/03-ir.md`, every diagnostic code of `design/05-diagnostics.md`, and
every entry of the spec's deprecation schedule has a fixture. The seed
fixtures below show the format; the rest are written with the constructs.
