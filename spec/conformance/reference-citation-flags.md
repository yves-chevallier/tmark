# Citation item flags

Spec §Cite (C51): a bare `@key` is the short citation the bibliography
style gives, the same as `@[key]`; inside the brackets `+key` is the
narrative citation and `-key` the year alone, whatever the document
default, so both forms are always spellable. Backends: `\cite`,
`\textcite`, `\citeyear`; `#cite(<key>)` with `form: "prose"` or
`form: "year"`; the web has one author-year form and reads no flag.

## input

```md
---
press:
  sources:
    bibliography:
      ein05: {type: article, author: "Einstein, Albert", year: 1905, title: Zur Elektrodynamik bewegter Körper}
      ko20: {type: book, author: "Ada Knuth and Bob Ritchie", year: 2020, title: Systems}
---

Time is relative @ein05 and @[ein05]; as @[+ein05] showed, and @[+ein05, p. 33; ko20], the year @[-ein05].
```

## canonical

```md
---
press:
  sources:
    bibliography:
      ein05: {type: article, author: "Einstein, Albert", year: 1905, title: Zur Elektrodynamik bewegter Körper}
      ko20: {type: book, author: "Ada Knuth and Bob Ritchie", year: 2020, title: Systems}
---

Time is relative @ein05 and @[ein05]; as @[+ein05] showed, and @[+ein05, p. 33; ko20], the year @[-ein05].
```

## ir

```json
{
  "front_matter": {
    "raw": "---\npress:\n  sources:\n    bibliography:\n      ein05: {type: article, author: \"Einstein, Albert\", year: 1905, title: Zur Elektrodynamik bewegter Körper}\n      ko20: {type: book, author: \"Ada Knuth and Bob Ritchie\", year: 2020, title: Systems}\n---",
    "keys": {
      "press": {
        "sources": {
          "bibliography": {
            "ein05": {
              "type": "article",
              "author": "Einstein, Albert",
              "year": 1905,
              "title": "Zur Elektrodynamik bewegter Körper"
            },
            "ko20": {
              "type": "book",
              "author": "Ada Knuth and Bob Ritchie",
              "year": 2020,
              "title": "Systems"
            }
          }
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
          "text": "Time is relative "
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
          "text": " and "
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
          "text": "; as "
        },
        {
          "type": "Ref",
          "items": [
            {
              "narrative": true,
              "key": "ein05"
            }
          ]
        },
        {
          "type": "Str",
          "text": " showed, and "
        },
        {
          "type": "Ref",
          "items": [
            {
              "narrative": true,
              "key": "ein05",
              "suffix": "p. 33"
            },
            {
              "key": "ko20"
            }
          ]
        },
        {
          "type": "Str",
          "text": ", the year "
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
          "text": "."
        }
      ]
    }
  ]
}
```
