# Straight quotes next to a dash

Spec Appendix "PyMdownX compatibility profile", `"quotes"` row: an opening
quote may follow a dash character (`-`, `—`, `–`, `/`) as well as
whitespace or a bracket, and a closing quote may be followed by a dash
character as well as whitespace, the end of the run, or other punctuation.
Regression b3ad4fa: a closing quote before an em dash was rejected because
the boundary check only accepted ASCII punctuation after the quote, so the
pairing fell through to the next `"` of the paragraph.

## input

```md
Authors declare what a document element is—"this is a theorem," "this is a section," "this is a quotation"—and let the macros decide.

The root meant "art, craft, technique"—fitting for a craft-based reading. The "answer" is obvious.
```

## canonical

```md
Authors declare what a document element is—"this is a theorem," "this is a section," "this is a quotation"—and let the macros decide.

The root meant "art, craft, technique"—fitting for a craft-based reading. The "answer" is obvious.
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
          "text": "Authors declare what a document element is—"
        },
        {
          "type": "Quoted",
          "kind": "double",
          "content": [
            {
              "type": "Str",
              "text": "this is a theorem,"
            }
          ]
        },
        {
          "type": "Str",
          "text": " "
        },
        {
          "type": "Quoted",
          "kind": "double",
          "content": [
            {
              "type": "Str",
              "text": "this is a section,"
            }
          ]
        },
        {
          "type": "Str",
          "text": " "
        },
        {
          "type": "Quoted",
          "kind": "double",
          "content": [
            {
              "type": "Str",
              "text": "this is a quotation"
            }
          ]
        },
        {
          "type": "Str",
          "text": "—and let the macros decide."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "The root meant "
        },
        {
          "type": "Quoted",
          "kind": "double",
          "content": [
            {
              "type": "Str",
              "text": "art, craft, technique"
            }
          ]
        },
        {
          "type": "Str",
          "text": "—fitting for a craft-based reading. The "
        },
        {
          "type": "Quoted",
          "kind": "double",
          "content": [
            {
              "type": "Str",
              "text": "answer"
            }
          ]
        },
        {
          "type": "Str",
          "text": " is obvious."
        }
      ]
    }
  ]
}
```

## latex

```latex
Authors declare what a document element is---\enquote{this is a theorem,} \enquote{this is a section,} \enquote{this is a quotation}---and let the macros decide.

The root meant \enquote{art, craft, technique}---fitting for a craft-based reading. The \enquote{answer} is obvious.
```

## html

```html
<p>Authors declare what a document element is—“this is a theorem,” “this is a section,” “this is a quotation”—and let the macros decide.</p>
<p>The root meant “art, craft, technique”—fitting for a craft-based reading. The “answer” is obvious.</p>
```
