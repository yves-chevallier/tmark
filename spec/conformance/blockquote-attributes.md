# Block quote, attribute list in host position

Spec §Attributes, Table "Hosts": a block quote takes an attribute list on
a line holding only the list, closing the quote's last paragraph
(`> {.epigraph}`); the list is the quote's, not a host-less one, so no
`attr-no-host` is reported (challenge C60). A quote tagged `{.epigraph}`
renders as an epigraph.

## input

```md
> Simplicity is prerequisite for reliability.
> {.epigraph}
```

## canonical

```md
> Simplicity is prerequisite for reliability.
> {.epigraph}
```

## ir

```json
{
  "blocks": [
    {
      "type": "BlockQuote",
      "content": [
        {
          "type": "Para",
          "content": [
            {
              "type": "Str",
              "text": "Simplicity is prerequisite for reliability."
            }
          ]
        }
      ],
      "attrs": {
        "classes": [
          "epigraph"
        ]
      }
    }
  ]
}
```
