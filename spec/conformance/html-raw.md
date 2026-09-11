# HTML kept as typed

Spec §Raw passthrough: inline and block HTML other than comments is
`RawInline` / `RawBlock` with `format=html`, class C, printed as typed;
paged writers drop it (the span text survives, `<br>` prints nothing).

## input

```md
Inline <span class="x">text</span> and <br> here.

<div class="note">
Raw block.
</div>
```

## canonical

```md
Inline <span class="x">text</span> and <br> here.

<div class="note">
Raw block.
</div>
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C37) with input and canonical only. -->

