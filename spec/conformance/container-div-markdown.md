# `<div markdown>` container

Spec §Div: an HTML block whose opening tag carries `markdown` (`md_in_html`)
is sugar for a container named after the tag, `id` and `class` becoming the
attribute list; `<div markdown>` is `::: div`, class E, kept indefinitely.

## input

```md
<div class="grid cards" markdown>
**Bold** inside.
</div>
```

```md
::: div {.grid .cards}
**Bold** inside.
:::
```

## canonical

```md
::: div {.grid .cards}
**Bold** inside.
:::
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C37) with input and canonical only. -->

