# Bare reference

Spec §Ref, §Cite, deviation X4. The X4 guard keeps `@` out of words, e-mails
and URLs; sentence punctuation stays out of the key. Resolution (label versus
citation) is the registry's job; the IR only records the items.

## input

```md
See @sec:intro and @ein05, not me@example.com.
```

## canonical

```md
See @sec:intro and @ein05, not me@example.com.
```

## ir

```json
{
  "type": "Document",
  "blocks": [
    { "type": "Para", "content": [
      { "type": "Str", "text": "See " },
      { "type": "Ref", "bracketed": false, "items": [ { "key": "sec:intro" } ] },
      { "type": "Str", "text": " and " },
      { "type": "Ref", "bracketed": false, "items": [ { "key": "ein05" } ] },
      { "type": "Str", "text": ", not me@example.com." }
    ] }
  ]
}
```
