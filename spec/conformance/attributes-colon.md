# Attribute list with the Python-Markdown colon

Spec §Attributes: `{: #id .cls}` (Python-Markdown `attr_list`) is accepted
on every host and deprecated; the printer drops the colon.

## input

```md
## Boot {: #sec:boot .draft}
```

```md
## Boot {#sec:boot .draft}
```

## canonical

```md
## Boot {#sec:boot .draft}
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C35) with input and canonical only. -->

## diagnostics

```text
deprecated @ 1:9-1:29
```
