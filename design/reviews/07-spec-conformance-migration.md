# Review 07 — Spec conformance of the migration decisions (C27–C50)

Scope: the sections of `spec/tmark.md` that challenges C27–C50 of
`design/12-spec-challenges.md` wrote or rewrote, checked against the code
that implements them (`tmark-syntax::lower`, `tmark-markdown::construct`,
`tmark-registry::collect`, the three writers, `tmark-fmt`) and the fixtures
of `spec/conformance/`, on branch `texsmith-migration` at commit `353311f`.
Method and vocabulary follow `02-spec-conformance.md`: every row is one
spec statement, the code that implements it, the fixture that demonstrates
it, and a classification — **conforms**, **code deviates**, **spec
ambiguous** (or self-contradictory), **no fixture**. Every claim below was
exercised through the CLI (`tmark parse|fmt|check|write`) on scratch inputs
under `/tmp/claude-1000/-home-ycr-texsmith/3f80713c-…/scratchpad/c27/`
(`tabs.md`, `div.md`, `div2.md`, `logos.md`, `emoji.md`, `progress.md`,
`pb*.md`, `hr.md`, `raw.md`, `header.md`, `nfc.md`, `jp.md`, `smart.md`,
`foreign.md`, `lead*.md`, `anchor.md`, `subfig.md`, `gls*.md`,
`critic.md`, `cite.md`, `esc.md`, `deprec.md`, `key27.md`, `mk.md`,
`last.md`); two backend claims were confirmed by compiling the writer's
output with `typst` and `pdflatex` (`~/.cache/tmark-review-probe/`).
Nothing under `crates/`, `spec/` or `design/` other than this file was
modified.

The citation default is being changed by another agent in a worktree; the
`\textcite` / `#cite` question is listed once, as a known item (K1), and
not counted.

## 1. Findings, ranked

| Id | Section | Severity | Claim | Evidence |
| -- | ------- | -------- | ----- | -------- |
| F1 | §Round-trip, §ProgressBar, Appendix smart symbols | **major** | `parse(print(ir)) == ir` fails for escaped sugar: the printer never escapes `[=`, `(c)`, `-->`, so an author's literal `\[=50% "x"]`, `\(c)`, `\-\->` come back as a `ProgressBar`, `©`, `→` on the next parse; `1\/2` is `½` on the first parse already (the CommonMark escape is consumed before the text-run scan) | `pb5.md`: `A \[=50% x] B \[=50%] C \[=50% "q"] D.` → `fmt` prints `A [=50% x] B [=50%] C [=50% "q"] D.`, re-parsed: 2 `ProgressBar`; `smart.md` line 10: `fmt` not idempotent (`(c)` → `©`, `-->` → `→` on the second pass); `crates/tmark-fmt/src/escape.rs:184` (`[` escaped only before `^`/`@`), no smart-symbol escape anywhere in `escape.rs` |
| F2 | Appendix `"quotes"` row (C46) | **major** | "a phrase whose quotes sit on either side of inline markup stays literal" holds for that phrase only: its orphan closing `"` then pairs with the *next* phrase's opening `"`, so `"one *two* three" and "plain"` renders `three\enquote{ and }plain\enquote{ and …}` — every quote of the paragraph is shifted by one | `smart.md` line 4 → LaTeX `Quotes: "one \emph{two} three\enquote{ and }plain\enquote{ and 'single' and }unclosed …`; HTML `three“ and ”plain“`; `crates/tmark-syntax/src/lower/sugar.rs:192` (`quoted`: any `"` with a later `"` on the line, no word boundary on either side). The `Quoted` nodes also all carry the same span (`[123,175]`), so an editor cannot locate them |
| F3 | §Lexical grammar, bare and bracketed key (C27, still open) | **major** | A digit-initial key (`7HA7H`, Zotero/Better BibTeX) with a locator is swallowed whole: `@[7HA7H, p. 3]` → key `"7HA7H, p. 3"`, `@[see 1RgTv]` → key `"see 1RgTv"`; the "whole item is the key" fallback the row relies on has no locator grammar, and the spec text still shows `[A-Za-z]…` with no sentence on digit-initial keys | `key27.md` → IR `{"key":"7HA7H, p. 3"}`, `check`: ``ref-unresolved: `@7HA7H, p. 3` ``; `spec/tmark.md:417,426`; `design/12-spec-challenges.md` C27 status `spec` (open) |
| F4 | §Lexical grammar, family-4 info string (C28, still open) | **major** | The normative recogniser admits only bare `key=value` after the node word, yet fixture `fence-attributes` and the parser accept a brace attribute list (`` ```md {.snippet caption="…"} ``); the spec's "normative recognisers" contradict its own conformance suite | `spec/tmark.md:409` versus `spec/conformance/fence-attributes.md`; C28 resolution says "Add `(?:\s+\{…\})?` to the regex", not done |
| F5 | §Table rung 5 (C30, still open) | **major** | The row grammar C30 decided (first column a column like the others, an item fills its column's *remaining* leaves, every absorbed slot `~` inside a group list too, omitted named column under a span left absorbed, two-column minimum) is implemented and fixtured but absent from §Table, which still only says "`~` acknowledging absorbed slots"; a rung-5 table cannot be written from the spec alone | `spec/tmark.md:1216-1221`; `crates/tmark-syntax/src/lower/table_yaml.rs`; fixtures `fence-yaml-table-spans`, `diag-table-span`, `diag-table-row-width`; the rule exists only in `design/03-ir.md:249-258` |
| F6 | §Header, implicit id (C38) | **major** | "NFC-normalised" is stated and not done: the slug of a decomposed title (`Café`) is not the slug of its NFC form, so `[](#café-decomposed)` typed precomposed is `ref-unresolved`; the code comment records the deviation instead of a challenge row (AGENTS.md P1) | `nfc.md`: first link unresolved, second (decomposed) resolves; `crates/tmark-registry/src/collect.rs:533-534` "NFC normalisation is left to the editor" |
| F7 | §Header (C38), Typst writer | **major** | An implicit id that is not ASCII is mangled to dashes in Typst labels (`café-au-lait` → `<caf--au-lait>`, `日本語 見出し` → `<------->`); two such headings referenced in one document give a duplicate label and Typst refuses to compile | `header.md` → `== 日本語 見出し <------->`; `~/.cache/tmark-review-probe/lab.typ`: ``error: label `<------->` occurs multiple times``; `crates/tmark-writers/src/typst/escape.rs:26-37` |
| F8 | §Anchor, §Attributes (C45), §Div | **major** | A numeric reference to an anchor with no counter (`@top` on `[]{#top}`, `@d1` on `::: div {#d1}`, `@fig:u` on a sub-figure of an unnumbered container) resolves silently and renders `?` (HTML), `Figure ?`, or a meaningless `\ref{top}` (LaTeX: a `\phantomsection\label` prints the enclosing section number); no diagnostic anywhere; the spec never says what such a reference renders | `anchor.md`, `div2.md`, `subfig.md` → `check` exit 0, HTML `<a href="#top" class="reference">?</a>`, LaTeX `\ref{d1}`; `crates/tmark-writers/src/html/mod.rs:995` |
| F9 | §Ref, empty-link form | **major** | `[](#id)` is "the empty-link form" of `@id` (class C) but the HTML writer renders it as an empty anchor, invisible on the page, while `@id` renders "Section 2"; LaTeX renders the bare number without the label word. The `heading-implicit-id` fixture uses this form and has no backend block to catch it | `el.md` → `<a href="#sec:x" class="reference"></a>` vs `<a href="#sec:x" class="reference">Section 1</a>`; LaTeX `\ref{sec:x}` vs `Section~\ref{…}` |
| F10 | §Image, Figure (C43), Typst writer | **major** | A `::: figure` that is "a plain float" (images plus prose) is written by Typst as one `#figure([#box(image(…), height: 1em)) …])`: the images shrink to 1em inline boxes and their anchors are dropped, so `@fig:p` becomes `#ref(<fig:p>)` to a label that does not exist (compile error). LaTeX nests `figure` environments inside the outer one (compiles under `float`'s `[H]`, unorthodox) | `subfig.md` → Typst `#figure([#box(image("p.png", height: 1em)) #parbreak() Some prose. …], caption: [Prose.]) <fig:prose>` and `#ref(<fig:p>, …)`; LaTeX `\begin{figure}[H]…\begin{figure}[H]…` |
| F11 | §Raw / writer escaping, Typst | **major** | Typst markup treats `//` as a line comment and the writer does not escape `/`: any prose holding `//` (or `/*`) loses the rest of its line silently | `esc.md` → `; //x #link("https://e.com/a_b").`; `cm.typ` compiled, `pdftotext`: `Text ;` — the link and `end.` are gone; `crates/tmark-writers/src/typst/escape.rs:5` (`MARKUP` has no `/`) |
| F12 | §Tabs (C31) | minor | "A `tab` outside `tabs` is a `tabs` of one": the code merges consecutive orphans into one set (design 05 says so; the spec does not); "`tabs` takes no attribute of its own" and "each with a `title=`" are not checked (silent), and a `tabs` id is a resolvable label that the paged writers never emit (`\ref{ts}` dangles) | `tabs.md`, `last.md`; `lower/block.rs:907-946`; design `05-diagnostics.md:209-211` |
| F13 | §ProgressBar (C35) | minor | An id on a bar is not a label (`@pb` → `ref-unresolved`), against "Attributes attach as on any host" + §Anchor "Any element takes an id"; extra classes are forwarded to LaTeX (`\tsprogress[thin,other]`) although the spec says they "are ignored in print" | `last.md`, `progress.md` |
| F14 | Appendix critic (C49) | minor | Leading/trailing whitespace of an annotation is lost (`{++ x ++}` → `{++x++}`: a reviewer cannot insert a space); nesting is unspecified and literal; an annotation broken across a line (`{++a\nb++}`) falls through to the `++…++` keystroke sugar and renders `{` + a key `AB` + `}`; `\{++x++}` (escaped brace) does the same | `critic.md` lines 5, 7, 15; `lower/inline.rs:712-800` |
| F15 | §Emoji and icon shortcodes (C34) | minor | "hints `icon-web-only` once per shortcode" reads as once per distinct name; the code hints once per occurrence. `:smile:a` stays literal (the code requires a non-alphanumeric after the closing colon); PyMdownX's pattern has no such boundary and the spec states none | `emoji.md`: three hints for `:material-cog:`; `sugar.rs:95-112` |
| F16 | §Header `.unlisted` (C39) | minor | Both paged writers make an `.unlisted` heading unnumbered as well (`\subsection*`, `numbering: none, outlined: false`); the spec's "additionally keeps it out of the table of contents" leaves the numbering of an `.unlisted`-only heading open (Pandoc: unlisted needs unnumbered). The fixture has no backend block | `header.md`; `heading-unnumbered.md` |
| F17 | §Header implicit id, HTML writer | minor | The HTML body never carries the implicit id on the heading (`<h2>Boot sequence</h2>`) but links to it (`href="#boot-sequence"`), by design 07 ("none (the site slugs)"); standalone `tmark write --to html` output therefore dangles, and a site whose slugifier is not GitHub's (Python-Markdown `toc`) dangles differently. The spec says nothing about which writer emits the id | `header.md` HTML; `design/07-writers.md:332`; `html/mod.rs:270-279` |
| F18 | §Div `mkdocs` profile (C37), design 04 | minor | `[]{#id}` prints unchanged under `--profile mkdocs`, where Python-Markdown shows a literal `[]`: the deprecation table names `[](){#id}` as the MkDocs idiom, and the design-04 profile table has no row for it, nor for `tabs`, `div`, progress bars, critic or icons although `mkdocs.rs` implements the first two | `mk.md` → `[]{#anc}`; `design/04-printer.md:69-92`; `crates/tmark-fmt/src/mkdocs.rs:503,547` |
| F19 | Appendix deprecations, `latex render` | minor | Row "never shipped": still lowers silently to `CodeBlock lang=latex` with no diagnostic and the word dropped on print (review 02 D10, unchanged) | `deprec.md` line 16 → ```` ```latex ```` |
| F20 | §Lexical grammar (C35) | minor | The ProgressBar recogniser covers the percentage form only; the deprecated fraction sugar has no pattern; no recogniser for emoji or critic (accepted: appendix constructs) | `spec/tmark.md:822` |
| F21 | §Foreign directive (C40) | minor | "a hint `directive-foreign` in every backend but the printer": it is a lint rule (`tmark check`); `tmark write` emits no hint. Wording, not behaviour | `foreign.md`; `design/05-diagnostics.md:43` |
| F22 | §Div `markdown="span"` (C37) | minor | "parses it as inlines": the IR is a `Div` holding a `Para`, indistinguishable from `markdown="1"`; the spec should say so (or drop the sentence) | `div.md` |
| F23 | §Math (inline), writers | minor (carried over) | `$5 and $6` is inline math (HTML `\(5 and \)6`, Typst `#mi(...)`); PyMdownX also forbids a space *before* the closing `$`; review 02 D21 covered the opening side | `esc.md` line 1 |
| F24 | Typst writer escaping | minor | `~` is not escaped and is Typst's no-break space; `↔` is emitted raw in LaTeX while `→` becomes `\(\rightarrow\)` | `esc.md`, `smart.md` |
| F25 | Fixtures | minor (no fixture) | No fixture for TeX logos (C33 "no node", but the writer behaviour is what the section specifies and fixtures carry backend blocks); `container-figure-subfigures` claims "`@fig:left` reads figure 2a" with no `html`/`latex` block; `heading-unnumbered` has no backend block; `container-tabs` has no `container-orphan` case; `inline-smart-symbols` shows `\"` only, not `\(c)` | `spec/conformance/` |
| K1 | §Cite | known item | Bare `@key` → LaTeX `\textcite`, Typst `#cite(form: "prose")` (consistent); bracketed `@[key]` → LaTeX `\cite` (biblatex: neither narrative nor parenthetical) but Typst `#cite(<key>)` (parenthetical by default) — the two backends disagree on the parenthetical form. Being changed in a worktree; not counted | `latex/inline.rs:261`, `typst/inline.rs:316-321` |

Counts: 0 blocking, 11 major (F1–F11), 14 minor (F12–F25), 1 known.

## 2. Blocking and major findings, with resolution

**F1 — escaped sugar does not round-trip.** The §Round-trip guarantee is
the spec's first operational promise, and the fixture suite tests
idempotence of `canonical` blocks only, so an escaped spelling never enters
it. Three escapes fail: `[=…%…]` (the printer's `[` rule at
`tmark-fmt/src/escape.rs:184` predates the progress bar), the eight
`smartsymbols` spellings (no escape rule at all), and `1\/2`, where the
CommonMark escape is decoded by the tokenizer before the text-run scan
sees it, so no escape can protect a fraction. Resolution: *fix code* —
teach `escape::text` to escape `[` before `=` when the run matches the
ProgressBar recogniser, and the first character of any `SYMBOLS` /
`FRACTIONS` spelling (`\(c)`, `\-->`, `1\/2` — for the fraction, escape
the `/` and make the lowering skip a fraction whose `/` was escaped in the
source, as `compat.rs` already does with `source.contains("\\…")`); *add
fixtures* `progressbar-escaped`, `inline-smart-symbols-escaped` whose
`input` is the escaped form and whose `canonical` must re-parse to `Str`.
*Fix spec*: one sentence under §ProgressBar and the appendix rows saying
that a backslash before `[=`, `(`, `-`, `<`, `+`, `=` or the `/` of a
fraction keeps the spelling literal, and that the printer preserves it.

**F2 — quote pairing shifts across phrases.** `sugar::quoted` pairs any
`"` with the next `"` on the line. When a phrase straddles inline markup,
its opening quote is correctly left literal, but its closing quote is then
the *opening* of the next pair, and every later quote is off by one; the
LaTeX output puts `\enquote{}` around the words *between* quoted phrases.
Common prose (`"a *b*" and "c"`) triggers it. Resolution: *fix code* —
require an opening `"` to be at run start or after whitespace/`(`/`[`, and
followed by a non-space; a closing `"` to be preceded by a non-space and
followed by end/space/punctuation (the SmartyPants boundaries C46 says it
adopts); give each `Quoted` and its neighbours their own spans. *Fix
spec*: the appendix row should state the boundary rule, since "read inside
one text run" alone does not exclude this. *Add fixture*: a paragraph with
a straddling phrase followed by a plain one, with `latex` and `html`
blocks.

**F3 — digit-initial keys (C27).** C27 is still open in
`12-spec-challenges.md` and neither the bare nor the bracketed grammar
mentions digit-initial keys; the implementation's fallback treats the whole
bracketed item as the key, so a locator or a prefix on such a key is
silently part of the key and the citation is unresolved. Resolution: *fix
spec + close C27* — widen the key class in both recognisers to
`[A-Za-z0-9][\w:.-]*[A-Za-z0-9]` with the all-digit exclusion for the bare
form (`[^123]` stays a footnote label), and state that a bare digit-initial
key needs `@[…]` only if the widening is refused; *fix code* — the
bracketed item parser must find the key by the same class rather than fall
back to the whole item; *add fixture* `reference-key-digit-initial` with a
locator and a prefix.

**F4 — the family-4 recogniser contradicts `fence-attributes` (C28).**
The lexical grammar is declared normative ("what an editor grammar or a
linter needs"); an editor grammar generated from it rejects the fixture's
own input. Resolution: *fix spec + close C28* — append
`(?:\s+(?<attrs>\{…\}))?` (the family-1 pattern) before `\s*$` at
`spec/tmark.md:409`, and say that its classes and id live in
`CodeBlock.options` (or the generated image's attrs) and are printed in
braces, bare `key=value` otherwise, as the fixture prose already does.

**F5 — the rung-5 row grammar is not in §Table (C30).** The decision is
implemented (`table_yaml.rs`) and fixtured (`fence-yaml-table-spans`,
`diag-table-*`), and stated in design 03, but the spec still defers to
"validated before rendering". Resolution: *fix spec + close C30* — add the
five sentences of the C30 resolution to rung 5 (first column ordinary, one
item per top-level column, a group's leaves as a list, an item fills its
column's remaining leaves, `~` for every absorbed slot including inside a
group list, omitted named column under a span left absorbed, at least two
columns), and reference the fixture.

**F6 — NFC is promised and declined.** The spec's slug rule says
"NFC-normalised"; `github_slug` does not normalise and its doc comment says
the editor should. Either is defensible; encoding the choice in a comment
is what AGENTS.md P1 forbids. Resolution: *fix code* (preferred: an NFC
pass through the `unicode-normalization` crate, one dependency line) or
*fix spec + challenge row* dropping the word; *add* the decomposed case to
`heading-implicit-id`.

**F7 — Typst labels of non-ASCII implicit ids collide.** `escape::label`
replaces every non-ASCII character by `-`, so two accented or non-Latin
titles map to the same label and Typst refuses the document. Typst labels
accept Unicode letters; the writer could keep them, or hash the id when it
must fall back. Resolution: *fix code* — keep `char::is_alphanumeric()`
characters in `label`, and when a label still collides append a stable
suffix; *add fixture* with two accented headings referenced, `typst`
block. The spec need not change (labels are a writer concern), but design
07 should say how a Unicode id is spelled in each backend.

**F8 — references to unnumbered anchors render `?` silently.** Review 02
asked (D13) that span and container anchors become labels; they now do,
but nothing says what `@id` renders for a host without a counter, and the
writers print `?`, `Figure ?` or a `\ref` whose value is the enclosing
section number, with `check` exit 0. Resolution: *fix spec* — §Anchor:
"a numeric reference to an anchor whose host has no counter (a span, a
`Div`, a sub-figure of an unnumbered container) is a `ref-unnumbered`
warning; write a textual reference `[text](#id)` instead"; *fix code* —
emit the warning in `tmark-registry::refs` and render the textual fallback
(the anchor's text, or the key) rather than `?`; *add fixture*
`anchor-span-reference` with a `resolution` block.

**F9 — the empty-link reference is empty on the web.** `[](#id)` is the
class-C spelling of `@id` and must render the same number; the HTML writer
emits an empty `<a>` and LaTeX the bare number. Resolution: *fix code* —
route `Link{target: Anchor, content: []}` through the same rendering as
`Ref` in all three writers; *add* `html`/`latex` blocks to
`heading-implicit-id` and the `reference-*` fixtures.

**F10 — plain-float `::: figure` in Typst.** The spec's "a container
holding a table, prose or a listing is a plain float and an image in it
numbers like any other" is honoured by the registry (numbers 4, 5, 6 in
`subfig.md`) and by LaTeX and HTML, but the Typst writer renders the
container's blocks as inline content, shrinking the images to 1em and
dropping their labels, so every reference to them fails to compile.
Resolution: *fix code* — in `typst/mod.rs` write the blocks of a plain
float as blocks (each image its own `#figure` with its label, or a
`#block` wrapper), mirroring `latex/figure.rs:226-243`; *add* a `typst`
block to `container-figure-subfigures` and a fixture
`container-figure-prose` with all three backends.

**F11 — `//` in prose is a Typst comment.** The escaper covers Typst's
markup specials but not the comment openers; a paragraph with `//` (URLs
are autolinked, but "C++ // comment", paths, `and//or`) loses its tail
silently, and `/*` would swallow to the next `*/`. Resolution: *fix code*
— escape `/` when followed by `/` or `*` in `typst/escape.rs::markup`;
add the case to `escapes_markup_specials` and to a fixture with a `typst`
block. Design 07's escaping table should list the comment openers.

## 3. Sections that conform, with their fixture

- §Tabs: `=== "Title"` sugar, `:::: tabs` / `::: tab {title=}`, paged
  writers as titled `tsdiv` blocks, HTML tabbed set — `container-tabs`
  (orphan and attribute cases: F12).
- §Div: closed registry, `multicolumn {cols=}`, `div`, `tsdiv` /
  `#ts-div` contract with `id`, `class={…}`, `key=val` forwarded and
  `lang`/`media` withheld, unknown name kept as `Div` + `container-unknown`
  — `container-multicolumn`, `container-unknown`; `div2.md` for the
  forwarding.
- §Div `md_in_html`: `<div class markdown>` → `::: div {.cls}`,
  `<section markdown>` → unknown container with the diagnostic naming
  both spellings, `mkdocs` profile emits `<div markdown>` —
  `container-div-markdown`.
- §Raw passthrough (C37): HTML as typed (inline and block), printer's
  as-typed test is exactly CommonMark's tag shape (`{raw html}(<span
  class="x">text</span>)` correctly stays a role), non-HTML payloads as
  fence/role, paged writers drop raw HTML and keep the text between tags,
  `<br>` prints nothing — `html-raw`, `role-raw`.
- §Foreign directive (C40, C42): `[TOC]` alone, dotted `:::` line with
  four-space or tab continuation and blank lines, closing at the first
  dedent, `RawBlock{format=markdown}` verbatim, dropped by the writers,
  `directive-foreign` per dotted directive, `[TOC]` silent —
  `directive-foreign`.
- §Header (C38, C39): implicit ids by GitHub's rule (`-1` suffix, Unicode
  kept, punctuation removed, `_` kept), replaced by an explicit id,
  `ref-implicit-id` on each reference, `\label` only when referenced;
  `.unnumbered` → `\section*` + `\addcontentsline`, `numbering: none`,
  `class="unnumbered"` — `heading-implicit-id`, `heading-unnumbered`
  (NFC: F6; Typst labels: F7; `.unlisted`: F16).
- §Para (C44): promotion of a whole-strong paragraph under 80 characters,
  not in a list item, not a run-in, `{lead}[…]` kept whatever the feature
  says (review 02 D3 closed), `lead-promotion` info — `lead`,
  `lead-run-in`. A mid-paragraph `{lead}` still silently becomes `Strong`
  (D3's second half, unchanged).
- §Attributes, anchor (C45): `[](){#id}`, `[](){ #id .c k=v }`,
  `[]( ){#id}` → `Span`, `deprecated` with the `[]{#id}` fix,
  `\phantomsection\label`, `<span id>` — `span-anchor-deprecated`.
- §Attributes `{: …}` (C35): accepted on headings and bars, `deprecated`,
  colon dropped — `attributes-colon`.
- §ProgressBar (C35): percentage canonical, no-label form (label = the
  percentage), decimal value, clamping at 100, fraction sugar normalised
  and deprecated, `[=0/0]` literal, inline in a table cell, nothing in
  code, `\tsprogress`, `#ts-progress`, `<progress>` — `progressbar`
  (escape: F1; anchor: F13).
- §HorizontalRule (C36, C48): `\tsdivider` / `#ts-divider()` / `<hr>` at
  the top level; `\tsrule` / `#ts-rule()` / `<hr class="rule">` inside a
  list item, callout, figure, div, footnote, tab and aside — `divider`
  (block quote), `hr.md` for the other seven containers.
- §Emoji and icon shortcodes (C34): `gemoji` names → character, unknown
  names and `12:30:45` literal, nothing in code, four icon prefixes →
  `Span{.icon media=web}`, printed as the shortcode, dropped from print
  with the spaces collapsed, `<span class="icon">` on the web,
  `icon-web-only` — `inline-emoji`, `inline-icon`.
- §TeX logos (C33): the twelve words as whole words, case-sensitive,
  never in code, math, destinations, attribute values or raw; `\LaTeX{}`,
  `\LaTeXe{}`, `\tslogo{…}`, `#ts-logo("…")`, `<span class="tex-logo">`;
  `typography.tex-logos: false` disables — no fixture (F25).
- §Inline text `^^x^^` (C41): literal + `feature-off` hint by default,
  `Underline` printed as the role with `inline.insert` — `inline-insert`,
  `inline-insert-off`.
- Appendix smart symbols (C46): the eight groups with PyMdownX's
  boundaries (dash runs, number runs, `c/o` as a word), ordinals not
  applied, `--`/`...` untouched, nothing in code, math, raw, destinations
  or attribute values — `inline-smart-symbols` (quotes: F2; escapes: F1).
- §Image, Figure (C43): one number per container, letters in document
  order, a lone image shares the container's number with no letter, plain
  floats number their images normally, an unanchored uncaptioned
  container numbers nothing — `container-figure-subfigures` plus
  `crates/tmark-registry/tests/subfigures.rs`; LaTeX `subfigure`, Typst
  `#ts-subfigure` / `#ts-subnumber`, HTML `(a)` (Typst plain float: F10;
  fixture backend blocks: F25).
- §Glossary and acronyms (C50): flat and structured spellings, mixed
  declarations, `style`/`groups`/`entries` structural at the top level
  only, a term named `style` or `entries` under `entries` winning over a
  flat key, `glossary: <style>` declaring no term, a non-mapping value
  declaring nothing without failing the front matter — `glossary-flat`,
  `glossary-structured`, `gls*.md`.
- Appendix critic (C49): the four annotations as `Span{.critic}` around
  `Underline` / `Strikeout` / both / `Comment`, `{==x==}` a plain
  `Highlight`, first `~>` splits, `{~~x~~}` literal, inline content
  re-parsed, nothing in code, math, destinations, attribute values or raw,
  `\tsins` / `\tsdel` / `\tssubst` / `\tscomment`, `#ts-*`, `<ins>` /
  `<del>` / zero-width comment span, critic spelling printed in every
  profile — `critic-insert`, `critic-delete`, `critic-substitute`,
  `critic-highlight`, `critic-comment` (whitespace and nesting: F14).
- §Includes snippets (C47): any `-{2,}8<-{2,}` marker, `;` escape with no
  diagnostic, paragraph and fence forms, printer fixed points —
  `include-snippet-dashes`, `include-snippet-escaped`,
  `fence-include-snippet`.
- §Lexical grammar family 4 attribute list (C28): the code and fixture
  agree — `fence-attributes` (the regex: F4).
- Appendix deprecation table versus the lowering: every `fmt`-horizon row
  fires `deprecated` with the canonical replacement (`#{p:k}`, `[^key]`,
  `^[k1,k2]`, `{latex}[…]`, `/// latex`, `/// caption`, `{index:r}`,
  `{index}[…]{b}`, `{margin}`, `::: margin`, `--8<--`, `[](gls:t)`,
  `[](){#id}`, `{: …}`, `[=a/b]`, the top-level front-matter groups,
  `press.callout_style` / `admonition_style`); every `indefinite` row is
  silent sugar (`[@key]`, `@https://doi.org/`, bare `mermaid`, `Table:`
  before, `!!!` / `???`, `===`, `<div markdown>`) — `deprec.md`; review
  02's D5–D8 are closed (`latex render`: F19). `lower/compat.rs` now
  reports only wiki links and fancy list markers, as its header says.

## 4. Verdict

The C31–C50 decisions were written into the spec faithfully and the code
follows them in every construct exercised: the twenty-two conforming
entries above cover tabs, layout containers, raw HTML, foreign directives,
implicit ids, lead-ins, anchors, progress bars, dividers, emoji and icons,
logos, insert, symbols, sub-figures, glossary, critic and snippets, each
with its fixture (except TeX logos). What keeps the text from being the
source of truth for a 0.1 release is smaller than review 02's list but
of a different kind:

1. Three challenges are still open and the normative text contradicts
   shipped behaviour: the family-4 recogniser (F4), the bare/bracketed key
   grammar (F3) and the rung-5 row grammar (F5). An editor grammar or a
   second implementation built from `spec/tmark.md` today would reject
   the conformance suite.
2. Two explicit sentences are not implemented (NFC, F6; the ProgressBar
   host as an anchor, F13) and one implemented rule has no sentence (what
   a numeric reference to an unnumbered anchor renders, F8).
3. The round-trip guarantee has holes an escaped input opens (F1) and the
   quote pairing produces wrong output on ordinary prose (F2); neither is
   visible to the suite because fixtures only check canonical inputs.
4. Writer defects that turn a spec-conformant IR into an uncompilable or
   truncated Typst body (F7, F10, F11) or an invisible reference on the
   web (F9).

None is architectural; all eleven majors are a day or two of work each,
and F3–F5 are spec edits plus closing three rows. **Not yet fit as the
sole source of truth**: fit once F3–F6 are closed in the spec (with the
challenge rows deleted per the file's own rule), F1, F2 and F8 have their
fixtures, and the Typst writer has F7, F10, F11 fixed or documented as
known limitations in design 07.

## 5. Triage

Disposition of every finding, on branch `fix/spec-text` (the spec text)
and `fix/spec-code` (the code side of F1, F2, F7, F8, F10, owned by the
sibling worktree). Spec commits: `3d66d60` (identifiers, item grammar,
info string), `331bc1e` (row grammar, minors).

| Id | Disposition |
| -- | ----------- |
| F1 | deferred to `fix/spec-code` (printer escaping of `[=`, the smart symbols, `1\/2`). Spec side covered: §Round-trip now states the general rule (`331bc1e`) — the printer re-escapes the first character of anything a recogniser of the grammar or of the appendix would match — which includes the progress bar and the symbol spellings. |
| F2 | deferred to `fix/spec-code` (quote pairing, spans). Spec side fixed (`331bc1e`): the `"quotes"` row states SmartyPants' boundaries and that an orphan quote never pairs with the next phrase's. |
| F3 | fixed (`3d66d60`): §Identifiers gives `key` a digit-initial first character, the bracketed item recogniser finds the key by the production, so `@[7HA7H, p. 3]` is key plus locator; a bare `@` still needs a letter. C27 → code: `tmark-syntax::lower::head::is_ref_key` still requires a letter and falls back to the whole item (`{"key": "7HA7H, p. 3"}` today); fixture `reference-key-digit-initial` to add. |
| F4 | fixed (`3d66d60`): the family-4 recogniser ends with `(?:\s+(?&attrs))?`, and the prose says where the classes and id land and when the printer keeps the braces. C28 closed; `fence-attributes` agrees. |
| F5 | fixed (`331bc1e`): rung 5 of §Table states the row grammar (positional items over leaves, scalar and list fill the remaining leaves, rich cells span from the cursor, `~` on every absorbed slot, named rows, two-column minimum) and names the diagnostics and fixtures. C30 closed. |
| F6 | deferred (C54): the spec keeps "NFC-normalised"; the code adds the pass. |
| F7 | deferred to `fix/spec-code` (Typst labels of non-ASCII ids). |
| F8 | deferred to `fix/spec-code` (`ref-unnumbered`, including its spec sentence in §Anchor and its row in Appendix "Diagnostics"). |
| F9 | deferred (C55): the empty-link reference renders empty on the web and bare in LaTeX; a writer fix with backend blocks in `heading-implicit-id`. |
| F10 | deferred to `fix/spec-code` (Typst plain-float `::: figure`). |
| F11 | deferred (C56): `//` and `/*` in Typst prose. |
| F12 | fixed in text (`331bc1e`): consecutive orphans form one set; a `tab` without a title and attributes on `tabs` are silent. The dangling `tabs` label in the paged writers: C55. |
| F13 | fixed (`331bc1e`): a bar's `#id` defines no label; classes are forwarded and the print contract honours `thin` alone — the spec now says what the code does. |
| F14 | fixed in part (`331bc1e`): no nesting, one line. Whitespace loss: C57. |
| F15 | fixed (`331bc1e`): `icon-web-only` on every occurrence; the closing colon of an emoji must be followed by a non-alphanumeric. |
| F16 | fixed (`331bc1e`): `.unlisted` implies `.unnumbered`, as in Pandoc, which is what both paged writers do. |
| F17 | fixed (`331bc1e`): the HTML writer emits no id for an implicit id; a label is emitted in print only when referenced. |
| F18 | deferred (C58): design 04's profile table and `mkdocs.rs`. |
| F19 | deferred (C58): `latex render` should be `fence-unknown-node-word`. |
| F20 | fixed (`331bc1e`): the recogniser covers the fraction form; emoji and critic are appendix constructs and stay without a PCRE. |
| F21 | fixed (`331bc1e`): `directive-foreign` is a lint hint of `tmark check`. |
| F22 | fixed (`331bc1e`): `markdown="span"` and `markdown="1"` are the same `Div` of parsed blocks. |
| F23 | fixed (`331bc1e`): no space before the closing `$` either; `$5 and $6` is prose. |
| F24 | deferred (C56). |
| F25 | deferred (C59): fixtures to add. |
| K1 | closed by C51 (`4593a6b`): bare `@key` is the short form on every backend, `@[+key]` and `citations.narrative` the narrative one. |
