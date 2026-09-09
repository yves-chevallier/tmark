# 04 — Canonical printer and formatter

Crate: `tmark-fmt`. Input: `&Document`, a `Profile`. Output: `String`.
Spec: §Round-trip and source spans, the "Canonical" column of §Node
catalogue, Appendix "PyMdownX compatibility profile", Appendix "Deprecation
schedule".

## Contract

1. **Fixed point.** `parse(print(doc)) == doc` modulo spans and ids, for
   every document the parser produces. This is a property test, not a hope.
2. **Idempotent.** `print(parse(print(doc))) == print(doc)`.
3. **Deterministic.** No dependence on the environment, the clock, a hash
   seed. Attribute order, fence lengths, marker characters and caption
   position are normalised; everything else is preserved.
4. **Prose is not re-wrapped.** `SoftBreak` prints a newline. Line width is
   the author's.
5. **Front matter is copied byte for byte.** The printer emits
   `FrontMatter.raw` and never re-serialises YAML.
6. **Comments, raw passthroughs and escapes survive.** Text that looked like
   syntax but was not recognised is `Str` and prints as typed, re-escaped
   only where the canonical context requires it.

## What normalises

| Aspect | Canonical |
| ------ | --------- |
| Emphasis | `*x*`, `**x**` |
| Small caps, del, mark, sub, sup, keys, underline | roles: `{sc}[x]`, `{del}[x]`, `{mark}[x]`, `{sub}[x]`, `{sup}[x]`, `{keys}[ctrl+s]`, `{underline}[x]` |
| Headings | ATX `#` … `######`, attributes at end of line, `#id` first |
| Lists | `-` bullets, `1.` numbers with the source start, two-space nesting |
| Fences | backticks, length = longest inner backtick run + 1, minimum 3 |
| Containers | `:::`, nested fences longer by one per level |
| Callouts | `::: type {title="…"}`; `!!!` only under the `mkdocs` profile |
| Captions | after the block, `Kind: text {#id}` |
| References | `@key` bare; `@[…]` as soon as an item has a space or there are several |
| Index and counters | `{index}[…]`, `{counter}(prefix:key)` — the roles are canonical, `#[…]`/`#(…)` are sugar. (Spec challenge C6 asks whether the sigils should be canonical instead; until settled, the roles win because the printer must pick one.) |
| Raw | `{raw latex}(…)`, `latex raw` fences |
| Attributes | `{#id .class key=value}`; values quoted only when they contain whitespace or `}` |
| Math | `$…$`, `$$ … $$` on their own lines |
| Includes | `{include}(path)` alone on its line |
| Deprecated spellings | rewritten to their replacement (Appendix Deprecation schedule) |

## Profiles

```rust
pub enum Profile { Canonical, Strict, Mkdocs }
```

- `Canonical`: the table above.
- `Strict`: `Canonical` plus X1 and X2 rewritten to class-C forms
  (`{sc}[x]` stays a role; `---` becomes `{raw latex}(\clearpage)`? No: the
  strict profile keeps `---` as a divider and only *disables* the small-caps
  reading of `__x__`, so nothing to rewrite; the difference is in what the
  parser accepts). Rejects Appendix-PyMdownX sugar at parse time, which is a
  parser option, not a printer one. The printer difference between
  `Canonical` and `Strict` is therefore empty today; the variant exists so
  `tmark fmt --profile strict` can be a parse-with-strict-then-print.
- `Mkdocs`: emits PyMdownX spellings a site understands: `!!! type "Title"`
  for callouts, `??? type` for collapsed ones, `==x==` / `~~x~~` / `^x^` /
  `~x~` / `++keys++` / `` `#!py …` `` for the inline sugar, `--8<-- "file"`
  for includes, footnote-style citations where the site has no citation
  plugin. Class-X constructs that a site cannot render (`@key`, `#[…]`,
  `#(…)`, roles) are emitted as their canonical text: the site shows them
  literally, which is the spec's degradation contract, and TeXSmith's
  companion Python-Markdown extensions render them when installed.

Profiles are a *table* (construct → spelling function), not subclasses.

## Local edits

```rust
pub struct NodeEdit { pub id: NodeId, pub replacement: Node }
pub fn edit(text: &str, doc: &Document, edit: NodeEdit) -> String
```

Prints `replacement` in the context of its parent (block vs inline, current
indentation for nested blocks) and splices it into `doc[id].span`. Every
other byte is untouched. This is how the LSP renames a label, rewrites a
citation, converts sugar to canonical on a code action. A whole-document
`format` is the same operation applied to the root.

## Implementation notes

- One module per node family (`inline.rs`, `block.rs`, `attrs.rs`,
  `frontmatter.rs`), each a set of functions `fn para(w: &mut Out, node:
  &Para, ctx: &Ctx)`. No trait per node.
- `Out` tracks column and indentation for nested containers and list items;
  it is the only state.
- Escaping is contextual and minimal: `\@` and `\#` only when the following
  characters would otherwise form a reference or a definition; `\{` only
  before a role or attribute head; standard CommonMark escapes elsewhere.
  The fixed-point test catches every miss.
- The printer is also the reference for the spec: when a canonical spelling
  is ambiguous in prose, the fixture wins and the spec is amended
  (`12-spec-challenges.md`).

## Tests

- `proptest`: generate documents from the IR (a generator per node, bounded
  depth), assert the fixed-point and idempotence properties.
- Conformance fixtures: `canonical` block of every fixture must print back
  identically.
- Snapshot tests for the `Mkdocs` profile on the fixture corpus.
