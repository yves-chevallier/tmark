# Divider

Spec §HorizontalRule: `---` is a `HorizontalRule`; the LaTeX writer emits
`\tsdivider`, the Typst writer `#ts-divider()` (a page break by default,
redefinable by the template), the HTML writer `<hr>`.

## input

```md
First page.

---

Second page.
```

## canonical

```md
First page.

---

Second page.
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C36) with input and canonical only. -->

