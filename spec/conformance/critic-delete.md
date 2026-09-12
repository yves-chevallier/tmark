# Critic markup: deletion

Spec Appendix "PyMdownX compatibility profile", challenge C49: `{--x--}` is a
reviewer's deletion, a `Span{.critic}` around the `Strikeout` the appendix
names. LaTeX emits `\tsdel{…}` of the `ts-critic` contract, Typst `#ts-del`,
the web a `del` element. Nothing fires inside a code span (C14): TMark never
reads a construct in code, where PyMdownX's critic does.

## input

```md
A {--redundant--} word, and `{--not this--}` in code.
```

## canonical

```md
A {--redundant--} word, and `{--not this--}` in code.
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
          "text": "A "
        },
        {
          "type": "Span",
          "content": [
            {
              "type": "Strikeout",
              "content": [
                {
                  "type": "Str",
                  "text": "redundant"
                }
              ]
            }
          ],
          "attrs": {
            "classes": [
              "critic"
            ]
          }
        },
        {
          "type": "Str",
          "text": " word, and "
        },
        {
          "type": "Code",
          "text": "{--not this--}"
        },
        {
          "type": "Str",
          "text": " in code."
        }
      ]
    }
  ]
}
```
