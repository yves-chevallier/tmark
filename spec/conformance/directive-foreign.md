# Foreign directives

Spec §Foreign directive: `[TOC]` alone in a paragraph and a dotted `:::`
line with its indented continuation (mkdocstrings) are `RawBlock` with
`format=markdown`, kept verbatim by the printer, emitted by no other writer;
the dotted form closes at the first dedent and raises no container
diagnostic, only the lint hint `directive-foreign` (C42 for `[TOC]`).

## input

```md
[TOC]

::: texsmith.core.config
    options:
      members: true

Next paragraph.
```

## canonical

```md
[TOC]

::: texsmith.core.config
    options:
      members: true

Next paragraph.
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C40) with input and canonical only. -->

## resolution

```text
directive-foreign @ 3:1-6:1
```
