# Reference to an anchor with no number

Spec §Anchor: a numeric reference to an anchor whose host has no counter
(a span, a `Div`, a sub-figure of an unnumbered container) is the warning
`ref-unnumbered`; the writers show the anchor's text or its id in place of
the number, where they once printed `?` or a `\ref` to the enclosing
section. The textual form `[text](#id)` is the one to write, and is silent.

## input

```md
This [claim]{#claim} and []{#top} are anchors.

::: div {#d1}
A block.
:::

::: figure
![Left](l.svg){#fig:u}
![Right](r.svg){#fig:v}
:::

See @claim, @top, @d1, @fig:u, @[d1, second], and [the claim](#claim).
```

## canonical

```md
This [claim]{#claim} and []{#top} are anchors.

::: div {#d1}
A block.
:::

::: figure
![Left](l.svg){#fig:u}
![Right](r.svg){#fig:v}
:::

See @claim, @top, @d1, @fig:u, @[d1, second], and [the claim](#claim).
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
          "text": "This "
        },
        {
          "type": "Span",
          "content": [
            {
              "type": "Str",
              "text": "claim"
            }
          ],
          "attrs": {
            "id": "claim"
          }
        },
        {
          "type": "Str",
          "text": " and "
        },
        {
          "type": "Span",
          "attrs": {
            "id": "top"
          }
        },
        {
          "type": "Str",
          "text": " are anchors."
        }
      ]
    },
    {
      "type": "Div",
      "name": "div",
      "content": [
        {
          "type": "Para",
          "content": [
            {
              "type": "Str",
              "text": "A block."
            }
          ]
        }
      ],
      "attrs": {
        "id": "d1"
      }
    },
    {
      "type": "Figure",
      "content": [
        {
          "type": "Para",
          "content": [
            {
              "type": "Image",
              "src": "l.svg",
              "alt": [
                {
                  "type": "Str",
                  "text": "Left"
                }
              ],
              "attrs": {
                "id": "fig:u"
              }
            },
            {
              "type": "SoftBreak"
            },
            {
              "type": "Image",
              "src": "r.svg",
              "alt": [
                {
                  "type": "Str",
                  "text": "Right"
                }
              ],
              "attrs": {
                "id": "fig:v"
              }
            }
          ]
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
              "key": "claim"
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
              "key": "top"
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
              "key": "d1"
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
              "key": "fig:u"
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
              "key": "d1",
              "suffix": "second"
            }
          ]
        },
        {
          "type": "Str",
          "text": ", and "
        },
        {
          "type": "Link",
          "content": [
            {
              "type": "Str",
              "text": "the claim"
            }
          ],
          "target": {
            "type": "Anchor",
            "value": "claim"
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
ref-unnumbered @ 12:5-12:11
ref-unnumbered @ 12:13-12:17
ref-unnumbered @ 12:19-12:22
ref-unnumbered @ 12:24-12:30
ref-unnumbered @ 12:32-12:45
```
