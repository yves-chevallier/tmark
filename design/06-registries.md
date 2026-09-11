# 06 — Registries and resolution

Crate: `tmark-registry`. Input: `&Document`, `&dyn Loader`. Output:
`Resolved` plus diagnostics. Spec: §Registries, §Anchors, references,
citations, §Cross-document references, §Counters, §Bibliography, §Glossary and
acronyms, §Index.

## The Loader seam

```rust
pub trait Loader {
    /// Text of a path relative to `from`; `None` if it does not exist.
    fn load(&self, from: &Path, rel: &str) -> Option<String>;
}
```

The only I/O abstraction in the core. `MemoryLoader` (a map, for tests and
WASM) and `FsLoader` (CLI, LSP, PyO3). No caching in the trait: callers cache.
No async: the LSP calls it on its worker thread.

What goes through it: included TMark files (to collect their labels and
counter items for numbering), `.bib` files, `refs.json` inventories, glossary
files if the front matter points to one. What never goes through it: DOI
resolution, image files, executed fences. Those are TeXSmith passes that run
*before* resolution and hand back a document with the results spliced in.

## Registries

```rust
pub struct Resolved {
    pub counters: Counters,       // prefix → declaration + allocated numbers
    pub labels: Labels,           // id → (NodeId, prefix, number, kind)
    pub bibliography: Bib,        // key → entry (fields as parsed; no formatting here)
    pub glossary: Glossary,       // term → entry
    pub index: IndexTable,        // registry name → entries with NodeIds
    pub crossrefs: CrossRefs,     // alias → inventory
    pub refs: Vec<RefResolution>, // for every Ref node: what it resolved to, or why not
    pub diagnostics: Vec<Diagnostic>,
}
```

Each registry is a plain struct with a `HashMap`; each is built by one
function in its own module. `Resolved` is immutable once built.

## Resolution algorithm

Spec §Registries (lookup): a `@key` whose head is a declared prefix is a
label or counter reference; `gls:` is a glossary reference; `doi:` a DOI
citation; anything else a bibliography key. A key present in two registries
is `ref-ambiguous`.

1. **Declare.** Merge the predeclared prefixes (`tmark_ir::registry::PREFIXES`)
   with `declare.counters` from the front matter; a user entry overrides
   fields of a predeclared one but cannot shadow a role name. Same for
   admonition types (theorem kinds carry a counter).
2. **Collect definitions**, in document order, across the document and its
   includes (through `Loader`, depth-first, cycle-guarded): every `#id`
   attribute on a host, every `CounterItem`, every `Caption` with an id, every
   `IndexEntry`, every footnote definition. The host decides the counter
   (heading → section, `Table:` → `tbl`, image → `fig`, code block → `lst`,
   `MathBlock` → `eq`, theorem admonition → its declared counter); an explicit
   prefix must agree with the host or `prefix-host-mismatch` is raised.
   Duplicates raise `label-duplicate` with the other span as `related`.
3. **Allocate numbers** for TeXSmith-numbered series only (user counters,
   `thm` when TeXSmith numbers it). Backend-numbered series (`sec`, `fig`,
   `tbl`, `lst`, `eq`) get *no* number here: the backend numbers them. The
   registry records the kind and the order, which is enough for labels,
   diagnostics, and the `mkdocs` companion that numbers on the web.
   Multi-document numbering (a series continuing across files of a book) is
   an input: `ResolveOptions { start: HashMap<Prefix, u32> }`, set by
   TeXSmith from the previous document's inventory.
4. **Load sources.** `.bib` files named in `sources.bibliography` (parsed
   with the `biblatex` crate; entries kept as fields, no CSL here), inline
   pybtex-shaped entries, DOI shorthands recorded as *pending* (TeXSmith
   fetches them). `sources.crossrefs` inventories (`refs.json`, format below).
   Glossary and acronym declarations.
5. **Resolve every `Ref`** by the lookup rule, honouring the X4 guard already
   applied by the parser and the capitalised-prefix convention (`@Fig:x`).
   Result per ref: `Label { node, prefix, number? }`, `Citation { key }`,
   `Glossary { term }`, `Doi { doi }`, `External { alias, label, page? }`, or
   `Unresolved { reason }`. Unresolved refs render visibly as `[?key]`
   downstream (spec P4); the registry only records the fact.
6. **Footnote-versus-citation shadowing** (deprecated `[^key]` citations): a
   real footnote definition wins; a `[^key]` without definition whose key is
   in the bibliography becomes a citation with a `deprecated` diagnostic.

## Inventory format (`refs.json`)

Published by TeXSmith after a build (page numbers need the backend), read
here for cross-document references:

```json
{ "document": { "id": "RHE-423", "title": "Firmware review", "source": "review.md", "hash": "…" },
  "refs": { "fw:pas-de-temps": { "label": "FW-10", "page": 14, "kind": "counter", "prefix": "fw" } } }
```

`tmark-registry` owns the *reader* and the type; TeXSmith owns the writer,
generated from the same `schemars` schema. A stale `hash` raises
`crossref-inventory-stale`.

## What the LSP gets from this

Completion after `@` (every label, key and term with its kind), go-to-definition
from a `Ref` to its host node, hover with the label word and the number when
known, rename of a label (a `NodeEdit` on the definition plus one per
reference), document links for includes.

## Non-goals

- Formatting citations (CSL) or labels: writers and TeXSmith.
- Fetching anything.
- Persisting registries between runs: the LSP keeps `Resolved` in memory per
  document; TeXSmith keeps inventories on disk.

## Implementation notes (milestone 2)

- `tmark-registry` exposes `resolve(doc, loader, options) -> Resolved`;
  `ResolveOptions` carries the document path (includes and sources resolve
  against its directory), the `.bib` paths and the first values of series
  that continue across documents.
- `FsLoader` lives in this crate behind the default `fs` feature (off in
  WASM) rather than in each edge crate: one implementation, three users.
- Hosts and prefixes: a heading accepts any heading-class prefix (`part`,
  `chap`, `sec`, `app`); a predeclared prefix on the wrong host is
  `prefix-host-mismatch`, a user series on any host numbers it in that
  series (spec §Anchor). Image and span nodes span their attribute list, so
  a label diagnostic covers `![…](…){#id}` as a whole.
- Numbers are allocated for the TeXSmith-numbered series only (declared
  counters and theorem kinds with a counter of their own); `Counter::label`
  applies the Python-style `format` (`{n:02d}`, `{prefix}`, `{key}`).
- A key present in a label registry and in the bibliography resolves to
  `Resolution::Ambiguous` with a `ref-ambiguous` diagnostic, not to
  `Unresolved`, so that it is not reported twice.
- Anchor links (`[text](#id)`) are resolved like references and appear in
  `Resolved.refs`.
- Inventories are read from `sources.crossrefs`; a missing or invalid file
  is `crossref-inventory-missing`. The staleness check (`hash`) waits for
  the writer side in TeXSmith, which fixes the hash algorithm.
- Footnote-versus-citation shadowing is not implemented: the GFM footnote
  construct only forms a footnote reference when a definition exists, so a
  `[^key]` citation never reaches the IR as a `Note`. The deprecated sugar
  needs its own tokenizer rule if it is ever wanted (milestone 5, or never).
- Glossary terms come from `declare.glossary`, `declare.acronyms` (term to
  string or object with `name`/`description`) and the `*[KEY]: …` lines.

## Implementation notes (milestone 3)

- References inside included files resolve too: the collector keeps the
  parsed included documents (`Resolved.included`) and `resolve_all` runs
  on each after the main document, so `ref-unresolved` is reported with
  the included file's `FileId` and the language server publishes it under
  that file. `RefResolution.span` is the key token (`RefItem.key_span`);
  `Label.id_span` is the id token.
- Heading-class prefixes carry `Prefix::heading` in the registry instead
  of a list in the collector.
