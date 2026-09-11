# Progress bar

Spec §ProgressBar: an inline node `{value, label}`; the PyMdownX percentage
spelling is canonical; the fraction form and the Python-Markdown `{: .thin}`
attribute colon are deprecated sugar (Appendix "Deprecation schedule").

## input

```md
[=25% "Research"]
[=9/20 "Review"]{: .thin}
```

```md
[=25% "Research"]
[=45% "Review"]{.thin}
```

## canonical

```md
[=25% "Research"]
[=45% "Review"]{.thin}
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C35) with input and canonical only. -->

## diagnostics

```text
deprecated @ 2:1-2:17
deprecated @ 2:17-2:26
```
