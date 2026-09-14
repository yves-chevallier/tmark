# Deprecated citation forms

Spec §Cite, Appendix "Deprecation schedule" (decision X7): `[^key]` with no
`[^key]:` definition and `^[k1,k2]` groups are citations, lowered to a bare
`Ref` (they were the short form, C51) with a `deprecated` fix to `@key` /
`@[k1; k2]`. A bare DOI
takes the `doi:` prefix. `^[` never opens a caret superscript, so two
groups in one paragraph never swallow the prose between them. A defined
label (`[^note]`) stays a footnote, a numeric one (`[^1]`) stays literal
(the parser's smoke tests cover it).
A group hugging the word before it (`tutor.^[ein05]`) prints with a space
(`tutor. @ein05`), which the X4 guard needs; see the printer's tests, and
`crates/tmark/tests/fixes.rs` for the fix of `sortie[^key]` (the space
changes the `Str`, so the runner cannot host that input). The bibliography
is declared so that the writer snapshots show what the sugar always meant:
the short citation, `\cite{ein05}` (C51).

## input

```md
---
press:
  sources:
    bibliography:
      ein05: {type: article, author: "Einstein, Albert", year: 1905, title: Zur Elektrodynamik bewegter Körper}
      AI2027: {type: report, author: "Kokotajlo, Daniel", year: 2025, title: AI 2027}
---

Slow to talk [^ein05], said the tutor. ^[ein05,AI2027] He failed the test, ^[10.1007/s00016-014-0153-5] see ^[AI2027] and [^note].

[^note]: A real footnote.
```

## canonical

```md
---
press:
  sources:
    bibliography:
      ein05: {type: article, author: "Einstein, Albert", year: 1905, title: Zur Elektrodynamik bewegter Körper}
      AI2027: {type: report, author: "Kokotajlo, Daniel", year: 2025, title: AI 2027}
---

Slow to talk @ein05, said the tutor. @[ein05; AI2027] He failed the test, @doi:10.1007/s00016-014-0153-5 see @AI2027 and [^note].

[^note]: A real footnote.
```

## ir

```json
{
  "front_matter": {
    "raw": "---\npress:\n  sources:\n    bibliography:\n      ein05: {type: article, author: \"Einstein, Albert\", year: 1905, title: Zur Elektrodynamik bewegter Körper}\n      AI2027: {type: report, author: \"Kokotajlo, Daniel\", year: 2025, title: AI 2027}\n---",
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
            "AI2027": {
              "type": "report",
              "author": "Kokotajlo, Daniel",
              "year": 2025,
              "title": "AI 2027"
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
          "text": "Slow to talk "
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
          "text": ", said the tutor. "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "ein05"
            },
            {
              "key": "AI2027"
            }
          ]
        },
        {
          "type": "Str",
          "text": " He failed the test, "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "doi:10.1007/s00016-014-0153-5"
            }
          ]
        },
        {
          "type": "Str",
          "text": " see "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "AI2027"
            }
          ]
        },
        {
          "type": "Str",
          "text": " and "
        },
        {
          "type": "Note",
          "label": "note"
        },
        {
          "type": "Str",
          "text": "."
        }
      ]
    }
  ],
  "footnotes": [
    {
      "label": "note",
      "content": [
        {
          "type": "Para",
          "content": [
            {
              "type": "Str",
              "text": "A real footnote."
            }
          ]
        }
      ]
    }
  ]
}
```

## diagnostics

```text
deprecated @ 9:14-9:22
deprecated @ 9:40-9:55
deprecated @ 9:76-9:104
deprecated @ 9:109-9:118
```
