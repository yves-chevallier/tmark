# 02 — Syntax: the parser

Crate: `tmark-syntax`. Input: `&str` and a `FileId`. Output: `Document` plus
`Vec<Diagnostic>`. It never fails and never performs I/O.

## Decision: vendor the micromark architecture (ADR 0002)

TMark is CommonMark plus constructs. The parser is built by vendoring
`markdown-rs` (MIT, a faithful port of micromark) into `tmark-syntax` and
adding constructs to it, rather than by wrapping it as a dependency or by
writing a CommonMark parser from scratch.

Why: micromark's design is *exactly* "a state-machine tokenizer where each
construct is a module" (`construct/*.rs`), positions are attached to every
event, GFM tables, footnotes, task lists, math and front matter already exist
as constructs, and CommonMark conformance comes with the vendored spec tests.
Post-processing a third-party AST is not an option: role content
(`{aside}[see **x**]`) interleaves with emphasis and links, and must be
tokenised in the same inline pass.

Cost accepted: a fork. CommonMark changes at glacial pace; upstream merges are
rare and mechanical. The vendored tree stays in `crates/tmark-syntax/src/md/`
with its license and a `VENDORED.md` naming the upstream commit.

## Two passes, as in micromark

1. **Block pass** (document → containers and flow): headings, paragraphs,
   lists, block quotes, fenced code, thematic breaks, HTML blocks, tables,
   footnote definitions, front matter, plus the TMark block constructs below.
2. **Inline pass** (per text chunk): attention (emphasis, strong), code spans,
   links, images, autolinks, escapes, entities, math, plus the TMark inline
   constructs below.

Events carry byte offsets. A third step, **lowering** (`lower.rs`), turns the
event stream into the IR of `03-ir.md`. Lowering is where most of TMark
happens: a great deal of the dialect is *reinterpretation* of CommonMark
events, not new tokenisation.

## TMark constructs, and where each is handled

Reference: spec §Four syntactic families, §Two sigils, §Lexical grammar.

| Construct | Spec regex | Pass | How |
| --------- | ---------- | ---- | --- |
| Attribute list `{#id .cls k=v}` | family 1 | inline | New construct `attributes`, recognised when `{` is followed by `#`, `.` or `ident=`. Attaches to the *preceding* host in lowering: heading (end of line), image, link, span `[…]`, or the caption line. Unattached list: literal text + diagnostic `attr-no-host`. |
| Role `{name …}[content]` / `(argument)` | family 2 | inline | New construct `role`: head, then one or more bracket groups tokenised as inline content, or one parenthesised verbatim argument with balanced parens. Name not in the registry: literal text (spec). |
| Anonymous span `[text]{attrs}` | family 1 | inline | No new tokenisation: a `[…]` that is not a link and is immediately followed by an attribute list lowers to `Span`. |
| Container `::: name {attrs}` … `:::` | family 3 | block | New container construct; nesting by fence length; content is flow. Unknown name: `Div` with `attrs.name`, diagnostic `container-unknown` (spec §Div: class D error). |
| Data directive ```` ```lang node opts ```` | family 4 | lowering | Fenced code exists; lowering parses the info string with the family-4 regex and produces `CodeBlock`, `Table`, `TableConfig`, `Image` or `RawBlock`. Bare `mermaid` → `Image` (spec exception). |
| Bare reference `@key` | sigil | inline | New construct `reference` with the X4 look-behind guard. Key grammar from the spec; `doi:` keys additionally accept `/` (spec challenge C3). |
| Bracketed reference `@[…]` | sigil | inline | Same construct; item grammar (prefix, `-`, key, suffix, `;`) parsed in lowering, not in the tokenizer. |
| Index entry `#[a][b]` | sigil | inline | New construct `define`, bracket groups are inline content. |
| Counter item `#(prefix:key)` | sigil | inline | Same construct, parenthesised argument. |
| Caption line `Kind: text {#id}` | §Caption | lowering | A paragraph whose text matches the caption regex lowers to `Caption`, attached to the adjacent float (after it canonically; before it as accepted sugar). |
| Lead-in `{lead}[…]` | §Para | inline/lowering | Role. The `paragraph.lead` feature promotes a leading short `Strong` in lowering. |
| Small caps `__x__` | X1 | lowering | Attention already tokenises `__`; lowering maps `__` strong to `SmallCaps` unless the profile disables X1. |
| `==x==`, `~x~`, `^x^`, `++k++`, `~~x~~` | §Inline | inline | New attention-like constructs with the PyMdownX rules (no space inside the delimiters). `~~` is Strikeout, `~` Subscript. |
| Definition list | §DefinitionList | block | New construct (PHP-Markdown-Extra rules). |
| Abbreviation `*[HTML]: …` | §Glossary | block | New construct, definition-only line; lowering records it in the document's abbreviation table and substitutes `Abbr` inlines. |
| `!!! type "Title"`, `??? type` | §Admonition | block | New construct; body is the following indented block. Lowers to `Admonition` (same node as `::: type`). |
| `/// name … ///` | deprecated | block | New construct, lowers like a container, diagnostic `deprecated`. |
| Footnote `[^1]`, `[^1]:` | §Note | block+inline | GFM footnotes construct (vendored). Deprecated citation use (`[^key]` with no definition but a bibliography key) is decided in resolution, not parsing. |
| Math `$…$`, `$$…$$`, `\(…\)`, `\[…\]` | §Math | inline/block | Vendored math construct plus the two LaTeX-habit delimiters. |
| Moustache `{{ key }}` | §Front matter | lowering | Text scan in lowering; produces `Var` inline. Not inside code. |
| Comment `<!-- -->` | §Comment | block/inline | HTML construct; lowering produces `Comment` for the comment form only, `RawInline`/`RawBlock` with `format = "html"` for other HTML. |
| Critic markup | Appendix | inline | Deferred to milestone 5 (compat profile). Until then literal. |
| Escapes `\@`, `\#` | §Lexical grammar | inline | Added to the escape construct's character set. |

### Rule of thumb

Add a tokenizer construct only when the syntax must interleave with other
inline or block tokenisation. If the construct can be recognised from a
finished paragraph, an info string or a text node, recognise it in lowering.
This keeps the fork small.

## Spans, files, identity

- Offsets are byte offsets into the file's text; line/column are computed on
  demand from a `LineIndex` (`tmark-ir::span`).
- Each node gets a `NodeId` (u32, dense, per document) assigned in lowering in
  document order. Ids are stable for a given text; a re-parse of an edited
  file renumbers. The LSP maps old→new ids by span when it needs continuity.
- A node produced from an included file (TeXSmith's include pass, not this
  crate) keeps the included file's `FileId`. The parser only ever sees one
  file.

## Error handling

The parser never returns an error. Three outcomes for unrecognised input:

1. Literal text (the CommonMark way): `{foo}` alone, an unknown role name, an
   `@` inside a word.
2. Literal text plus a diagnostic when the author clearly meant a construct:
   an attribute list with no host, `#[` with an unbalanced bracket, a role head
   followed by nothing, a container never closed (closed at end of document,
   diagnostic `container-unclosed`).
3. A node plus a diagnostic for accepted-but-deprecated spellings
   (`deprecated`, with the canonical replacement in the message).

Diagnostics from the parser are syntactic only; anything that needs a
registry (unknown prefix, unresolved key) is `tmark-registry`'s or
`tmark-lint`'s job.

## Front matter

The YAML island is tokenised by the vendored construct and handed *as text*
to `tmark-ir::frontmatter::parse`, which produces a `FrontMatter` value:
known keys typed, unknown keys kept as a JSON value under `extra` (TeXSmith
validates its own `press` keys). Parsing keeps the raw text so the printer can
copy it byte for byte (spec §Round-trip). A YAML error is a diagnostic; the
document still parses with an empty front matter.

## Performance targets

- 1 MB of prose: under 50 ms on a laptop core, single-threaded.
- No allocation per character; events are a `Vec<Event>` reused across
  parses where the caller keeps the parser.
- The LSP re-parses whole files. Incremental parsing is out of scope until a
  measurement says otherwise (YAGNI).

## What the conformance fixtures cover

Every row of the table above has at least one fixture under
`spec/conformance/` showing sugar → canonical → IR (format in
`spec/conformance/README.md`). The vendored CommonMark spec tests run
unchanged, except for the documented X-class deviations, which are listed by
example number in `crates/tmark-syntax/tests/commonmark_exceptions.rs`.
