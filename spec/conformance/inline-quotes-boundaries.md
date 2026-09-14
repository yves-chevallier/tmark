# Straight quotes, SmartyPants boundaries

Spec Appendix "PyMdownX compatibility profile", `"quotes"` row: a quote
opens when it starts the run or follows whitespace or an opening bracket
and is followed by a non-space; it closes when it follows a non-space and
is followed by the end, whitespace or punctuation. A phrase that straddles
inline markup stays literal, and its closing quote never opens the next
pair: the plain phrase after it is still a `Quoted`. An inch mark (`5"`)
opens nothing.

## input

```md
Quotes: "one *two* three" and "plain" and 'single' and "unclosed here.

A 5" screen, a ("bracketed") one, and "last".
```

## canonical

```md
Quotes: "one *two* three" and "plain" and 'single' and "unclosed here.

A 5" screen, a ("bracketed") one, and "last".
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
          "text": "Quotes: \"one "
        },
        {
          "type": "Emph",
          "content": [
            {
              "type": "Str",
              "text": "two"
            }
          ]
        },
        {
          "type": "Str",
          "text": " three\" and "
        },
        {
          "type": "Quoted",
          "kind": "double",
          "content": [
            {
              "type": "Str",
              "text": "plain"
            }
          ]
        },
        {
          "type": "Str",
          "text": " and 'single' and \"unclosed here."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "A 5\" screen, a ("
        },
        {
          "type": "Quoted",
          "kind": "double",
          "content": [
            {
              "type": "Str",
              "text": "bracketed"
            }
          ]
        },
        {
          "type": "Str",
          "text": ") one, and "
        },
        {
          "type": "Quoted",
          "kind": "double",
          "content": [
            {
              "type": "Str",
              "text": "last"
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

## latex

```latex
Quotes: "one \emph{two} three" and \enquote{plain} and 'single' and "unclosed here.

A 5" screen, a (\enquote{bracketed}) one, and \enquote{last}.
```

## html

```html
<p>Quotes: &quot;one <em>two</em> three&quot; and “plain” and 'single' and &quot;unclosed here.</p>
<p>A 5&quot; screen, a (“bracketed”) one, and “last”.</p>
```
