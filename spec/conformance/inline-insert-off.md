# Caret insert with the feature off

Spec §Inline text: with `inline.insert` off (the default) `^^x^^` is
literal text and lint hints `feature-off`.

## input

```md
Now ^^inserted^^ text.
```

## canonical

```md
Now ^^inserted^^ text.
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C41) with input and canonical only. -->

## resolution

```text
feature-off @ 1:5-1:17
```
