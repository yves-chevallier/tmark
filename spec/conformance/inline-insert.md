# Caret insert with the feature on

Spec §Inline text, Appendix "PyMdownX compatibility profile": with
`inline.insert` on, `^^x^^` is sugar for `{underline}[x]`; the printer
emits the role.

## input

```md
---
press:
  features:
    inline.insert: true
---

Now ^^inserted^^ text.
```

```md
---
press:
  features:
    inline.insert: true
---

Now {underline}[inserted] text.
```

## canonical

```md
---
press:
  features:
    inline.insert: true
---

Now {underline}[inserted] text.
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C41) with input and canonical only. -->

