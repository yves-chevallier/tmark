# Table caption line

Spec §Caption: the caption is the paragraph adjacent to the float; canonical
position is after the block, the line before the table is accepted sugar
(spec challenge C7 for the attachment rule). The anchor lives on the caption.

## input

```md
| A | B |
| - | - |
| 1 | 2 |

Table: Stock. {#tbl:stock}
```

```md
Table: Stock. {#tbl:stock}

| A | B |
| - | - |
| 1 | 2 |
```

## canonical

```md
| A | B |
| - | - |
| 1 | 2 |

Table: Stock. {#tbl:stock}
```

## ir

```json
{
  "type": "Document",
  "blocks": [
    { "type": "Table", "attrs": {}, "model": {
      "columns": [ { "name": "A" }, { "name": "B" } ],
      "rows": [ { "cells": [ "1", "2" ] } ]
    } },
    { "type": "Caption", "kind": "Table", "position": "After",
      "attrs": { "id": "tbl:stock" },
      "content": [ { "type": "Str", "text": "Stock." } ] }
  ]
}
```
