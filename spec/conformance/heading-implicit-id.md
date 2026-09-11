# Heading implicit id

Spec §Header: a heading without `{#id}` has an implicit id derived at
resolution with the GitHub rule; it is a label like any other, never stored
or printed, and a reference to it is the hint `ref-implicit-id`.

## input

```md
## Boot sequence

See [](#boot-sequence) and @boot-sequence.
```

## canonical

```md
## Boot sequence

See [](#boot-sequence) and @boot-sequence.
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C38) with input and canonical only. -->

## resolution

```text
ref-implicit-id @ 3:5-3:23
ref-implicit-id @ 3:28-3:42
```
