# Dotted container name (a foreign directive)

Spec §Div, and TeXSmith `web-profile.md` open question 2: `::: pkg.mod`
is the directive of another tool (mkdocstrings), never closed. It is a
`Div` like any unknown container, closed at the end of the document, but
its `container-unknown` and `container-unclosed` diagnostics are `info`
rather than errors: the site renders the directive and nothing else can.
The web lowering keeps its bytes and still splices inside it (the `@`
reference after the options).

## input

```md
---
press:
  declare:
    counters:
      fw: {name: Finding, format: "FW-{n:02d}"}
---

#(fw:boot) A finding.

::: texsmith.core.counters
    options:
      show_source: false

Prose after the directive still refers to @fw:boot.
```

## canonical

````md
---
press:
  declare:
    counters:
      fw: {name: Finding, format: "FW-{n:02d}"}
---

{counter}(fw:boot) A finding.

::: texsmith.core.counters
```
options:
  show_source: false
```

Prose after the directive still refers to @fw:boot.
:::
````

## ir

```json
{
  "front_matter": {
    "raw": "---\npress:\n  declare:\n    counters:\n      fw: {name: Finding, format: \"FW-{n:02d}\"}\n---",
    "keys": {
      "press": {
        "declare": {
          "counters": {
            "fw": {
              "name": "Finding",
              "format": "FW-{n:02d}"
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
          "type": "CounterItem",
          "prefix": "fw",
          "key": "boot"
        },
        {
          "type": "Str",
          "text": " A finding."
        }
      ]
    },
    {
      "type": "Div",
      "name": "texsmith.core.counters",
      "content": [
        {
          "type": "CodeBlock",
          "text": "options:\n  show_source: false"
        },
        {
          "type": "Para",
          "content": [
            {
              "type": "Str",
              "text": "Prose after the directive still refers to "
            },
            {
              "type": "Ref",
              "items": [
                {
                  "key": "fw:boot"
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
  ]
}
```

## diagnostics

```text
container-unclosed @ 10:1-15:1
container-unknown @ 10:1-15:1
```
