# Reference, digit-initial key

Spec §Identifiers (challenge C27): a key starts with a letter or a digit
(Zotero's `1RgTv`), so a bracketed item whose key is digit-initial keeps
its prefix and locator out of the key. A bare `@` reference still needs a
letter first, so such a key is written bracketed.

## input

```md
Cite @[7HA7H, p. 3] and @[see 1RgTv; -ko20].
```

## canonical

```md
Cite @[7HA7H, p. 3] and @[see 1RgTv; -ko20].
```

## ir

```json
{
  "blocks": [
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "Cite "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "7HA7H",
              "suffix": "p. 3"
            }
          ]
        },
        {
          "type": "Str",
          "text": " and "
        },
        {
          "type": "Ref",
          "items": [
            {
              "prefix": "see",
              "key": "1RgTv"
            },
            {
              "suppress_author": true,
              "key": "ko20"
            }
          ]
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
