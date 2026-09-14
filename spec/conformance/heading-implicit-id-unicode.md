# Heading, implicit id outside ASCII

Spec §Header: the implicit id is GitHub's slug of the title, which keeps
letters of any script (`café-au-lait`, `日本語-見出し`). A bare `@key`
takes ASCII only (§Ref), so such an id is referenced bracketed or through
the empty-link form. Each backend spells the id as a label of its own:
Typst reads a label as XID characters plus `_ - : .`, so an accented or
non-Latin id is a label as written and two of them never collide (the
writer once flattened both to dashes, and `typst compile` refused the
duplicate).

## input

```md
## Café au lait

## Thé à la menthe

## 日本語 見出し

See @[café-au-lait], @[thé-à-la-menthe] and [](#日本語-見出し).
```

## canonical

```md
## Café au lait

## Thé à la menthe

## 日本語 見出し

See @[café-au-lait], @[thé-à-la-menthe] and [](#日本語-見出し).
```

## ir

```json
{
  "blocks": [
    {
      "type": "Header",
      "level": 2,
      "content": [
        {
          "type": "Str",
          "text": "Café au lait"
        }
      ]
    },
    {
      "type": "Header",
      "level": 2,
      "content": [
        {
          "type": "Str",
          "text": "Thé à la menthe"
        }
      ]
    },
    {
      "type": "Header",
      "level": 2,
      "content": [
        {
          "type": "Str",
          "text": "日本語 見出し"
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "See "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "café-au-lait"
            }
          ]
        },
        {
          "type": "Str",
          "text": ", "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "thé-à-la-menthe"
            }
          ]
        },
        {
          "type": "Str",
          "text": " and "
        },
        {
          "type": "Link",
          "target": {
            "type": "Anchor",
            "value": "日本語-見出し"
          }
        },
        {
          "type": "Str",
          "text": "."
        }
      ]
    }
  ]
}
```

## resolution

```text
ref-implicit-id @ 7:5-7:21
ref-implicit-id @ 7:23-7:43
ref-implicit-id @ 7:48-7:72
```

## typst

```typst
== Café au lait <café-au-lait>

== Thé à la menthe <thé-à-la-menthe>

== 日本語 見出し <日本語-見出し>

See #ref(<café-au-lait>, supplement: [Section]), #ref(<thé-à-la-menthe>, supplement: [Section]) and #ref(<日本語-見出し>, supplement: none).
```
