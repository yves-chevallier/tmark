# Counter item

Spec §CounterItem: `{counter}(prefix:key)` canonical, `#(prefix:key)` sugar,
`#{prefix:key}` deprecated (challenge C16: recognised only for a declared
prefix). The prefix must be declared; `prefix-unknown` otherwise (resolve
stage, not shown here).

## input

```md
---
press:
  declare:
    counters:
      fw: {name: Finding, format: "FW-{n:02d}"}
---

#(fw:boot-loop) The firmware reboots when the watchdog fires.
```

```md
---
press:
  declare:
    counters:
      fw: {name: Finding, format: "FW-{n:02d}"}
---

{counter}(fw:boot-loop) The firmware reboots when the watchdog fires.
```

## canonical

```md
---
press:
  declare:
    counters:
      fw: {name: Finding, format: "FW-{n:02d}"}
---

{counter}(fw:boot-loop) The firmware reboots when the watchdog fires.
```

## ir

```json
{
  "type": "Document",
  "front_matter": { "keys": { "press": { "declare": { "counters": {
    "fw": { "name": "Finding", "format": "FW-{n:02d}" } } } } } },
  "blocks": [
    { "type": "Para", "content": [
      { "type": "CounterItem", "prefix": "fw", "key": "boot-loop" },
      { "type": "Str", "text": " The firmware reboots when the watchdog fires." }
    ] }
  ]
}
```
