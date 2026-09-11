# Icon shortcodes

Spec §Emoji and icon shortcodes: a Material icon shortcode lowers to
`Span{.icon media=web}` holding the shortcode as text; print drops it, the
printer writes the shortcode back, and lint hints `icon-web-only`.

## input

```md
Click :material-cog: Settings.
```

## canonical

```md
Click :material-cog: Settings.
```

## ir

<!-- TODO(parser wave): fill in the IR JSON block; this fixture was written
by the spec wave (challenge C34) with input and canonical only. -->

## resolution

```text
icon-web-only @ 1:7-1:21
```
