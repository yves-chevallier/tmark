# Smart symbols, escaped

Spec Appendix "PyMdownX compatibility profile", §Round-trip and source
spans: a backslash on any character of a smart-symbol spelling keeps the
whole spelling literal (`\(c)`, `1\/2`, `\<-->` is not `<` and an arrow),
and the printer escapes the first punctuation character of every spelling
a `Str` holds, so that `fmt` is a fixed point and the next parse reads the
same text. An entity earlier on the line does not move the escapes.

## input

```md
Copyright \(c) 2025, tea \(tm), reg \(r), c\/o Ada: 1\/2 a cup, \+/- 3, 4 \=/= 5.

Arrows a \--> b, c \<-- d, e \<--> f; &copy; then (c) is still a symbol, \(c) is not.
```

## canonical

```md
Copyright \(c) 2025, tea \(tm), reg \(r), c\/o Ada: 1\/2 a cup, \+/- 3, 4 \=/= 5.

Arrows a \--> b, c \<-- d, e \<--> f; © then © is still a symbol, \(c) is not.
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
          "text": "Copyright (c) 2025, tea (tm), reg (r), c/o Ada: 1/2 a cup, +/- 3, 4 =/= 5."
        }
      ]
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "Arrows a --> b, c <-- d, e <--> f; © then © is still a symbol, (c) is not."
        }
      ]
    }
  ]
}
```
