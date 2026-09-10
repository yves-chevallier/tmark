# 04 — IR review for milestone 3

Mandate 4 of `design/13-handoff.md`, read against what the language server
of `design/08-lsp.md` concretely needs: diagnostics, document symbols,
folding, semantic tokens, completion with replacement ranges,
go-to-definition, references, rename, code actions, hover. Reviewed at
`main` `f3443a6`. Nothing under `crates/` was modified; throwaway tests and
a micro-benchmark live in the session scratchpad (`scratchpad/ir/lineidx`,
`scratchpad/ir/walkbench`, `scratchpad/ir/ids.md`), their results are
quoted below.

## Summary

Three fields, one function, one bug fix. Everything else the LSP needs is
either already there or is recoverable from the node span plus the source
text with a few lines of code that belong in `tmark-lsp`.

| # | Change | Needed by | Breaks | Files |
| - | ------ | --------- | ------ | ----- |
| C1 | `RefItem.key_span: Span` | completion replacement range, rename, references, precise `ref-unresolved` | schema, 3 struct-literal sites, 2 fixture diagnostic lines | 6 |
| C2 | `Attrs.id_span: Option<Span>` | rename of a label at its definition, go-to-definition selection range, prepareRename | schema, 2 struct-literal sites, parser attribute sites | 6 |
| C3 | `CounterItem.key_span: Span` | rename of a counter key at its definition | schema, 2 struct-literal sites | 3 |
| C4 | `tmark_ir::structural_json(&Document)` replaces the three `SUGAR_FIELDS` lists; strips `span`, node `id`, every `*_span`, the sugar fields | C1–C3 without touching 38 fixtures; SSOT | conformance runner, round-trip test, `dump` example, `fixture-ir.py` | 5 |
| C5 | `LineIndex::to_utf16` must not underflow on a mid-character byte column; `LineIndex::offset` must clamp before the line terminator | server robustness (a diagnostic span off a char boundary kills the server in debug builds) | nothing | 1 |

Not changed: `Meta::eq`, generated images as `Image` attributes, asides as
`Para([Aside])`, `DefinitionList.items` as a tuple, no `map`, no node index.
Reasons in §What not to change.

## 1. `Meta::eq` always true

`crates/tmark-ir/src/node.rs:50-54` makes `Meta == Meta` unconditionally
true so that the derived `PartialEq` of every node is structural (design
rule 3). `Meta` derives neither `Eq` nor `Hash` (`node.rs:32`), so a
`HashSet<Block>` or `HashMap<Inline, _>` does not compile: the surprise the
handoff fears cannot happen silently.

Facts checked:

- No crate keys a map by a node or a `Meta`. Every `HashMap`/`BTreeMap` in
  the workspace is keyed by `String`, `Uri`, `PathBuf` or `Code`
  (`grep -rn "HashMap\|BTreeMap" crates`), and the two `HashSet`s hold
  `PathBuf`s (`crates/tmark-registry/src/collect.rs:83`).
- `Diagnostic`, `Label`, `RefResolution` carry a `Span`, not a `Meta`, and
  `Span` derives real equality (`span.rs:51-53`): comparing diagnostics
  compares positions, as it should.
- Node-id lookup today is a full pre-order walk: `tmark_fmt::edit::span_of`
  (`crates/tmark-fmt/src/edit.rs:25-33`). Measured in release on
  `spec/tmark.md` (75 KB, 3824 nodes): one full `walk` takes 14 µs; the
  parse of the same file takes 16.6 ms. On `design/03-ir.md` (11 KB, 763
  nodes): 2.4 µs. A lookup is three orders of magnitude below the parse
  and below any LSP round-trip; no index is justified.
- The `NodeId` doc says "dense, document-ordered" (`span.rs:29-30`,
  `design/03-ir.md` §Identity). The lowerer does not deliver this: in
  `scratchpad/ir/ids.md` the inline aside's `Plain` gets id 13 after its
  children 10–12 (`crates/tmark-syntax/src/lower/inline.rs:609-621`
  allocates `plain_meta` after lowering the content), the `::: aside`
  `Aside` gets id 20 after its children 16–19
  (`lower/block.rs:566-580`), the `DefinitionList` gets id 43 after its
  items 22–24, and id 21 is never used. Nothing relies on the order (a
  binary search by id would, and none exists). Either fix the three
  allocation sites in `tmark-syntax` or weaken the doc to "unique; new
  ids above the maximum". Not an IR change; flagged for mandate 1.

Verdict: keep `Meta::eq` as is. If C1–C3 add `Span` fields outside `Meta`,
those fields must keep the same rule (see §Proposed changes), otherwise
`parse(format(doc)) == doc` would start comparing positions.

## 2. Generated images as `Image` attributes

`lower/block.rs:483-497` lowers `python image` and bare `mermaid` fences to
`Para([Image { src: "", attrs: [generate=<lang>, code=<fence text>, …] }])`;
`crates/tmark-fmt/src/block.rs:337-359` prints them back. What the LSP
does with them:

- Folding: `outline.rs:266-286` folds `CodeBlock` but the generated image
  is a `Para`, so a 30-line Python fence does not fold. Three lines in
  `outline.rs` (match `Para` whose only inline is an `Image` with
  `generate`) fix it without an IR change.
- Semantic tokens, hover: the node span is the fence; nothing else needed.
- Completion of fence options: text-driven (the user is inside an info
  string, there is no node for a half-typed fence).
- Labels: `collect.rs:171` defines `Image.attrs.id()` as a figure label,
  but `parse_fence_info` (`lower/head.rs:199-232`) has no `#id` handling:
  in the scratch sample, ```` ```python image title="Plot" {#fig:p} ````
  loses `{#fig:p}` into `info.rest` (dump shows `kv: [generate, code,
  title]`, no id). Likewise `collect.rs:157` reads
  `CodeBlock.options.id()`, which the parser never sets. The spec's
  example at `spec/tmark.md:1003` shows no attribute list on the fence, so
  whether fence attributes exist at all is a mandate-2 question, not an IR
  one.
- Risk of the attribute encoding for the LSP: none. `Attrs.kv` keeps
  duplicate keys and `get` returns the first (`attrs.rs:22-25`,
  `48-54`), so a user option named `code` cannot shadow the fence text.

Verdict: no change for M3. The writers (M4) are the first consumer that
must branch on `generate`; they decide whether `Image` grows a
`generated: Option<Generated { lang, code }>` field. Note for that agent:
today the code text sits in an attribute value, which the printer quotes
and escapes (`block.rs:356`) — the round-trip test covers it, a writer
will want the raw field.

## 3. Asides as `Para([Aside])`

`lower/block.rs:566-580` wraps a `::: aside` container into a `Para` with
one `Aside` inline whose `content` are the blocks; `lower/inline.rs:609-621`
wraps an inline `{aside}[…]` into `Aside { content: [Plain(inlines)] }`.
Both share one node type, which the printer splits back
(`fmt/block.rs:71-83`).

LSP consequences:

- Outline and folding: a block aside is neither a `Div` nor an
  `Admonition`, so `outline.rs` does not list or fold it. Same three-line
  fix as for generated images, in `outline.rs`.
- Node-at-offset: `Para`, `Aside` and `Plain` share one span (ids 9 and 13
  in the sample, span `[65, 92]`). The ancestor chain is longer by one or
  two nodes; harmless.
- Hover on the role name: the LSP checks the text at `span.start` (`{`
  for the role, `:::` for the container) — no field records which form
  was used, and none is needed.

Verdict: no change for M3. Whether an empty paragraph hosting a margin
note is what the LaTeX and Typst writers want (`\marginpar` in its own
paragraph produces a blank line) is a M4 question; leave it to the writer
that first hits it.

## 4. `DefinitionList.items` as a tuple

`node.rs:683-691`: `Vec<(Vec<Inline>, Vec<Vec<Block>>)>`. Users: the
lowerer (`lower/block.rs:630-648`, 3 sites), the printer
(`fmt/block.rs:159-160`), `walk.rs:105-112`, one fixture
(`spec/conformance/definition-list.md`).

The LSP does not read definition lists beyond folding the whole block
(`outline.rs:276`). A term has no span of its own; if the outline ever
lists terms, joining the term's inline spans gives one. The tuple
serialises as a JSON pair `[[inlines], [[blocks]]]` and `schemars`
describes it as a fixed-length array, which is the only real cost: a
Python reader gets positional access instead of `item["term"]`.

Verdict: no change for M3. A `DefinitionItem { term, definitions }` struct
is a five-file change (node, walk, fmt/block, syntax/block, the fixture's
`ir` block, plus schema) and pays off when `tmark-py` or a writer reads
definition lists; do it then.

## 5. Sub-spans: inventory

The design (`03-ir.md` §Identity and spans) promises "sub-spans that tools
need … are stored as fields of the node"; `03-ir.md` §Implementation notes
defers them to M3. For each sub-span an LSP feature wants, the table says
whether the node span plus the source text (`Doc.text` in
`crates/tmark-lsp/src/lib.rs:77`) recovers it, and the verdict.

| Sub-span | Feature | Recoverable by re-scan? | Verdict |
| -------- | ------- | ----------------------- | ------- |
| key of a `RefItem` in `@key`, `@[see fig:a; -ein05, p. 3]`, Pandoc `[@a; @b]` | completion replacement range (handoff step 5), rename, references, `ref-unresolved` on the key not the group | Only by re-running the item grammar (`lower/head.rs:145-185`: split on `;`, `,`, whitespace, strip `-`, fallback "whole item is the key"), over a string from which the lowerer has already removed `@` (`lower/inline.rs:230`), after `doi_key` rewrote `https://doi.org/…` keys (`inline.rs:839-846`). Re-implementing this in the LSP duplicates a grammar. | **Store** (C1) |
| `#id` of an attribute list | rename at the definition site, `LocationLink.targetSelectionRange`, prepareRename, references (the definition occurrence) | Host-dependent: last brace group on the line for `Header`/`Caption`/`MathBlock`; brace adjacent to the span end for `Image`/`Span`; on the *first* line for containers (`::: figure {#fig:x}`); on the *last* line inside `> ` for `BlockQuote` (`lower/block.rs:148-160`); never for `CodeBlock`/`Table`. Six rules that mirror the parser. | **Store** (C2) |
| key of `#(fw:key)` / `{counter}(fw:key)` | rename of a counter key at its definition; `Label.id_span` for counter-item labels | Yes: the span ends with `(prefix:key)`, keys are identifiers, no escapes. But `collect.rs` has no text and builds `Label`s for counter items (`collect.rs:173-192`), and the LSP wants one `Label.id_span` whatever the host. | **Store** (C3), for uniformity with C2 |
| role name in `{role …}` | hover on a role, semantic token for the head | Yes: `text[span.start..]` starts with `{` then an identifier. Roles lower to typed nodes (`sc` → `SmallCaps`) with no record of the form, so the LSP checks the first byte anyway. | Re-scan in `tmark-lsp` |
| label of `[^1]` / `[^1]:` | go-to-definition (target: `Footnote.meta.span`), references | Yes: `[^` … `]` at the span start (`Note.label` gives the text). Rename of footnote labels is not in `08-lsp.md`. | Re-scan |
| key of `*[HTML]: …` | go-to-definition of an `Abbr` | The span is wrong today: every `AbbrDef` of a paragraph gets the paragraph span (`lower/block.rs:48-58` allocates `meta(span)` inside the loop; the dump shows both defs at `[170, 206]`). Once each def has its line, `*[` … `]:` is trivial. | Parser fix (mandate 1), then re-scan |
| include path `{include}(path)` | document links, go-to-file | Yes: last `(` … `)` of the span. A document link may also cover the whole node; the spec of `DocumentLink.range` allows it. | Re-scan or whole span |
| image `src`, link `(other.md)` | document links | Reference-style images/links keep no target text in the span (the definition line is dropped, `lower/block.rs:231`). Whole-node range is the only uniform choice. | Whole span |
| `Target::Anchor` of `[text](#id)` | go-to-definition, references | Node span suffices for both; rename of an anchor written as `[text][ref]` cannot be done from the IR at all (the definition is not in the tree). | Whole span; document the limitation |
| label inside a caption line | rename, definition | It is the caption's `{#id}`: covered by C2. | C2 |
| front-matter key/value | hover from the schema, completion | `FrontMatter.raw` plus `meta.span` (`frontmatter.rs:22-37`); YAML positions are a `serde_yaml_ng` question, outside the IR. | Re-scan `raw` |
| `Var` path `{{ press.x }}` | completion of paths | Node span; the path is the whole content. | Re-scan |

A consequence worth stating: with C1–C3, **rename does not need `edit()`
at all.** Every occurrence is a `(Span, String)` text edit grouped into a
`WorkspaceEdit`; the client applies them atomically. Going through
`tmark_fmt::edit` for a rename would reprint the whole host node in
canonical form — renaming the id of a `::: figure` would rewrite forty
lines of its body — and would turn `#(fw:x)` into `{counter}(fw:x)` and
`@[see x]` into whatever the printer chooses, which is not what a rename
means. `edit()` remains right for code actions that replace one node
(deprecated spelling → canonical), where reprinting is the point. Two
things about `edit` for that use, outside this crate but found here:
`edit()` returns a spliced `String` (`edit.rs:49-62`) while the LSP wants
`(Span, String)`; `span_of` and `print` are `pub` but the module is
private, so `tmark_fmt` exposes no way to get a `TextEdit` without
diffing. Re-export them or add `pub fn text_edit(doc, NodeEdit) ->
Option<(Span, String)>` (one file, `crates/tmark-fmt/src/lib.rs:19`).
Applying two `NodeEdit`s in sequence with `edit()` is also wrong today:
the second edit's spans refer to the original text.

## 6. `walk` without `map`; node at offset and ancestors

`walk.rs:35-40` is a pre-order visitor over blocks, inlines, table cells,
captions, admonition titles, image alt text, index paths and footnote
bodies. It does not visit `Document.abbreviations` (no inlines there; the
LSP reads the vector directly) and skips `Row::Separator` rows, whose
labels are strings. No `map`, no parent pointers, no index.

What the LSP needs: the innermost node under the cursor and its ancestors
(hover, definition, completion context, prepareRename). With spans that
nest, "every node whose span contains `offset`, in pre-order" *is* the
ancestor chain, outermost first. Measured with a full walk on
`spec/tmark.md`: 16 µs per query (chain of 2 at the middle of the file),
2.8 µs on a design document. A parse of the same file costs 16.6 ms and the
LSP clones the `Document` for the worker on every change
(`lib.rs:220`, 178 µs). Any index would cost more to build than it saves.

Two caveats for the implementer, both visible in the sample dump:

- Spans of siblings can be equal or overlap when the lowerer falls back to
  literal text (`lower/inline.rs:267-272` gives both bracket `Str`s of an
  unclaimed group the group's span before `merge_strs` joins them). The
  chain may then contain two siblings; take the last node of the chain as
  "innermost" and it still works.
- A `Para([Aside([Plain])])` yields three nodes with one span (see §3).

Verdict: no `map` (still no user), no index. Add one function next to
`walk` (or in `tmark-lsp` if the reviewer of the architecture prefers the
IR crate to stay traversal-only): `pub fn nodes_at(doc, offset) ->
Vec<NodeRef>`; and, since `tmark-fmt::span_of` (`edit.rs:25`), the
registry's `Resolution::Label { target: NodeId }` (`refs.rs:12`, hover
wants the target's caption text) and the LSP all look up by id, `pub fn
find(doc, id) -> Option<NodeRef>`. Both are twelve-line walks; the second
has three users today, which is the bar `AGENTS.md` sets.

## 7. `SUGAR_FIELDS` in three places

`crates/tmark/tests/conformance.rs:95`, `crates/tmark-fmt/tests/roundtrip.rs:101`
and `scripts/fixture-ir.py:21` each define `{"position", "bracketed"}` and
each re-implement "strip `span`, strip `id` next to a `span`, strip sugar
fields, drop defaults" with small differences (the runner and the script
drop defaults, the round-trip test does not; the script treats `None` as
absent). C1–C3 add fields that all three must also strip, which is the
moment to stop copying.

Proposal (C4): one function in `tmark-ir`, next to `eq_with_spans`
(`node.rs:75`), since it is the JSON twin of that comparison:

```rust
/// The document as JSON without identity and spelling: node ids, spans
/// (`span` and every `*_span` field), and the fields that record which
/// sugar was written (`Caption.position`, `Ref.bracketed`); defaults and
/// empty containers dropped. This is what conformance fixtures store and
/// what the round-trip tests compare (design 03 §Serialization, spec
/// §Round-trip and source spans: "modulo source spans").
pub fn structural_json(doc: &Document) -> serde_json::Value
```

with the sugar list as `pub const SUGAR_FIELDS: &[&str]` beside it, one
doc line at `Caption.position` and `Ref.bracketed` pointing to it. Users:
`conformance.rs` and `roundtrip.rs` call it; the `dump` example
(`crates/tmark-syntax/examples/dump.rs`) gains a `--structural` flag and
`fixture-ir.py` calls dump with it and deletes its own `normalise`. The
Python side then has no list to keep in sync, which is the SSOT rule of
`AGENTS.md` ("Grammars, JSON schemas, Python stubs … are generated").
The `*_span` naming rule is what lets C1–C3 land without regenerating any
of the 38 fixtures' `ir` blocks.

## 8. `LineIndex` and UTF-16

`span.rs:169-211` builds line starts and a sorted list of non-ASCII
characters `(line, byte col, utf8 len)`; `to_utf16` (`258-271`) subtracts
`len − utf16_len` for every wide character before the column;
`from_utf16` (`273-299`) walks the same list forward. `utf16_len` is 2 for
a 4-byte sequence and 1 otherwise (`144-152`), which is exactly the
UTF-8/UTF-16 relation (BMP ↔ 1–3 bytes, supplementary ↔ 4 bytes).

Throwaway tests (`scratchpad/ir/lineidx/src/lib.rs`, 7 tests, all pass
except where noted):

- Surrogate pairs: `😀` counts 2 units; a UTF-16 column inside the pair
  snaps to the character start (`from_utf16`, `285-291`). Correct.
- Combining characters: `e` + U+0301 is two scalars, 3 bytes, 2 units;
  both directions agree. Correct — the LSP counts code units, not
  graphemes, so this is the required behaviour.
- CRLF: `ab\r\né😀c\r\n😀` gives 3 lines, `c` at byte col 6 / unit col 3,
  round-trips; a lone `\r` also starts a line. Correct. The `\n` of a
  `\r\n` is reported at column `len+1` of the previous line (`line_col(3)
  == (0, 3)`), acceptable for a span end.
- BOM: 3 bytes, 1 unit. Correct.
- **Bug (C5)**: `to_utf16` with a byte column *inside* a multibyte
  character panics in debug builds and wraps in release:
  `LineIndex::new("😀b").to_utf16(LineCol { line: 0, col: 1 })` →
  `attempt to subtract with overflow` at `span.rs:265`. Column 3 of the
  same text yields unit column 1 (past the emoji's start, wrong
  direction), and column 1 of `"éb"` yields 0. Every diagnostic span goes
  through this function (`convert.rs:14-21`). The parser produced no
  off-boundary span on `spec/tmark.md` (0 of 3824 nodes) and the lint
  rules compute offsets on ASCII words, so it is not reachable *today*;
  it is one bad span away from a dead server. Fix: when `w.col < pos.col
  < w.col + len`, return the column of `w` (snap to the character start),
  and use `saturating_sub` as belt and braces. Two lines.
- **Minor**: `offset()` (`242-250`) clamps a column past the end of the
  line to `line_start(line + 1)`, which is the first byte of the *next*
  line: `LineIndex::new("é\nb").offset(LineCol { line: 0, col: 6 }) == 3`,
  the `b`. The LSP says a character past the line length "defaults back
  to the line length"; the clamp should stop before the terminator
  (`\n`, `\r\n` or `\r`). Client editors send such positions for
  end-of-line completions and hovers.

`Document::line_index()` in `03-ir.md` §Identity does not exist;
`LineIndex::new(text)` is what everyone calls (`lib.rs:203`,
`dump.rs:12`). Doc drift, one line.

## Proposed changes (the minimal set)

Every `Span` field added outside `Meta` must be ignored by `PartialEq`, or
design rule 3 breaks and `parse(format(doc)) == doc` starts comparing
positions. Implement `PartialEq` by hand on the three structs (a dozen
lines in total); they keep `Eq` where they had it.

### C1 — `RefItem.key_span`

`crates/tmark-ir/src/node.rs:131-144`:

```rust
pub struct RefItem {
    pub prefix: Option<String>,
    pub suppress_author: bool,
    /// The key as written, prefix included (`fig:boot`, `ein05`, `Fig:x`).
    pub key: String,
    /// Source range of `key` alone, without `@`, `-`, prefix or suffix.
    /// Spec §Round-trip and source spans ("what an editor needs for …
    /// go-to-target"); design 03 §Identity and spans (sub-spans are
    /// fields). `Span::default()` when the item was built from JSON.
    #[serde(default)]
    pub key_span: Span,
    pub suffix: Option<String>,
}
```

Parser: `parse_ref_items` (`lower/head.rs:145`) takes the source slice
and its base offset and records the key's range instead of working on a
copy with `@` removed (`lower/inline.rs:230`); the bare form is
`span.start + 1 .. span.end` before `doi_key` rewriting. Registry:
`refs.rs:52` pushes `item.key_span` instead of `r.meta.span`, so
`RefResolution.span` becomes the key (`ref-unresolved` then underlines
the key, not the whole `@[…]` group; for a two-item group the two
diagnostics stop coinciding). Breaks: `schema/ir.json` regeneration; 3
struct-literal sites (`node.rs:924`, `inline.rs:234`, `head.rs:173`);
expected diagnostic positions in `spec/conformance/diag-ref-unresolved.md:109`
and `diag-include-missing.md:85` shift by one column (the `@`); nothing in
the `ir` blocks once C4 strips `*_span`. Files: `node.rs`, `head.rs`,
`inline.rs`, `refs.rs`, two fixtures, `ir.json`.

### C2 — `Attrs.id_span`

`crates/tmark-ir/src/attrs.rs:14-26`:

```rust
pub struct Attrs {
    /// `#id`: an anchor (spec §Anchor).
    pub id: Option<String>,
    /// Source range of the id token without its `#`, when the list was
    /// parsed from text (spec §Round-trip and source spans; design 03
    /// §Identity and spans). `None` when built by a pass or from JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_span: Option<Span>,
    pub classes: Vec<String>,
    pub kv: Vec<(String, String)>,
}
```

Only the id, not the whole list and not per-class or per-key spans:
rename, definition and references are the M3 features and they need the
id; completion inside a list is text-driven (there is no node for a
half-typed list). Parser: `parse_attrs` (`head.rs:62`) gains a base
offset; five call sites (`inline.rs:48` for adjacent and tail braces,
`block.rs:140` display math, `block.rs:784-794` block-quote trailing
attrs, `head.rs:235-254` container info, where the info string's offset
must be found on the fence's first line because `TmarkContainer` carries
only `info: String` and the container position, `mdast.rs:1648-1663`).
Registry: `Label` (`collect.rs:45-59`) gains `id_span: Option<Span>`,
filled from `attrs.id_span` in `define` and from C3 for counter items.
Breaks: schema; struct literals at `attrs.rs:71` and `node.rs:906`;
`fmt/attrs.rs` untouched (prints `id`). Files: `attrs.rs`, `head.rs`,
`inline.rs`, `block.rs`, `collect.rs`, `ir.json`.

### C3 — `CounterItem.key_span`

`node.rs:420-427`, same doc pattern as C1, filled from the
`TmarkArgument` position (`inline.rs:632`, `702`): the argument value is
`prefix:key`, so `key_span = arg.start + 1 + prefix.len() + 1 .. arg.end − 1`.
Breaks: schema, two struct literals. Files: `node.rs`, `inline.rs`,
`ir.json`.

### C4 — `structural_json` and `SUGAR_FIELDS` in `tmark-ir`

Described in §7. Files: `node.rs` (function and const), `conformance.rs`,
`roundtrip.rs`, `dump.rs`, `fixture-ir.py`. No fixture changes.

### C5 — `LineIndex` fixes

Described in §8: snap in `to_utf16` (`span.rs:261-266`), clamp before the
terminator in `offset` (`span.rs:248-249`); port the scratch tests
`byte_column_inside_a_multibyte_char_to_utf16` and
`utf16_column_past_line_end` into `span.rs`. Files: `span.rs`.

### Also, outside the IR but found by this review

- `tmark-syntax`: `AbbrDef` spans (all definitions of a paragraph share
  the paragraph span, `lower/block.rs:48-58`); id allocation order (§1);
  `#id` on fence info strings silently dropped (§2).
- `tmark-fmt`: expose a text-edit form of `edit` (§5); `edit()` is
  single-shot.
- `tmark-lsp`: fold and list `Para([Aside])` and generated images in
  `outline.rs`; put the re-scan helpers of §5 (role name, footnote label,
  include path, counter argument) in a private `subspan.rs` — five
  functions of three lines each, one user.

## What not to change, and why

- **`Meta::eq`**: coherent, compile-time safe (no `Eq`/`Hash`), no
  consumer hashes nodes, lookups cost microseconds. Changing it would
  force `eq_with_spans`-style comparisons into every test.
- **Generated images as attributes**: the LSP never branches on them
  beyond folding, which `outline.rs` can do. The writers decide the field.
- **`Para([Aside])`**: same; a writer-driven question.
- **`DefinitionList` tuple**: no LSP reader; a struct is cheap but lands
  with the first Python or writer consumer, so the fixture changes once.
- **`map`**: still no pass that rebuilds a document.
- **A node index / parent pointers / a CST**: ADR 0004 and the numbers in
  §6. A full walk is 14 µs on the largest document in the repository.
- **Spans for classes, keys, role keys, image `src`, link targets,
  footnote labels, include paths**: recoverable from the node span and the
  text with trivial scans, or served by the whole-node range
  (`DocumentLink`). Storing them now is the speculation `AGENTS.md`
  forbids; each becomes a one-field addition if a feature proves the need.
- **`Fix`** (`diagnostic.rs:202-205`): already a `(Span, String)` text
  edit, which is what code actions apply; nothing to add.
- **`Document.abbreviations` / `footnotes` as vectors** rather than maps:
  the registry builds the lookup it needs (`collect.rs:269-294`); the LSP
  does the same.
