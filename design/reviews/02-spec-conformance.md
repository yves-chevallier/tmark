# Review 02 — Spec conformance audit (handoff mandate 2)

Scope: `spec/tmark.md` draft 3, construct by construct, against the parser
(`tmark-syntax` over `tmark-markdown`), the printer (`tmark-fmt`), the
resolver (`tmark-registry`) and the lint rules, at commit `f3443a6`. Every
row below was verified with a run of `tmark parse` (IR, diagnostics),
`tmark fmt` (canonical, `--profile strict|mkdocs`) and `tmark check`
(resolve + lint). Scratch inputs live under
`/tmp/claude-1000/-home-ycr-tmark/2cbd51a1-bfb7-47e4-884e-62793ff9a596/scratchpad/spec-audit/`
(`attrs.md`, `roles.md`, `roles2.md`, `refs.md`, `inline.md`, `blocks.md`,
`containers.md`, `data.md`, `math.md`, `fm*.md`, `resolve.md`, `lint.md`,
`anchors.md`, `cap2-4.md`, `comments.md`, `gram.md`, `inc/`, `c16.md`).
Nothing was modified under `crates/`, `spec/` or `editors/`.

Status vocabulary: **implemented**, **partial**, **missing**, **deviates**
(the code does something the text forbids or contradicts), **spec bug
suspected** (the text is the problem). Deviations are numbered D1–D34 and
ranked at the end. Entries already recorded in `12-spec-challenges.md` or
in the "Implementation notes" of `02`–`06` are listed only when the run
showed something those notes do not say.

## Summary

- The core of the four families, the two sigils, references and citations,
  captions, containers, data directives, front matter precedence, the
  resolver's label/prefix/duplicate/ambiguity rules and the five lint rules
  behave as the spec says. `tmark fmt` is idempotent on `spec/tmark.md`
  and the round-trip IR of the spec is equal modulo spans and sugar fields.
- All known gaps in the mandate are confirmed: grid tables (a `CodeBlock`
  with `lang = "grid table"`), critic markup, progress bars, wiki links,
  inline footnotes `^[…]`, fancy list styles, `Space` never emitted,
  `Quoted` never produced, strict profile partial (and reachable only from
  `fmt`), `Mkdocs` a stub, footnote-versus-citation shadowing absent.
- Beyond those, 34 deviations were found. The ones that matter most for
  milestone 3: **anchors on spans and on admonitions without a counter are
  not labels** (D13: `[claim]{#claim:one}` then `@claim:one` is
  `ref-unresolved`, so go-to-definition and completion cannot see them);
  **links and fenced code blocks are not attribute hosts** (D1, D2: the
  id is dropped, once silently); **the canonical `{lead}[…]` role depends
  on the `paragraph.lead` switch** (D3); **`frontmatter-unknown-key`,
  `strict-x-construct`, `citation-shadowed-by-footnote` and
  `crossref-inventory-stale` are never emitted by any crate** (D16);
  **deprecated top-level front-matter groups are dropped, not merged, when
  the canonical group exists** (D17); **four rows of the deprecation
  schedule with horizon "fmt" produce neither a diagnostic nor the
  canonical form** (D5–D8); **unknown fence node words other than `raw` are
  dropped silently** (D10).
- Fixture coverage is below what `spec/conformance/README.md` claims: 12
  of 27 diagnostic codes and 10 rows of the deprecation schedule have no
  fixture (D33).

## §Four syntactic families

### Attributes

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| "Headings, images, links, fenced blocks, tables and caption lines are hosts" — links | deviates (D1) | `Link [x](http://e.com){.cls}` → `attr-no-host` at 7:23, IR `Link` then `Str "{.cls}"`, printed `[x](http://e.com)\{.cls}` | code: attach to the preceding `Link` in lowering; fixture `attributes-link.md` |
| same — fenced blocks | deviates (D2) | ```` ```python {#lst:attr} ```` → `CodeBlock` without attrs, **no diagnostic**, printed ```` ```python ````; ```` ```python {.cls #lst:q} ```` → `attr-no-host: malformed attribute list on the fence`, attrs dropped | code: `CodeBlock.attrs` from the info string; fixture `attributes-fence.md`. Spec: say where the list goes on an info string (`python {#id}` versus options) |
| same — headings, images, spans, caption lines | implemented | `## Title {#sec:a .cls k=v lang=fr media=print}`, `![alt](f.png){width=60% #fig:x}` (printed `{#fig:x width=60%}`), `[text]{#claim:one}`, `Table: x {#tbl:a .wide k=v}` all carry `attrs` | — |
| "There are no bare-word attributes: `{collapsed}` is not an attribute list" | implemented | `{collapsed}` → `Str`, no diagnostic (printer writes `\{collapsed}`) | — |
| Attribute order normalised "`#id`, then `.class`, then keys in source order" (§Round-trip) | implemented | `{title="…" #thm:pythagoras}` printed `{#thm:pythagoras title="…"}` | — |
| Universal `lang=`, `media=` "accepted on every host" | implemented (stored as plain `kv`) | `[this taylor]{lang=en}` → `kv [["lang","en"]]`; no validation of `media` values (`media=foo` accepted) | none now; a `media` value check is a lint candidate |
| Block quote attribute `{.epigraph}` (§BlockQuote) | partial (D26) | `> Quote\n> {.epigraph}` → `BlockQuote.attrs.classes=["epigraph"]` **and** `attr-no-host` at 2:3 (spurious) | code: suppress the diagnostic when the list was consumed; fixture `attributes-blockquote.md` |
| Zero-width nodes: "whitespace on both sides collapses" | not in IR (C8, documented) | `Je suis un chien {aside}[remarque].` → `Str "Je suis un chien "`, `Aside`, `Str "."`; no `Space` node anywhere | writers (M4) |

### Roles

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Positional argument is sugar for the principal key (`{aside left}`, `{code py}`, `{raw latex}`) | implemented | `{aside left}[x]` → `side: "left"`, printed `{aside side=left}[x]`; `{code py}[…]` → `lang: "py"`; `{raw latex}(…)` → `format: "latex"` | — |
| "There is no other micro-syntax inside a role head, in particular no `name:arg` form" | deviates (D4) | `{aside:left}[z]` → `deprecated: \`{aside:left}\` is deprecated, write \`{aside registry=left}\`` and lowers to `Aside` **without** `side` (the argument is lost); printed `{aside}[z]` | code: accept `name:arg` only for `index` (schedule row `{index:registry}`); fixture `role-colon-form.md` |
| "an unknown name is literal text" + C4 hint | implemented | `{qty}[9.81]`, `{qty}(x)` → `role-unknown` hint, literal | — |
| Head not followed by `[`/`(` → diagnostic | implemented | `{aside}` alone → `role-dangling-head`; `{counter}[fw:y]`, `{aside}(arg)`, `{raw latex}[x]` → `role-dangling-head` with the expected form in the message | — |
| "the `index` role accepts several bracket groups, for nesting" (max 3, §IndexEntry) | partial (D28) | `{index}[a][b][c][d]` → three-level `IndexEntry` then `Str "[d]"`, no diagnostic; printed `{index}[a][b][c][d]` (looks accepted) | code: diagnostic on a 4th group, or spec: say the 4th group is literal |
| Role keys are closed per role; positional only for the principal key | partial (D28) | `{index physics}[x]` → `IndexEntry` with no `registry`, silent; `{aside foo}[x]`, `{aside side=up}[x]`, `{aside color=red}[x]` → plain `Aside`, silent; `{code py lang=rs}` → `lang=rs`, silent; `{aside left right}[x]` → literal, silent | code: a `role-bad-argument` code (none exists); spec: say whether an unknown key is an error or ignored |
| Parentheses "nest when balanced" | implemented | `{raw latex}(a(b)c)` → `text: "a(b)c"` | — |
| Role content across a soft break | implemented | `{aside}[across\na soft break]` → `Aside` with `SoftBreak` inside | — |
| Content regex forbids unescaped `[` inside a group | lenient | `{aside}[a [b] c]` → `Str "a [b] c"`; printed `{aside}[a \[b\] c]` | none; note in spec that nested balanced brackets are accepted |
| `{lead}[…]` "has an explicit role" (§Para) | deviates (D3) | with `features: {paragraph.lead: false}`, `{lead}[Role lead.] Text.` → `Para` with `Strong`, no `lead` field, printed `**Role lead.** Text.`; with the feature on, `{lead}[mid]` mid-paragraph → `Strong`, printed `**mid**` (silent) | code: the role must lower to `lead` regardless of the feature (P4: "canonical spelling that does not depend on the switch"); a mid-paragraph `{lead}` needs a diagnostic or a spec sentence; fixtures `lead-feature-off.md`, `lead-mid-paragraph.md` |

### Container directives

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Fence regex `^:{3,}\s*name(?:\s+\{…\})?\s*$` | partial (D27) | `::: note {title=x} trailing` → `Admonition note` with **no attrs and no diagnostic** (attrs and text dropped); `::: note{title=x}` → `Div name="note{title=x}"` + `container-unknown`; `:::note` → `Admonition note` (fine) | code: a line that does not match the regex is a paragraph; fixture `container-fence-malformed.md` |
| Nesting by fence length; equal length closes | implemented (documented) | `:::: note` / `::: tip` / `:::` / `::::` → nested; equal `:::` closes the outer | — |
| Unknown name "is a class D error" | implemented | `::: foo` → `Div` + `container-unknown` | — |
| Unclosed container | implemented | `container-unclosed` + `container-unknown` for `::: unclosed` | — |

### Data directives

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Node words `code table table-config image raw`; "the word names the node produced" | implemented for known words | `python image` → `Image` (generate/code attrs), `yaml table` → `Table`, `latex raw` → `RawBlock`, `yaml table-config` → `TableConfig` | — |
| Unknown second word → `fence-unknown-node-word` (design C11; schedule row `latex render` "never shipped") | deviates (D10) | `latex render` → `CodeBlock lang="latex"`, **no diagnostic**, printed ```` ```latex ```` (the word vanishes); `python figure` → same; only `mermaid raw` warns (`raw needs a backend`) | code: warn on every unknown second word; fixtures `fence-render-deprecated.md`, `fence-unknown-word.md` |
| Bare `mermaid` → image; `mermaid code` → listing | implemented | as printed: ```` ```mermaid image ```` / ```` ```mermaid code ```` | — |
| `grid table` (§Table rung 4) | missing (confirmed) | → `CodeBlock lang="grid table"`, no diagnostic, printed unchanged | milestone 5; add a `fence-unknown-node-word`-style hint? no: the fence is valid `code`. Note in 02 stays |
| Fence options unquoted (`title=x`) | implemented | `python title=x linenums=1` → options, printed quoted | — |

## §Two sigils, §Registries (lookup)

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| `#[term]`, `#[a][b]`, `#[**term**]` = `main=true`, `#(prefix:key)` | implemented | all four lower as specified; printer emits the roles (C6) | — |
| `#{prefix:key}` recognised only when the prefix is declared (C16) | implemented | `#{fw:x}` with `declare.counters.fw` → `CounterItem` + `deprecated`; `#{user.name}` and undeclared `#{fw:x}` → literal | — |
| `#(x)` without prefix, `#(fw:)` | implemented | literal; `{counter}(nokey)` → `role-dangling-head: takes a prefix:key argument` | — |
| `\#`, `\@` escapes; nothing fires in code | implemented | `\#[x]`, `\@key` literal; `` `@key #[x] {{ x }} {aside}[y]` `` and the same in a fence → `Code`/`CodeBlock` text | — |

## §Lexical grammar

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Bare reference guard `(?<![\w@/:.-])@key[A-Za-z0-9]$` | implemented | `a@b`, `foo:@key`, `@1abc` literal; `user@example.com` and `http://x.com/@y` autolinks; `@key.` `@key,` `@key;` `@key:` `@key)` keep punctuation out; `@a-b-` → key `a-b` | — |
| Bracketed item grammar (prefix, `-`, key, suffix, `;`) | implemented | `@[see ein05, pp. 33-35; AI2027, ch. 1]` → two items with prefix/suffix; `@[-ein05]` → `suppress_author` | — |
| Caption regex `^(Table|Figure|Listing):\s+…(?:\s*\{#id\})?\s*$` | lenient | `table:` and `Table:no-space` literal (correct); `Table: x {#tbl:a .wide k=v}` accepted with classes and kv | spec: allow a full attribute list on caption lines (`media=`, `lang=` are universal) — line for 12-spec-challenges |
| Attribute value `\S+` (C1) | implemented as C1 | `{k=a}b}` → `{k=a}` then `b}` | — |
| Role content escapes `\[ \]` | implemented | `{aside}[a \[b\] c]` → `a [b] c` | — |

## §Conformance and deviations, profiles

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| `strict` "disables X1 and X2 and rejects Appendix PyMdownX sugar" | partial (D19) | `fmt --profile strict`: `__sc__` → `**sc**`, `~sub~` → `\~sub\~`; but `==x==`, `^x^`, `++k++`, `` `#!py` `` still lower to their nodes; **no diagnostic is produced under strict** and `check` has no `--profile`, so `strict-x-construct` is unreachable | M5; meanwhile `check --profile strict` and the lint rule (05 notes say it waits) |
| "A conformant processor MUST document which X-deviations are active" | missing | no `--profile` on `parse`/`check`; no listing anywhere | code: `tmark check --profile`, and print the active set in `--help` or `tmark.toml` (M3 item 8) |
| Feature `compat.pymdownx` "off under strict", flippable from `features:` | deviates (D18) | `features: {compat.pymdownx: false}` → `==m==` still `Highlight` | code: honour the switch in lowering, or spec: say the front matter cannot flip it |
| `mkdocs` profile "emits the PyMdownX spellings" | stub (documented) | `fmt --profile mkdocs`: `{sc}[sc]`, `{lead}[…]`, `{keys}[ctrl+s]` | M5 |
| X4: `@` never fires in e-mails/URLs | implemented | see §Lexical grammar | — |

## §Front matter

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| "YAML island at line 1" | implemented | a blank line before `---` → `HorizontalRule` + setext heading | — |
| "`press` wins" per key | implemented | root `title: Root title` + `press.title: Press title` → `keys.title = "Press title"`; `title: null` → `null` (distinct from absent) | — |
| Draft-2 top-level keys "remain accepted with a deprecation warning" | deviates (D17) | root `counters: {fw: …}` **plus** `press.declare.counters: {rq: …}` → `deprecated-frontmatter-key` for `counters`, and `fw` is gone (`keys.press.declare.counters = {rq}`), so `#(fw:two)` → `prefix-unknown`. Group-level "canonical wins" drops every deprecated key, not only colliding ones | code: merge per key inside the group; fixture `frontmatter-deprecated-merge.md` |
| Schedule rows `admonitions.<type>.icon/.color` → `press.callouts`, `press.callout_style` → `press.callouts.style` | missing (D17) | `admonitions.solution.icon: "x"` is silently dropped from `keys` (not in `extra`, no diagnostic); `press.callout_style` lands in `extra`, no diagnostic | code: two `deprecated-frontmatter-key` emissions; fixtures |
| C9: "Unknown keys inside a TMark-owned group are errors" (`frontmatter-unknown-key`) | missing (D16) | `declare.nonsense: 1`, `declare.counters.rq.bogus: 1`, `sources.nothing: 1`, `features.bogus.feature: true` → nothing; `grep FrontmatterUnknownKey crates/*/src` finds only the enum | code: emit from `tmark-ir::frontmatter::parse` (`serde(deny_unknown_fields)` on the owned structs); fixture `diag-frontmatter-unknown-key.md` |
| `frontmatter-yaml` | implemented | `bad yaml: [unclosed` → error, body still parsed | fixture missing (D33) |
| Moustache `{{ key }}`: "An unresolved moustache warns" | missing (D22) | `{{ missing }}` → `Var path=["missing"]`; `check` prints nothing; no `Code` exists for it | code: `var-unresolved` in the resolver (front matter is available there); fixture |
| Moustache "never inside code spans or fenced blocks" | implemented | `` `{{ code }}` `` → `Code` | — |
| `&nbsp;` "accepted as an explicit override, class C" | deviates in the printer (D25) | `1&nbsp;km` → `Str` with U+00A0; `fmt` prints the raw byte pair `302 240`, the entity spelling is lost (§Round-trip: "never loses … escapes") | code (fmt): print U+00A0 as `&nbsp;`; spec: list entities among what the printer preserves |

## §Node catalogue — Structure

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Header: six levels, attributes at end of line | implemented | `####### Seven` → paragraph; `# Heading {#id} more text` → literal + `attr-no-host` | — |
| Para: lead promotion under 80 chars, feature-switched, `lead-promotion` info | implemented | `**Boot sequence.**` (the whole paragraph) → `lead`; `**Boot sequence.** The device…` stays a bold run-in (parity F4); a 110-character strong paragraph stays `Strong`; `paragraph.lead: false` disables | — |
| BlockQuote `{.epigraph}` | partial (D26) | see §Attributes | — |
| Task items `- [ ]`, `- [x]`; `- [.]` under `tasklist.partial` | implemented | `task: open/done`; `[.]` literal by default, `partial` with the feature | — |
| `pymdownx.fancylists` markers | missing (confirmed) | `a.`, `i.`, `#.` lists → paragraphs; `1)` → `OrderedList` printed `1.` | M5 |
| DefinitionList | implemented | two definitions per term kept | — |
| Comment: "the HTML comment, inline or block" is a node | partial (D23) | `Text <!-- c --> more.` → inline `Comment` (good); `<!-- alone -->` → block `Comment` (good); `<!-- leading comment --> trailing text.` at line start → `RawBlock format=html` holding the whole line, printed as a ```` ```html raw ```` fence: the prose "trailing text." becomes raw HTML and disappears from LaTeX/Typst output | code: split a leading comment from the rest of the HTML-block line, or spec: say a comment at line start must be alone on its line; fixture `comment-leading.md` |
| Other HTML (`<div>`, `<span>`) | spec gap (D24) | `<div>…</div>` → `RawBlock html` printed as ```` ```html raw ````; `<span>` → `RawInline html` printed `{raw html}(<span>)`. Class-C input is rewritten to class-D spellings | spec: give inline/block HTML a row in §Raw passthrough with its canonical spelling (keep the HTML as typed?) — line for 12-spec-challenges |
| HorizontalRule `---`, `***` | implemented | both → `HorizontalRule`, printed `---` | — |

## §Node catalogue — Inline text, Math (inline), Notes

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Table `tbl:inline` sugar → nodes | implemented | `__x__`→`SmallCaps`, `~~x~~`→`Strikeout`, `==x==`→`Highlight`, `~x~`→`Subscript`, `^x^`→`Superscript`, `++ctrl+s++`→`Keystroke ["ctrl","s"]`, `` `#!py print(1)` ``→`Code lang=py`; canonical roles printed | — |
| `Quoted` `"x"`, `'x'` | missing (confirmed) | `"double"` stays in `Str` | M5 or drop the row (spec) |
| `^^x^^` only with `inline.insert` | implemented | literal by default, `Underline` with the feature | — |
| Math inline: "No space directly after the opening delimiter" | deviates (D21) | `$ spaced$` → `Math " spaced"`, printed `$ spaced$` | code: keep the PyMdownX rule; fixture `math-inline-space.md` |
| `\(…\)` compatibility layer, rewritten by fmt | implemented | `\(y\)` → `Math`, printed `$y$` | — |
| Footnote `[^1]` + definition | implemented | `Note label="1"`; the definition is printed at the end of the document | — |
| Inline footnotes `^[text]` | missing (confirmed) | literal, printed `\^[an inline note]` | after `[^key]` retirement (schedule) |
| Aside: `{aside}`, `{margin}` deprecated, `{margin}[…]{l}`, `::: aside`, `::: margin` | implemented | all lower; deprecations fire for `{margin}` and `::: margin` | — |

## §Node catalogue — Anchors, references, citations

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| "Any element takes an id through attributes … a span (`[this claim]{#claim:one}`)" | deviates (D13) | `[span]{#claim:one}` then `@claim:one` → `ref-unresolved`; `::: note {#my:note}` then `@my:note` and `[u](#my:note)` → `ref-unresolved`; declared `::: solution {#sol:one}` (no counter) → same. `::: theorem {#thm:t}` resolves (it has a counter) | code: register every anchored node as a label (with `kind`), numbered or not; fixtures `anchor-span.md`, `anchor-admonition.md`. This blocks M3 navigation and completion |
| "Where a block has a caption line, the anchor lives on the caption line; otherwise on the element. One rule, no second place." | missing check (D12) | `![alt](f.png){#fig:a}` + `Figure: … {#fig:b}` → two labels, no diagnostic | code: `label-duplicate`-class warning "two anchors on one float"; fixture |
| "when a prefix is present it must agree with the host, and a mismatch is linted" | implemented for image/table/heading | `![…]{#tbl:wrong}` → `prefix-host-mismatch` | — |
| Case-insensitive prefixes, `@Sec:` capitalisation | implemented | `@Sec:intro`, `@SEC:INTRO` resolve to `{#sec:intro}` | — |
| `[](#sec:intro)` empty-link form | implemented | `Link target=Anchor`, resolved like a reference | — |
| `[](other.md)` "section number of another document's main heading" (C12: `ref-unresolved` unless an inventory matches) | deviates (D15) | `[](other.md)` → `Link target=Document`, `check` prints nothing (no crossrefs declared) | code: emit `ref-unresolved`; fixture `reference-document.md` |
| Pandoc `[@key, locator; -@key2]` import, never emitted | implemented | printed `@[key, locator; -key2]` | — |
| `@doi:…` with `/`, trailing punctuation out; `@https://doi.org/…` normalised | implemented | both → key `doi:10.1002/andp.19053221004`; the URL form gets no `deprecated` (schedule horizon "indefinite") | see D29 |
| Resolution order, key in two registries "is a hard warning" | implemented | `stock` label + `stock` bib key → `ref-ambiguous` | — |
| Citation sugar `[^key]`, `^[k1,k2]`, shadowing rule | missing (confirmed) | `[^ein05]` with a `.bib` key and a real `[^ein05]:` definition → `Note`, no `citation-shadowed-by-footnote` (never emitted anywhere); `[^ein05]` without definition → literal | M5 or drop the row |
| Glossary `@gls:term`; `[](gls:term)` deprecated (horizon fmt) | partial (D6) | `@gls:solid` resolves from `declare.glossary`; `[](gls:term)` → `Link target=Url "gls:term"`, **no `deprecated`**, printed unchanged; `check` says nothing | code: lower to `Ref gls:term` + `deprecated`; fixture `role-gls-link-deprecated.md` |
| `note` prefix "footnotes" (Table `tbl:prefixes`) | missing (D14) | `[^1]` defined, `@note:1` → `ref-unresolved` | code: register footnotes under `note`, or spec: drop the row |
| CounterItem: undeclared prefix warns | implemented | `#(fw:x)` → `prefix-unknown` | — |
| A user prefix "may not shadow a role name" | implemented | `declare.counters.aside` → `prefix-unknown: … shadows the role of the same name` (message good, code reused) | consider a dedicated code |
| IndexEntry `{index}[…]{b}` / `{i}` deprecated (horizon fmt) | missing (D5) | `{index}[t]{b}` → `IndexEntry` + `Str "{b}"`, no diagnostic, printed `{index}[t]\{b}` | code + fixture `role-index-suffix-deprecated.md` |

## §Node catalogue — Captions and floats

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Attachment rule: block before if a float without caption, else float after | implemented | `Table A / Table: x / Table B` → caption of A (`position: after`); `Table: x / Table` → `position: before`; sandwiched case correct; `caption-no-host` on an orphan | — |
| Kind of the caption versus kind of the host | deviates (D11) | `Table A / Figure: mis {#fig:mis} / Table B` → `Caption kind=figure` attached to Table A, no diagnostic at parse or resolve (`prefix-host-mismatch` compares the prefix with the caption's own kind) | code: `caption-kind-mismatch` or reuse `prefix-host-mismatch` against the host node; spec: state that the kind must match the host — line for 12-spec-challenges |
| Sugar: "a `Table:` line before the table" | lenient | `Listing:` before a code block and `Figure:` before an image also attach (`position: before`) | spec: generalise the sentence to every kind, or code: only `Table:` |
| "In the IR the caption always follows its host; the source position is recorded" | implemented | `position: before/after` | — |
| `::: figure` caption "with no such neighbour is the caption of the figure itself" | implemented, ambiguous IR | `Figure.content = [Para(images), Caption]`: the same shape as "caption of the image paragraph" | IR review (mandate 4) |
| Table rung 3: `table-config` then caption, whatever the source order | implemented | printed table / config / caption | — |
| `yaml table` → Table; printed as a pipe table when plain | implemented (documented) | — | — |
| Images `.mmd`, `.drawio`, `.mp4` are images | implemented (syntax only) | `Image src=…` | — |
| Listing caption promotes a code block | implemented | `Listing: … {#lst:bubble}` after a fence → `Caption kind=listing`; `@lst:l` resolves | — |
| Math display `$$…$$ {#eq:x}`, `\[…\]`, one-line `$$x$$ {#eq:a}` | implemented | all → `MathBlock`; `\[…\]` printed `$$`; `$$` unclosed at EOF → `MathBlock`, no diagnostic (D32, minor) | optional `math-unclosed` |

## §Node catalogue — Containers, Raw passthrough, Includes

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| `!!!`, `???`, `???+` → `::: type {title collapsed}` | implemented | `??? note "Folded"` → `collapsed=true`; tab-indented body accepted | — |
| `/// name … ///` → `::: name`, deprecated | partial (D9) | `/// note | Title` → `Admonition note` **without the title**, plus a spurious `attr-no-host: malformed attribute list on the fence`, plus `deprecated` | code: parse the PyMdownX `| Title` form; fixture |
| `/// latex … ///` → `latex raw` fence (Table `tbl:raw`) | deviates (D7) | → `Div name="latex"` + `container-unknown` + `deprecated: write \`::: name\``; the body `\clearpage` is parsed as Markdown; `fmt` prints `::: latex`, which re-parses with `container-unknown` (the printer emits a spelling that warns) | code: lower `/// latex|typst|html` to `RawBlock`; fixture `fence-raw-slash-deprecated.md` |
| `/// caption`, `/// figure-caption` → caption line | missing (D8) | `/// caption` → `Div caption` + `container-unknown`; no `deprecated` with the right replacement | code + fixture |
| Theorem types predeclared, `proof` without counter | implemented | `::: theorem {#thm:t}` → `@thm:t` resolves; `::: proof` → `Admonition proof` | — |
| Custom types from `declare.admonitions` | implemented at parse time | `::: solution` → `Admonition kind=solution` when declared, `Div` + `container-unknown` otherwise | but see D13 for its anchor |
| RawInline `{raw latex}(…)`; `{latex}[…]` deprecated | implemented | `{latex}[\x]` → `RawInline` + `deprecated` | — |
| Block include splices, nested includes resolve against the included file's directory | implemented | `inc/main.md` includes `sub/chapter.md` which includes `deep.md`: `@sec:inc`, `@sec:deep` resolve, `check` exit 0; a ```` ```md ```` fence containing `:::` inside the include closes nothing | — |
| `{include base=…}(…)`, `--8<--` deprecated, inline include → `include-inline` (C10) | implemented | as printed | — |

## §Registries, §Feature registry

| Spec claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Predeclared prefixes, `fig` on `tbl` host warns, user counters on headings | implemented | `## Boot loop {#fw:boot-loop}` resolves with `declare.counters.fw` | — |
| `.bib` on the command line feeds the bibliography | implemented | `check file.md refs.bib` → `ref-ambiguous` for a shared key | — |
| Acronyms `*[HTML]: …` | implemented differently from 02 | definitions land in `Document.abbreviations` (key, expansion); **no `Abbr` inline is substituted** in `Str` (02 says "substitutes `Abbr` inlines") | 02 note or code; writers decide (M4) |
| Crossrefs: missing inventory warns | implemented | `crossref-inventory-missing` | `crossref-inventory-stale` never emitted (documented: waits for TeXSmith) |
| Feature table: `paragraph.lead`, `tasklist.partial`, `inline.insert` | implemented | see above | — |
| `compat.pymdownx` | deviates (D18) | see §Conformance | — |
| `table.decimal-align`, `figures.exec`, `glossary.wikipedia` | not applicable to the parser | — | writers/TeXSmith |

## Appendix PyMdownX

| Sugar | Status | Evidence | Proposed action |
| - | - | - | - |
| `!!!`, `???`, `++k++`, `` `#!py` ``, `==`, `~~`, `^`, `~`, `^^` (feature), `--8<--`, bare URLs, bare `mermaid` | implemented | see the tables above; `www.example.com` → `Link` printed `[www.example.com](http://www.example.com)` (D30: not printed bare like `https://…`, stable after one pass) | printer: print `www.` autolinks bare |
| `/// name … ///` | partial | D7, D8, D9 | — |
| `[=75% "Review"]`, `.thin` (ProgressBar) | missing (confirmed) | literal; no `ProgressBar` node in `tmark-ir::Inline` | M5; the IR lacks the node the spec names |
| `[[Page Title]]`, `[[Page|label]]` | missing (confirmed) | literal | M5 |
| Critic markup | missing (confirmed, C14) | `{++ins++}` … `{>>c<<}` literal; printed with escapes (`{\+\+ins\+\+}`) | M5 |
| `:smile:`, `(c)`, `-->`, `1/2` → `Str` | implemented (literal is `Str`) | — | — |
| `"quotes"`, `--`, `...` → `Quoted`, `Str` | missing (`Quoted`) | — | M5 or spec |
| `1)`, `a.`, `i.`, `#.` → `OrderedList` with style | partial | `1)` only (CommonMark) | M5 |
| `[TOC]` "accepted and ignored" | partial | → `Para [Str "[TOC]"]`, printed `[TOC]`; not dropped, not flagged | spec: "kept as a paragraph" or code: drop with a hint |

## Appendix Deprecations

| Row | Status | Evidence | Proposed action |
| - | - | - | - |
| `#{prefix:key}` (fmt) | implemented (C16) | `deprecated` + `{counter}(fw:x)` | fixture exists (`counter-item.md`) |
| Pandoc `[@key]` (indefinite) | implemented, silent | no `deprecated` | policy, see D29 |
| `[^key]`, `^[k1,k2]` citations (fmt) | missing | literal / `Note` | M5 or spec |
| `{latex}[…]` etc. (fmt) | implemented | `deprecated` + `{raw latex}(…)` | — |
| `/// latex … ///` (fmt) | deviates (D7) | `Div` | code |
| `latex render` (never shipped) | deviates (D10) | silently `CodeBlock latex` | code |
| `/// caption`, `/// figure-caption` (fmt) | missing (D8) | `Div` | code |
| `{index:registry}` (fmt) | implemented | `deprecated` + `registry=` | — |
| `{index}[…]{b}` / `{i}` (fmt) | missing (D5) | literal `{b}` | code |
| `{margin}`, `{margin}[…]{l}` (fmt) | implemented | `side=left` | — |
| `::: margin` (fmt) | implemented | `deprecated` + `::: aside` | no fixture |
| `--8<--` (fmt) | implemented | `deprecated` + `{include}(…)` | — |
| `@https://doi.org/…` (indefinite) | implemented, silent | normalised, no `deprecated` | D29 |
| `[](gls:term)` (fmt) | missing (D6) | `Link` | code |
| bare `mermaid` (indefinite) | implemented, silent | — | D29 |
| `Table:` before (indefinite) | implemented, silent | — | D29 |
| `!!!` / `???` (indefinite) | implemented, silent | — | D29 |
| top-level `bibliography`, `crossrefs`, `counters`, `admonitions`, `glossary`, `acronyms` (fmt) | partial (D17) | `deprecated-frontmatter-key` fires; values dropped when the canonical group exists; message says "move it under its `press` group" (the spec says `sources.*` / `declare.*`) | code |
| `admonitions.<type>.icon/.color`, `press.callout_style` (fmt) | missing (D17) | silent | code |
| `--no-promote-title` | not applicable | CLI of TeXSmith | — |
| Policy: which rows emit `deprecated` | spec gap (D29) | rows with horizon "indefinite" are silent, rows with "fmt" warn (when wired); nothing in the spec or in 05 states this | line for 12-spec-challenges: "`deprecated` fires for rows whose horizon is `fmt`; indefinite rows are silent sugar" |

## Diagnostic codes (design 05, `Code::ALL`)

| Claim | Status | Evidence | Proposed action |
| - | - | - | - |
| Every code is emitted by some stage | deviates (D16) | never emitted anywhere in `crates/*/src`: `frontmatter-unknown-key`, `strict-x-construct`, `citation-shadowed-by-footnote`, `crossref-inventory-stale` | wire `frontmatter-unknown-key` now (C9); the other three are M5/TeXSmith and should say so in 05 |
| README: "every diagnostic code … has a fixture" | deviates (D33) | no fixture names these codes in a `diagnostics`/`resolution` block: `attr-no-host`, `role-dangling-head`, `role-unknown`, `caption-no-host`, `include-inline`, `fence-unknown-node-word`, `frontmatter-yaml`, `frontmatter-unknown-key`, `citation-shadowed-by-footnote`, `crossref-inventory-stale`, `strict-x-construct`, `deprecated-frontmatter-key` (12 of 27) | write the 8 that can fire today; the README should stop claiming full coverage until then |
| README: every deprecation-schedule row has a fixture | deviates (D33) | no fixture holds `[^key]` citations, `/// latex`, `latex render`, `/// caption`, `{index}[…]{b}`, `::: margin`, `[](gls:`, top-level `bibliography:`/`counters:`, `icon:`, `callout_style` | same |
| Self-check of the spec | implemented | `check spec/tmark.md` → 13 `position-word`, 2 `hardcoded-number` hints, nothing else; `fmt` idempotent; round-trip IR equal | — |

## Ranking

### For milestone 3 (editor: diagnostics, navigation, completion)

1. **D13** anchors on spans and on admonitions without a counter are not labels — go-to-definition, references and `@` completion miss every `[text]{#id}` and `::: note {#id}`; and `[link](#id)` to them is reported unresolved (false positive in the editor).
2. **D16 / C9** `frontmatter-unknown-key` never fires — the front-matter schema completion (M3 step 5) will offer keys the parser then ignores silently; typos in `declare:` go unnoticed.
3. **D1, D2** links and fenced blocks are not attribute hosts — ids typed there are dropped (once with no diagnostic), so labels vanish from the outline and the editor shows nothing.
4. **D3** `{lead}[…]` depends on the feature switch and lowers to `Strong` mid-paragraph — a code action "promote to lead" would produce a construct that can silently degrade.
5. **D17** deprecated front-matter groups dropped instead of merged — a `deprecated-frontmatter-key` warning followed by `prefix-unknown` on every item is a confusing pair for a user; the fix for the first (a code action moving the key) is what M3 step 7 wants to ship.
6. **D26, D9, D27** spurious or missing `attr-no-host` on block-quote attributes, `///` titles and malformed `:::` fences — noise or silence exactly where an editor should be precise.
7. **D10, D7, D8, D5, D6** unknown node words and four deprecation rows: no diagnostic and no fix, so the "deprecated spelling → canonical" code action (M3 step 7) has holes; D7 makes `fmt` emit a spelling that warns.
8. **D11, D12** caption kind mismatch and double anchors — two structural errors the resolver could report cheaply.
9. **D15, D14, D22** `[](other.md)`, `@note:1`, `{{ missing }}` silent — small but each is a promised warning.
10. **D19** strict profile only through `fmt` — `tmark.toml` (M3 step 8) will name a profile that `check`/LSP cannot apply.
11. **D33** fixture coverage — the LSP tests will lean on fixtures for expected diagnostics; 12 codes have none.

### For milestone 4 (writers)

1. **D23, D24** a leading comment swallows its line into raw HTML, and HTML is canonicalised to `html raw` fences / `{raw html}` — LaTeX and Typst writers will drop prose that GitHub shows; the spec needs a row for HTML passthrough.
2. **D25** `&nbsp;` printed as U+00A0 — writers must map the character, and the printer's output hides it from authors.
3. **D18** `compat.pymdownx` ignored — the `mkdocs` writer/profile (M5) will rely on the same switch.
4. **D21** `$ spaced$` is math — a `$5 and $6` sentence becomes a formula in print.
5. **D11** figure caption on a table host — the LaTeX writer will put a `\caption` of the wrong kind in a `table` environment.
6. Abbreviations: `Document.abbreviations` without `Abbr` inlines — the writers must do the substitution themselves; document where this lives (02 says otherwise).
7. Confirmed M5 gaps (grid tables, `Quoted`, `ProgressBar` without an IR node, fancy lists) — nothing to write for them yet; keep them out of the M4 definition of done.

### Proposed lines for `12-spec-challenges.md`

- C18 §Caption: the attachment rule must require the caption kind to match the host kind (`Figure:` on a table is an error, code `caption-kind-mismatch`); say whether `Figure:`/`Listing:` before their float is sugar like `Table:` before.
- C19 §Structure/§Raw: inline and block HTML other than comments have no catalogue entry; decide the canonical spelling (kept as typed, class C) so the printer stops rewriting them to class-D fences.
- C20 Appendix Deprecations: state which rows emit `deprecated` (horizon `fmt`) and which are silent sugar (`indefinite`).
- C21 §Roles: state what happens to an unknown key or an invalid positional on a known role (error with a code, or ignored); today both are silent.
- C22 §Front matter: `&nbsp;` and other entities are preserved by the printer (§Round-trip lists escapes; entities are not mentioned).
- C23 §Lexical grammar: caption lines accept a full attribute list, not only `{#id}` (universal `lang=`/`media=`).
