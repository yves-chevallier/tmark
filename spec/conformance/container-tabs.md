# Tabs

Spec §Tabs: `:::: tabs` holds `::: tab {title=…}` containers; the PyMdownX
`=== "Title"` line plus its indented body is class-E sugar, kept
indefinitely. Paged writers render the tabs in sequence as titled blocks.

## input

```md
=== "Windows"

    Windows is a Microsoft operating system.

=== "Linux"

    Linux is an open-source operating system.
```

```md
:::: tabs
::: tab {title=Windows}
Windows is a Microsoft operating system.
:::
::: tab {title=Linux}
Linux is an open-source operating system.
:::
::::
```

## canonical

```md
:::: tabs
::: tab {title=Windows}
Windows is a Microsoft operating system.
:::
::: tab {title=Linux}
Linux is an open-source operating system.
:::
::::
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C31) with input and canonical only. -->

