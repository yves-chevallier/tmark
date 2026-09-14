# Progress bar, escaped

Spec §ProgressBar, §Round-trip and source spans: a backslash before the
`[` of a progress bar spelling keeps the whole spelling literal, its quoted
label included, and the printer writes the backslash back. The input is
not a canonical block, so the fixed-point test alone never sees an escaped
spelling; the printer's `input_blocks_round_trip` test does.

## input

```md
Write \[=45% "Review"] for a bar, \[=100%] for a full one; [=x%] is text.
```

## canonical

```md
Write \[=45% \"Review"] for a bar, \[=100%] for a full one; [=x%] is text.
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
          "text": "Write [=45% \"Review\"] for a bar, [=100%] for a full one; [=x%] is text."
        }
      ]
    }
  ]
}
```
