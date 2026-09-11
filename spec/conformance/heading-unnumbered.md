# Unnumbered and unlisted headings

Spec §Header: `.unnumbered` takes a heading out of the numbering sequence,
`.unlisted` also out of the table of contents; each applies to its own
heading over the `press.numbered` default. Pandoc's `{-}` is literal text.

## input

```md
# Preface {.unnumbered}

## Colophon {.unlisted}
```

## canonical

```md
# Preface {.unnumbered}

## Colophon {.unlisted}
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C39) with input and canonical only. -->

