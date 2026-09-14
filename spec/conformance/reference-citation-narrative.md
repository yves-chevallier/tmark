# Narrative citations by default

Spec §Cite, Table "The feature registry" (C51): under `citations.narrative`
a bare `@key` is the narrative citation (`\textcite`, `form: "prose"`);
`@[key, locator]` stays the short, parenthetical one, `+key` and `-key`
read as written. `Ref.bracketed` is a sugar field for the IR and a meaning
for the print writers only under this switch; the web keeps its one
author-year form.

## input

```md
---
press:
  features:
    citations.narrative: true
  sources:
    bibliography:
      ein05: {type: article, author: "Einstein, Albert", year: 1905, title: Zur Elektrodynamik bewegter Körper}
---

As @ein05 showed, the effect is real @[ein05, p. 33] and dated @[-ein05]; see @[+ein05; ein05].
```

## canonical

```md
---
press:
  features:
    citations.narrative: true
  sources:
    bibliography:
      ein05: {type: article, author: "Einstein, Albert", year: 1905, title: Zur Elektrodynamik bewegter Körper}
---

As @ein05 showed, the effect is real @[ein05, p. 33] and dated @[-ein05]; see @[+ein05; ein05].
```

## ir

```json
{
  "front_matter": {
    "raw": "---\npress:\n  features:\n    citations.narrative: true\n  sources:\n    bibliography:\n      ein05: {type: article, author: \"Einstein, Albert\", year: 1905, title: Zur Elektrodynamik bewegter Körper}\n---",
    "keys": {
      "press": {
        "sources": {
          "bibliography": {
            "ein05": {
              "type": "article",
              "author": "Einstein, Albert",
              "year": 1905,
              "title": "Zur Elektrodynamik bewegter Körper"
            }
          }
        },
        "features": {
          "citations.narrative": true
        }
      }
    }
  },
  "blocks": [
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "As "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "ein05"
            }
          ]
        },
        {
          "type": "Str",
          "text": " showed, the effect is real "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "ein05",
              "suffix": "p. 33"
            }
          ]
        },
        {
          "type": "Str",
          "text": " and dated "
        },
        {
          "type": "Ref",
          "items": [
            {
              "suppress_author": true,
              "key": "ein05"
            }
          ]
        },
        {
          "type": "Str",
          "text": "; see "
        },
        {
          "type": "Ref",
          "items": [
            {
              "narrative": true,
              "key": "ein05"
            },
            {
              "key": "ein05"
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
