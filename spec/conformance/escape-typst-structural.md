# Typst structural character escaping

`design/07-writers.md` §Implementation notes: the Typst writer's markup
escaper covers more than the backslash-prefixed specials of TeXSmith's
`escaper.py` (`\ # $ * _ ` < > @ [ ]`). Plain prose text (`Str`) can still
contain a `//` or `/*` that would open a Typst comment anywhere in the
line, a leading `=`, `+`, `-` or `/ ` that would be read as a heading, a
list, an enum or a term-list marker at the start of a line, and a `~` that
Typst turns into a non-breaking space — none of TMark's own syntax, so none
of it is escaped by the parser; the Typst writer must neutralise it on the
way out.

## input

```md
Ratio a // b and the rest.

Block comment attempt: a /\* b \*/ c.

\= Not a heading, just a sentence starting with an equals sign.

\- Not a bullet, just a sentence starting with a hyphen.

\+ Not an enum item, just a sentence starting with a plus.

/ term: not a term list, just prose starting with a slash.

A tilde example: 10\~cm should stay a plain tilde.
```

## canonical

```md
Ratio a // b and the rest.

Block comment attempt: a /\* b \*/ c.

= Not a heading, just a sentence starting with an equals sign.

\- Not a bullet, just a sentence starting with a hyphen.

\+ Not an enum item, just a sentence starting with a plus.

/ term: not a term list, just prose starting with a slash.

A tilde example: 10\~cm should stay a plain tilde.
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
          "text": "Ratio a // b and the rest."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "Block comment attempt: a /* b */ c."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "= Not a heading, just a sentence starting with an equals sign."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "- Not a bullet, just a sentence starting with a hyphen."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "+ Not an enum item, just a sentence starting with a plus."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "/ term: not a term list, just prose starting with a slash."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "A tilde example: 10~cm should stay a plain tilde."
        }
      ]
    }
  ]
}
```

## latex

```latex
Ratio a // b and the rest.

Block comment attempt: a /* b */ c.

= Not a heading, just a sentence starting with an equals sign.

- Not a bullet, just a sentence starting with a hyphen.

+ Not an enum item, just a sentence starting with a plus.

/ term: not a term list, just prose starting with a slash.

A tilde example: 10\textasciitilde{}cm should stay a plain tilde.
```

## typst

```typst
Ratio a \// b and the rest.

Block comment attempt: a \/\* b \*/ c.

\= Not a heading, just a sentence starting with an equals sign.

\- Not a bullet, just a sentence starting with a hyphen.

\+ Not an enum item, just a sentence starting with a plus.

\/ term: not a term list, just prose starting with a slash.

A tilde example: 10\~cm should stay a plain tilde.
```

## html

```html
<p>Ratio a // b and the rest.</p>
<p>Block comment attempt: a /* b */ c.</p>
<p>= Not a heading, just a sentence starting with an equals sign.</p>
<p>- Not a bullet, just a sentence starting with a hyphen.</p>
<p>+ Not an enum item, just a sentence starting with a plus.</p>
<p>/ term: not a term list, just prose starting with a slash.</p>
<p>A tilde example: 10~cm should stay a plain tilde.</p>
```
