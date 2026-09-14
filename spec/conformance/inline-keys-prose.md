# Keystroke sugar does not fire on prose

Spec §Inline text, challenge C51: the opening `++` must not be preceded by
a word character, the closing `++` must not be followed by one, and the
content between a pair must be non-empty with no whitespace. `C++03`,
`C++`, `i++` and `a ++ b` stay prose; `++ctrl+c++` still recognises as a
keystroke in the same paragraph.

## input

```md
Le code est écrit en C++03 alors que le C++ moderne utilise i++, jamais a ++ b, sauf pour ++ctrl+c++ qui reste un raccourci.
```

## canonical

```md
Le code est écrit en C\+\+03 alors que le C\+\+ moderne utilise i\+\+, jamais a \+\+ b, sauf pour {keys}[ctrl+c] qui reste un raccourci.
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
          "text": "Le code est écrit en C++03 alors que le C++ moderne utilise i++, jamais a ++ b, sauf pour "
        },
        {
          "type": "Keystroke",
          "keys": [
            "ctrl",
            "c"
          ]
        },
        {
          "type": "Str",
          "text": " qui reste un raccourci."
        }
      ]
    }
  ]
}
```

## latex

```latex
Le code est écrit en C++03 alors que le C++ moderne utilise i++, jamais a ++ b, sauf pour \tskeys{Ctrl,C} qui reste un raccourci.
```
