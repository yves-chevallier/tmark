# ADR 0008 — Resolution requests instead of a mirrored IR

**Status:** **rejected as drafted.** Superseded by TeXSmith's
`specs/refactoring/07-synthesis.md`, which five independent analyses produced.

The load-bearing claim below — that TeXSmith stops holding a tree, so its IR
mirror goes — does not hold, and the sentence that breaks it is in this ADR:
*"A replacement is IR, serialised as the schema already defines it."* To answer
a patch the host must **construct** IR, so it keeps the node types, the field
names, the id space and the span rules. Measured: ~500-600 of the mirror's
generated lines survive, and a field added to `Image` or `Div` is still a
two-repository change. Three of the eleven passes cannot fit the shape at all
(`RefItem` carries no id, so citation items are unaddressable; the asset pass
inserts a *sibling* block; the script pass needs a whole-document text query).

What survives and should be taken up separately:

- **`sections` / `outline`** — a query, no patches, three users (`slots`,
  `title`, `headings`). It deletes the host's tree *walk*, not ten lines of
  partition. Worth its own small ADR.
- **Id and span custody.** `IdAllocator.floor` and the host's span rules are
  core invariants the host re-derives with nothing enforcing them. A span
  copied onto synthesised text is a location that exists and is wrong. Worth
  fixing narrowly, whatever else happens.
- **The rule this ADR needed and lacked:** a response carries an answer — a
  path, a key, a font name, a token stream, a typed failure — **never a tree**.
  If requests are ever revisited, that is the first line.

Before any of it: TeXSmith's mirror should simply be generated here and shipped
in the wheel. `pyproject.toml` already sets `python-source = "python"` and
`scripts/gen_stubs.py` already generates Python from Rust. That removes this
ADR's entire stated cost for about a day, with no new protocol.

The original text follows, unedited, as the record of what was proposed.

**Context.** TeXSmith's passes do the work the pure core cannot: read a file,
fetch a DOI, run a converter, ask Pygments to highlight, look up a font's
coverage. To do it they hold a Python mirror of the IR — 2 195 lines,
generated from this repository's schema and compared byte for byte in CI —
walk it, and hand the result back. The mirror is not maintenance TeXSmith
carries; it is a coupling. Every node field added here is a two-repository
change because the mirror must follow, and the mirror exists only so that a
pass can find the eleven kinds of node it acts on.

Eleven passes walk it. Ten of them replace one node with one node, one
subtree, or a short sequence: a `CodeBlock` becomes a highlighted `Div`, a
`Str` becomes a run of `Span{script}`, a `Var` becomes a `Str`, an `Include`
becomes the blocks of the included file, a `Header` is removed. The eleventh,
`slots`, partitions the top-level block list into named ranges and needs no
node at all — only the headers' levels, ids and texts, and the ranges between
them.

None of that requires the whole tree in another language. It requires the core
to say *what it needs resolved* and TeXSmith to say *what to put there*.

**Decision.** Three shapes, designed together:

    core → requests    [{node_id, kind, payload}]      what needs resolving
    host → patches     {node_id: replacement | drop}   what to put there
    core → sections    [{node_id, level, id, text,     the top-level outline
                         first_block, last_block}]

`kind` is a closed registry, like the role and counter-prefix registries
(spec P4, P6): `media`, `doi`, `link`, `font`, `highlight`, `script`, `var`,
`include`, `title`. A replacement is IR, serialised as the schema already
defines it; `drop` removes the node. Sections answer a query and are followed
by a write of a named block range, which is what a template slot is.

**Consequences.**

- The core keeps the tree. TeXSmith stops holding one, and `ir/` goes — but
  only when the *last* pass migrates. A partial migration leaves a
  request/patch contract and a tree walk running in parallel, which is a new
  defect, not a half-finished one. The contract is therefore designed for all
  eleven before any is implemented.
- `slots` decides the shape. It is the only pass the request/patch pair alone
  cannot express, which is why `sections` is in this ADR rather than a later
  one. A contract that serves `media` and not `slots` is the version this ADR
  replaces.
- The frontier does not move. TeXSmith still decides which template, what a
  slot is, where an include is looked up, which font covers a script; ADR 0001
  and the §Non-goals list are untouched. What changes is that it no longer
  needs a copy of the document to act on those decisions. No fourth trait:
  `Loader` is already a typed request/response across this boundary, and this
  generalises its shape.
- The bindings grow a surface that must round-trip every construct the passes
  touch. That is the cost, and it is real: it is a wider contract than
  `Loader`, and every `kind` is a conformance fixture.
