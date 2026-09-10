# Review 03 — Printer critic (`tmark-fmt`, `escape.rs`)

Mandate: `design/13-handoff.md` §Retrospective mandates, item 3. Crate under
review: `crates/tmark-fmt` at commit `f3443a6`, especially `src/escape.rs`,
with `attrs.rs`, `inline.rs` and `block.rs` where they print text without
going through `escape::text`. Nothing was modified; every finding below was
reproduced with `cargo run -q -p tmark-cli -- fmt FILE` and
`target/debug/examples/dump`, comparing IRs modulo `id`, `span`,
`Ref.bracketed` and `Caption.position` (the conformance runner's rule).
Scratch material: `/tmp/claude-1000/-home-ycr-tmark/2cbd51a1-bfb7-47e4-884e-62793ff9a596/scratchpad/printer/`
(`analyze.py` corpus run, `rt.py` round-trip harness, `necessity.py`,
`adv*/pc*.md` probes).

## Summary

- **Over-escaping is rare on real prose.** On 183 files (93 TeXSmith pages,
  `spec/tmark.md`, 89 `input`/`canonical` blocks of the fixtures) the printer
  adds 129 backslashes in 20 files; 87 of them are the three `=` of PyMdownX
  `=== "Tab"` lines, 14 are `\^` in undefined footnote references
  `[^key]`. Only 5 of the 129 are necessary (a backtick run in prose). The
  fear in the handoff (`\@`, `\#`, `\{` sprayed over prose) does not
  materialise on this corpus: zero `\@`, zero `\#`, four `\{` (all
  diagnostic-suppressing), zero `\_`, one `\*`, one `\&`, two `\!`.
- **Under-escaping is real and structural.** Nine distinct ways to make
  `parse(print(doc)) != doc` were found, all reachable from `parse` (not
  only from a hand-built IR). The worst is one root cause with five faces:
  the line-start rules of `escape.rs` fire on `out.at_line_start()`, which
  is false right after a list marker, `> `, `:   `, `[^1]: ` or `# `, so
  the first paragraph of a list item, block quote, definition or footnote
  and the content of a heading are printed with no block-start defence
  (`- \# x` prints `- # x`, a heading). Next: `|` inside code spans and
  column names of pipe tables, link destinations with spaces, `)` or a
  space in an anchor, attribute and fence-option values containing `"`,
  adjacent lists of the same kind, line breaks in pipe cells, numeric-looking
  YAML cells, `IndexEntry` followed by literal `[`, and `RawInline`
  arguments with unbalanced parentheses.
- The corpus itself: 182 of 183 files reach the fixed point; the one
  failure (`texsmith/examples/paper.md`) is finding U4 (fence option value
  with `"` and `}`), and it is also the only non-idempotent file.
- The "necessary" column below was measured, not guessed: each added
  backslash was deleted alone and (for paired delimiters) together with the
  others of its kind on the line, and the result re-parsed.

## 1. Over-escaping on the corpus

Method. `analyze.py` formats each file, diffs input and output line by
line, and locates the backslashes the printer inserted before punctuation
outside fences. For each one, three variants are re-parsed: the printed
text minus that backslash ("necessary" if the IR changes), minus every
added backslash of the same character on the same line ("jointly
necessary", for paired delimiters), and the diagnostics count is compared
("diagnostic-only" when the IR is equal but a warning or hint appears).

| escape | added | necessary | jointly necessary | diagnostic-only | unnecessary |
| ------ | ----- | --------- | ----------------- | --------------- | ----------- |
| `\=`   | 87    | 0         | 0                 | 0               | 87          |
| `\^`   | 14    | 0         | 0                 | 0               | 14          |
| `\[`   | 6     | 0         | 0                 | 0               | 6           |
| `\]`   | 6     | 0         | 0                 | 0               | 6           |
| `` \` `` | 6   | 5         | 0                 | 0               | 1           |
| `\{`   | 4     | 0         | 0                 | 4               | 0           |
| `\!`   | 2     | 0         | 0                 | 0               | 2           |
| `\&`   | 1     | 0         | 0                 | 0               | 1           |
| `\*`   | 1     | 0         | 0                 | 0               | 1           |
| `\#`, `\@` | 2 | (2)       |                   |                 |             |
| total  | 129   | 5         | 0                 | 4               | 118         |

The `\#` and `\@` rows are false positives of the line diff: both are
pre-existing escapes inside code spans of `spec/tmark.md` §Conformance
that moved when the table was re-aligned. The printer added no `\@` and no
`\#` anywhere in the corpus.

Representative examples (file, line of the printed output):

- `\=`, unnecessary (87): `texsmith/examples/letters.md:7` `\=\=\= "DIN
  (Germany)"`; `texsmith/examples/colorful.md:16` `\=\=\= "colorful.md"`;
  `texsmith/examples/paper.md:29` `\=\=\= "Article"`. PyMdownX tabs are not
  TMark, the line is `Str`, and `==` is escaped because the rule fires on
  any run of two. No `==` closer exists on the line; the parser does not
  produce `Mark` for `=== "x"`, escaped or not.
- `\^` and `\[`, unnecessary (14 + 6): `texsmith/assets/examples/cheese.md:40`
  `attributes \[\^Prentice1993].`; `:95` `Mozzarella \[\^1] exhibit`;
  `texsmith/syntax/supported.md:65` `\^\^inserted\^\^` (raw HTML cell).
  The footnote definitions are absent (they live in the `.bib`), so `[^key]`
  is `Str` either way; and the `[` rule and the `^` rule both fire on the
  same pair, so even a real footnote would be escaped twice.
- `\]`, unnecessary (6): `texsmith/guide/features/bibliography.md:45` `See
  the [academic paper\][cheese] example`; `texsmith/about/contribute.md:18`
  `[Release Notes\][releasenotes]`; `texsmith/syntax/references.md:195`
  `[Index / Tags\][index-tags]`. Undefined reference links; the `]`
  before `[` is escaped, the opening `[` is not, so the escape defends
  nothing and looks odd.
- `` \` ``, necessary (5) and unnecessary (1): `texsmith/syntax/supported.md:41`
  ``backticks (\`\`\`\` \`\`) are the recommended way``: the six backticks
  are literal text in the source (a run of 4 then 2 does not form a code
  span); removing any one of the first five changes the parse, the last one
  is redundant.
- `\{`, diagnostic-only (4): `texsmith/examples/book.md:1` `[]()\{#einstein}`
  (source `[](){ #einstein }`, an attribute list with no host: the parser
  keeps it as `Str` and warns; unescaped it warns again). Justified.
- `\!`, unnecessary (2): `texsmith/guide/fragments/geometry.md:24` `broadest
  consensus\!`; `texsmith/about/contribute.md:9` `[hear it](…)\!`. The `!`
  is the last character of the block; `inlines()` passes `next = None` for
  the last inline and the rule reads `None` as "an inline follows".
  Minimal input: `wow!` prints `wow\!`; likewise `I love C#` prints
  `I love C\#` and `ping me at @` prints `ping me at \@`. This is the one
  over-escape that will hit ordinary prose (every paragraph ending in `!`).
- `\&`, unnecessary (1): `texsmith/about/devel/index.md:149` `diagrammes
  b\&w`. `&w` is not an entity (no `;`); `AT&T` prints `AT\&T`.
- `\*`, unnecessary (1): `texsmith/syntax/supported.md:49` `\*[HTML\]:
  HyperText Markup Language` inside a raw-HTML `<code>` cell.

Over-escapes measured on synthetic prose (`adv2/pc-*.md`, one-at-a-time and
joint removal), none present in the corpus but each a pattern a user will
type:

| input `Str`         | printed              | needed?                         |
| ------------------- | -------------------- | ------------------------------- |
| `@x`, `@2024`, `@`  | `\@x`, `\@2024`, `\@` | no: key needs `[A-Za-z][\w:.-]*[A-Za-z0-9]`, two characters, letter first |
| `{foo} b`           | `\{foo} b`           | no IR change, no diagnostic     |
| `{1x}`, `{-x}`      | `\{1x}`, `\{-x}`     | no: neither a role head (letter first) nor an attribute list |
| `#{x}`              | `\#\{x}`             | one of the two suffices         |
| `#*em*`             | `\#*em*`             | no: `#` before an `Emph`, `next = None` |
| `#[not an index`    | `\#[not an index`    | no closing bracket              |
| `a == b`, `c ++ d`, `i++` | `a \=\= b`, `c \+\+ d`, `i\+\+` | no: no closer in the run |
| `x^2 and y^3`, `~5 and ~6` | `x\^2 and y\^3`, `\~5 and \~6` | not individually; see P1 below |
| `x<y`, `AT&T`       | `x\<y`, `AT\&T`      | no                              |
| `[^note]` (undefined) | `\[\^note]`        | no, and doubled                 |
| `[x] task` in a paragraph | `\[x] task`    | no: task markers exist in list items only |
| `\| not table` at block start | `\| not table` | no: no delimiter row follows |
| `x_ and _x`         | `x\_ and \_x`        | no: closer before opener        |

## 2. Under-escaping: round-trip failures

Ranked by severity; all break `parse(print(doc)) == doc`. "Reachable"
means the IR in question is produced by `parse` on the given input.

### U1. Block-start rules are keyed on the output column, not on the block (five contexts)

`escape::text` computes `line_start = out.at_line_start()` for the first
character of the run; `line_start_escape` runs only when it is true. After
`- `, `1. `, `> `, `:   `, `[^1]: ` or `# ` the column is not zero, so the
whole family of block-start escapes (`#`, `>`, `-`, `+`, `*`, `1.`, `:::`,
`:   `, `!!!`, `Table:`, `[x] `) is skipped. Table cells escape correctly
only because `cell_text` renders into a fresh `Out` at column 0.

| input (Str after the marker)      | printed              | re-parsed as                    |
| --------------------------------- | -------------------- | ------------------------------- |
| `- \[x] not task`                 | `- [x] not task`     | task item, text `not task`      |
| `- \# heading in item`            | `- # heading in item` | `Header` inside the item        |
| `- 1\. number in item`            | `- 1. number in item` | nested `OrderedList`            |
| `- \- dash in item`               | `- - dash in item`   | nested `BulletList`             |
| `- \> quote in item`              | `- > quote in item`  | `BlockQuote` in the item        |
| `1. \# in ordered item`           | `1. # in ordered item` | `Header`                      |
| `- a\n  - \[ ] nested`            | `  - [ ] nested`     | task item                       |
| `Term\n:   \# in definition`      | `:   # in definition` | `Header` as the definition     |
| `> \# in quote`                   | `> # in quote`       | `Header` in the quote           |
| `[^1]: \- not a list`             | `[^1]: - not a list` | `BulletList` as footnote body   |
| `# \# in heading`                 | `# # in heading`     | heading text `in heading`       |

Probes: `adv2/pc-listitem.md`, `adv3/pc3-blockstart-contexts.md`,
`adv4/pc4-quote-blank.md`, `adv4/pc4-footnote-body.md`. The second
`Str` of `> \# c\n> \- d` (after a `SoftBreak`) *is* escaped, because the
printer is back at column 0 of its own buffer before the `> ` prefix is
applied: the prefix mechanism and the line-start test disagree.

Fix: `let mut line_start = ctx.block_start || out.at_line_start();` at the
top of `text()` (with `block_start` meaning "first inline of the block",
which `inlines()` already computes) and pass `block_start: true` for
`Header` content. Fixtures: `escape-block-start-list.md`,
`escape-block-start-quote.md`, `escape-block-start-definition.md`,
`escape-block-start-footnote.md`, `escape-heading-hash.md`.

### U2. `|` inside code spans and column names of pipe tables

GFM splits cells on every unescaped `|`, code spans included; the only
spelling is `` `a \| b` ``. `code_span()` does not know `in_cell`, and
`pipe_table` prints `Column::Leaf.name` raw.

| input                                          | printed                | re-parsed                        |
| ---------------------------------------------- | ---------------------- | -------------------------------- |
| ```` ```yaml table\ncolumns: [A, B]\nrows:\n  - ["`a \| b`", x]\n``` ```` | `` \| `a \| b` \| x \| `` | three cells `` `a ``, `` b` ``, `x` |
| `\| `a \\\| b` \| x \|` (pipe source)          | same                   | same                             |
| `\| a \\\| b \| c \|` header                   | `\| a \| b \| c\|d \|`  | header has 3 cells, delimiter row 2: the table becomes a `Para` |
| `\| \\*b\\* \| \\`d\\` \|` header             | `\| *b* \| `d` \|`      | column names `b`, `d`            |

Probes: `adv2/pc-code-pipe-cell.md`, `adv2/pc-code-pipe-cell2.md`,
`adv3/pc3-cell-pipes.md`, `adv4/pc4-header-pipes.md`. The second row also
shows that column names are stored as plain strings by the parser (`*a*`
becomes `a`, `` `c` `` becomes `c`), which is a parser/IR question
(review 04), but whatever the type, the printer must escape `|` and the
Markdown punctuation of a column name as it does for a cell.

Fix: pass `in_cell` to `code_span` (replace `|` by `\|` inside the span;
GFM decodes it) and run column names through `escape::text` with
`in_cell: true`. Fixtures: `table-pipe-in-code.md`,
`table-pipe-in-header.md`.

### U3. Link destinations and anchors are printed bare

`link()` pushes `Target::Url`/`Document` verbatim and `#` + `Anchor`.
CommonMark needs `<…>` for a destination with spaces and `\)` for an
unbalanced `)`.

| input                | IR                           | printed        | re-parsed                          |
| -------------------- | ---------------------------- | -------------- | ---------------------------------- |
| `[a](<b c>)`         | `Link` → `Url "b c"`         | `[a](b c)`     | `Str "[a](b c)"`                   |
| `[d](e\)f)`          | `Link` → `Url "e)f"`         | `[d](e)f)`     | `Link` to `e`, then `Str "f)"`     |
| `[a](<#b c>)`        | `Link` → `Anchor "b c"`      | `[a](#b c)`    | `Str`                              |
| `![alt](<p q.png>)`  | `Image src "p q.png"`        | not probed; same code path |                       |

Probes: `adv2/pc-link-dest-space.md`, `adv3/pc3-link-dest-more.md`. Titles
are fine (`"t \"q\" t"` round-trips). Fix: print `<dest>` when the
destination contains whitespace, `<`, `>` or unbalanced parentheses (or
always `<…>` when it contains anything but URL-safe characters), and
escape `\(`/`\)`. Fixture: `link-destination-escapes.md`.

### U4. Attribute and fence-option values containing `"`

The lexical grammar has `"[^"]*"` for a quoted value: there is no escape
for `"`. `attrs::value` and `block.rs` (`CodeBlock` options) print
`v.replace('"', "\\\"")`, a convention the grammar does not have; the
parser, on its side, keeps a `\"` it meets verbatim and does not end the
value there. The two conventions are not inverses: each print doubles the
backslashes.

| input                                   | IR value          | printed                         | re-parsed value       |
| --------------------------------------- | ----------------- | ------------------------------- | --------------------- |
| `# H {#id key="a \"b\" c"}`             | `a \"b\" c`       | `{#id key="a \\"b\\" c"}`       | `a \\"b\\" c`; second print `\\\"` |
| ```` ```yaml {.snippet caption="Download PDF"} ```` (TeXSmith `paper.md`) | `caption` = `"Download PDF"}` | ```` ```yaml caption="\"Download PDF\"}" ```` | `""Download` |
| `!!! note "Title with \"quote\""`       | title `Title with "quote\` | `{title="Title with \"quote\\"}` | equal by accident, idempotent |

Probes: `adv/attr-quote.md`, `adv2/pc-fence-opts.md`,
`adv3/pc3-admon-title.md`; corpus: `texsmith/examples/paper.md` (the only
corpus file that fails the fixed point and idempotence). The second row is
also a parser leniency (an attribute list on a fence info string is
accepted as `key=` followed by the rest of the line); the spec grammar
would reject it and the value would never contain `"`. Decision needed in
the spec: either `"` is unrepresentable in a value (then the parser must
not produce it and the printer must not pretend to escape it) or the
grammar gains `\"` (then the parser decodes it). Fixtures:
`attributes-value-quote.md`, `fence-options-quote.md`.

### U5. Adjacent lists of the same kind merge

CommonMark continues a list across a blank line when the next marker is of
the same type; two `BulletList`s (or two `OrderedList`s) printed one after
the other become one. Reachable: `- a\n\n* b` (different bullet
characters), `1. c\n\n1) d` (different delimiters), `2024. year\n\n1.
item` (the printer renumbers the second list on the second pass, so the
output is not idempotent either).

| input                 | IR                               | printed              | re-parsed                     |
| --------------------- | -------------------------------- | -------------------- | ----------------------------- |
| `- a\n\n* b`          | two `BulletList`                 | `- a\n\n- b`         | one list, two items           |
| `1. c\n\n1) d`        | two `OrderedList`                | `1. c\n\n1. d`       | one list; second print `1. c\n2. d` |

Probe: `adv3/pc3-lists-adjacent.md`, `adv2/pc-blockstart-unescaped.md`.
Not an escape but the same contract. Fix: between two consecutive lists of
the same node type, emit a separator the IR does not carry (the CommonMark
idiom is `<!-- -->`, which TMark has as `Comment`; or alternate `-`/`*`
and `.`/`)`, which the IR also does not carry). Fixture:
`lists-adjacent.md`.

### U6. Line breaks inside pipe-table cells

A cell whose content holds a `SoftBreak` or `LineBreak` (reachable through
a `yaml table` whose model is plain) is printed as a pipe table with a raw
newline inside the row.

| input                                                       | printed                     | re-parsed                          |
| ----------------------------------------------------------- | --------------------------- | ---------------------------------- |
| ```` ```yaml table\ncolumns: [A, B]\nrows:\n  - ["a  \nb", "c\nd"]\n``` ```` | `\| a\\\nb \| c\nd \|` | rows `a\`, `b \| c`, `d`; second print differs again |

Probe: `adv3/pc3-pipe-linebreak.md`. In the YAML form (`adv2/pc-yaml-md.md`,
a model with a span) the break is printed as a literal newline inside a
double-quoted flow scalar, which YAML folds to a space: `line\nbreak` comes
back as `Str "line break"`. Fix: a model with a break in a cell is not
"plain" (use the YAML form) and `yaml_scalar` must write `\n` for a
newline. Fixture: `table-cell-linebreak.md`.

### U7. Numeric and null-looking YAML cells

`yaml_scalar` leaves anything not in its stop-list plain; YAML then types
it. Reachable through any `yaml table` with a span (the model is not plain,
so it prints as YAML again).

| cell text | printed | re-parsed |
| --------- | ------- | --------- |
| `1.10`    | `1.10`  | `1.1`     |
| `+1`      | `+1`    | `1`       |
| `0x10`    | `0x10`  | `16`      |
| `0o17`    | `0o17`  | `15`      |
| `1e3`     | `1e3`   | `1000.0`  |
| `NULL`    | `NULL`  | absorbed slot (`~`) |

Probe: `adv2/pc-yaml-scalars.md`. `007`, `1_000`, `.inf`, `NaN`,
`2024-01-01`, `12:30:00`, `y`, `n`, `Yes`, `NO`, `ON`, `Off` survive with
the `serde_yaml` in use, but that is the library's schema, not a rule. Fix:
quote every scalar that `serde_yaml` would not read back as the same
string (simplest: quote when the text is not `[A-Za-z][A-Za-z0-9 _-]*`
after the existing checks, or always double-quote and stop guessing).
Fixture: `fence-yaml-table-scalars.md`.

### U8. `IndexEntry` followed by literal `[`

`{index}` is the one role that takes several groups. A `Str` starting with
`[` after an `IndexEntry` is absorbed as the next group.

| input        | IR                                  | printed          | re-parsed                          |
| ------------ | ----------------------------------- | ---------------- | ---------------------------------- |
| `#[a]\[b]`   | `IndexEntry [a]`, `Str "[b]"`       | `{index}[a][b]`  | `IndexEntry [a][b]`, one node      |

Probe: `adv3/pc3-index-adjacent.md`. `{sc}[a]\[b]`, `@key\[x]` and
`{counter}(fw:k)\(x)` are fine (single group, single argument). Fix: the
`[` rule must also fire when the previous inline is an `IndexEntry` (the
`inlines()` loop knows it; `escape::text` only sees `prev` as the last
character, `]`). Fixture: `role-index-then-bracket.md`.

### U9. `RawInline` arguments with unbalanced parentheses

The role argument grammar is balanced parentheses, no escape (`\)` is
literal: `{raw latex}(a\)b)` parses as `RawInline "a\"` then `Str "b)"`,
`adv4/pc4-raw-inline-parens.md`). Inline HTML is lowered to `RawInline
html` and printed as `{raw html}(…)`; an HTML attribute containing `)` or
`(` is therefore unprintable.

| input                                | printed                                      | re-parsed                              |
| ------------------------------------ | -------------------------------------------- | -------------------------------------- |
| `a <span title="x)">b</span>`        | `a {raw html}(<span title="x)">)b{raw html}(</span>)` | `RawInline "<span title=\"x"`, `Str "\">)b"` |
| `<span title="(">d</span>`           | `{raw html}(<span title="(">)d{raw html}(</span>)` | dangling head; text swallows `)d{raw html}(</span>` |

Probe: `adv4/pc4-html-parens.md`. Fix at the printer: fall back to the
fenced form for an unbalanced text? There is no inline fenced form. This
is a spec gap (§Lexical grammar: "balanced parentheses allowed inside",
nothing for unbalanced ones); the spec must either add `\(`/`\)` escapes
to the argument grammar (and the parser decode them) or state that inline
HTML with unbalanced parentheses is not representable. Fixture once
decided: `role-raw-argument-parens.md`.

### U10. `www.` autolink literals are not printed bare

`link()` prints a link bare only when `url == text`; for `www.example.com`
the parser stores `http://www.example.com`, so the printer writes
`[www.example.com](http://www.example.com)`, and the parser (GFM autolink
literal inside link text, a `tmark-markdown` quirk) re-parses the text as a
nested `Link`. Not idempotent: the third form is
`[[www.example.com](http://www.example.com)](http://www.example.com)`.
Probe: `adv2/pc-autolink-str.md`. Fix: `bare` when `url ==
format!("http://{text}")` and the text starts with `www.`. Fixture:
`autolink-www.md`. (The nested-link parse is for the parser adversary.)

### Parser-side observations met on the way (not printer bugs, listed for review 01/02)

- `\[x\]{#id}` yields two adjacent `Str` nodes (`…[x]`, `{#id}`): the
  hostless-attribute fallback is not merged with its neighbour, so the
  printed text (one `Str`) fails the fixed point although nothing is wrong
  with the escaping (`adv2/pc-bracket-cases2.md`).
- `costs $5 and $6` parses as `Str "costs "`, `Math "5 and "`, `Str "6 and
  …"`: the dollar rule has no Pandoc flanking (closing `$` followed by a
  digit). The printer's "always escape `$`" is justified by this looseness.
- `x^2 and ^x^` parses as `Sup "2 and "`: same for `^` and `~`
  (`adv2/pc-tilde-caret.md`); the printer's "always escape" is justified
  by the parser, not by PyMdownX's rules.
- `# # foo` parses as heading text `foo` (CommonMark: `# foo`).
- `!!! not admon` produces `Admonition kind="not"`, printed `::: not
  {.admon}`, re-parsed as `Div` (unknown container): the `!!!` reader accepts
  any kind, the `:::` reader does not (`adv2/pc-blockstart-unescaped.md`).
- Cells of a `yaml table` whose Markdown is a non-paragraph block (`- x`,
  `1. num`, `> quote`, `# heading`, `Table: cap`, `<!-- c -->`) come out
  empty; a cell `-` disappears entirely (`adv2/pc-yaml-md2.md`).
- A `:::` line interrupts a paragraph and swallows the rest of the document
  when unclosed; `para line\n::: fence continuation` yields a `Div` containing
  everything after it.
- The fence info string `yaml {.snippet caption="Download PDF"}` is accepted
  and produces an option value with `"` and `}` (U4).

## 3. Cosmetic

- P1. `=`, `+`, `~`, `^`, `` ` ``, `$` and single `*`/`_` are escaped without
  looking for a closer. Since the parser needs a closing run in the same
  paragraph, a "closer exists later in this `Str`" check would remove all
  87 `\=` of the corpus and every `\^`/`\~` in `x^2`, `~5`. It must be
  joint (both of `x^2 and y^3` unescaped *is* a superscript with this
  parser), so the rule is: escape the k-th delimiter only if a later one
  exists in the run. Conservative and cheap.
- P2. End-of-block `!`, `#`, `@` (`wow\!`, `C\#`, `\@`): `inlines()` should
  pass `Some('\n')` (the block end) instead of `None` for the last inline;
  `text()` already treats `\n` as safe for those rules.
- P3. Doubled escapes: `\[\^1]`, `\#\{x}`, `` \`\` `` (second backtick),
  `\[\@k]`. One backslash per construct is enough; the second is noise.
- P4. `@` before a one-character or digit-leading key (`\@x`, `\@2024`)
  and `{` before a non-identifier (`\{1x}`, `\{-x}`, `\{foo}` with no
  bracket after): the rules could mirror the grammar's first character
  classes.
- P5. `&` before alphanumerics without a `;` later in the run (`AT\&T`).
- P6. `]` before `[` or `:` when the document has no matching definition
  (`[paper\][cheese]`): the printer could consult `Document` for reference
  definitions, but the IR does not keep them (they are resolved by the
  parser), so this one is a judgement call; leaving it is defensible.
- P7. `# {#only}` prints `#  {#only}` (two spaces) and an empty heading
  prints `# ` with a trailing space (`adv3/pc3-heading-trailing.md`).
- P8. A tight list with a nested list is printed loose (`- a\n\n  - b`):
  the IR has no tightness, so this is a normalisation, but it changes
  rendering in every backend (`adv3/pc3-blockstart-contexts.md`).
- P9. `{code python}[…]` prints `{code lang=python}[…]` while the fixture
  and spec use the positional form.

## 4. `escape.rs` rule by rule

"Spec" cites `spec/tmark.md`; "CM" is CommonMark/GFM; "parser" means the
rule is justified only by what `tmark-syntax` does today.

General rules (`general_escape`):

| rule | note |
| ---- | ---- |
| `\` before punctuation or `\n`, or before an unknown inline | CM §Backslash escapes; hard line break. Justified. `next = None` also escapes (`\\*em*`), harmless. |
| `*` unless between two whitespace | CM delimiter runs. Justified; conservative (P1). |
| `_` unless intraword or between whitespace | CM (`_` is not intraword). Justified, conservative. Also defends X1 `__x__` (spec §Deprecations "small caps sugar"). |
| `` ` `` always | CM code spans. Justified. |
| `~` always | Appendix PyMdownX (`~~del~~`, `~sub~`) and the parser's loose flanking. Justified by the parser only. |
| `^` always | same (`^sup^`); also `[^1]` (CM footnotes). Same. |
| `$` always | §Math; justified by the parser's dollar rule (no Pandoc flanking). |
| `=`, `+` when doubled | Appendix PyMdownX `==mark==`, `++keys++`. Justified in principle; fires without a closer (87 corpus hits, P1). |
| `<` before alnum, `/`, `!`, `?` | CM raw HTML, autolinks. Justified, conservative (`x<y`). |
| `[` in a group, or before `^` or `@` | Spec §Lexical grammar: role content `[^\[\]\\]`, "any backslash-escaped bracket inside role content"; CM footnotes; Pandoc `[@key]`. Justified. Misses `[` after `IndexEntry` (U8). |
| `]` in a group, or before `(`, `{`, `[`, `:` | Spec (groups), CM (links, definitions), §Attributes (`]{`). Justified, though `]` before `[` or `:` without a definition is unnecessary (P6). |
| `!` before `[` or `None` | CM images. `None` means "an inline follows" but also "end of block" (P2): half justified. |
| `{` before an identifier start, `_`, `-`, `#`, `.`, `{` | Spec §Lexical grammar: role head `[A-Za-z]`, attribute list `#`, `.`, `[\w-]+=`; moustache `{{`. Digits, `_` and `-` as first character are not in the grammar (over-escape, P4); a role head without a following `[`/`(` is literal anyway. |
| `@` before `[`, alnum or `None`, unless guarded | Spec X4 look-behind `(?<![\w@/:.-])`. The guard set matches the spec exactly. Look-ahead is wider than the key grammar (`[A-Za-z][\w:.-]*[A-Za-z0-9]`): `\@x`, `\@2024`, `\@` at end (P4, P2). |
| `#` before `[`, `(`, `{` or `None` | Spec §Two sigils and X5 (`#[`, `#(`; `#{` withdrawn). `None` case over-escapes (`C\#`, `\#*em*`). |
| `&` before alnum or `#` | CM entities. Over-escapes without a `;` (P5). |
| `\|` in a cell | GFM tables. Justified; not applied to code spans or column names (U2). |

Line-start rules (`line_start_escape`), applied when `out.at_line_start()`:

| rule | note |
| ---- | ---- |
| `#` before space, tab, `#`, `\n` or end | CM ATX headings. Justified. |
| `>` always | CM block quotes. Justified. |
| `\|` always | GFM tables. Over-escape unless a delimiter row follows; cheap to keep. |
| `-`, `+`, `*` before whitespace, doubled, or uniform line | CM lists and thematic breaks. `--` doubled is not a construct (`-- x`), minor. |
| `=`, `_` uniform line | CM setext underline and `___` break. Justified (the general `=` rule fires on `===` anyway). |
| `:` before space, tab or `:` | Spec §Container fence `^:{3,}`, definition lists `:   `. Justified. |
| `!` before `!`; `?` before `?` | Appendix PyMdownX `!!!`/`???` admonitions. Justified. |
| `[` + ` `/`x`/`X`/`.` + `]` + whitespace, block start only | GFM task items. `.` is not a task state (parser accepts `[.]`?: `adv2/pc-listitem-raw.md` shows `[.] weird` stays text). Fires in paragraphs, where no task item exists (over-escape); does *not* fire in list items because of U1. |
| digits (≤ 9) + `.`/`)` + whitespace: escape the delimiter | CM ordered lists. Justified. |
| `Table:`, `Figure:`, `Listing:` + whitespace, block start only | Spec §Lexical grammar caption line. Justified; the caption regex needs a paragraph of its own, so continuation lines are right not to escape. |
| gate: `out.at_line_start()` | Unjustified: the spec's block starts are about the block, not the column (U1). |

Outside `escape.rs`, text is printed with no escaping at all in:
`attrs::value` (`"` handling, U4), `block.rs` `CodeBlock` options (U4),
`pipe_table` column names (U2), `code_span` (`|`, U2), `link()`
destinations and anchors (U3), `RawInline` and `{include}` arguments
(U9; `{include}(a b.md)` round-trips, `{include}(a)b.md)` cannot),
`Comment` text (`-->` unreachable from parse), `yaml_scalar` (U7),
`container` attribute values (through `attrs::value`, U4).

## Proposed fixtures (names)

`escape-block-start-list`, `escape-block-start-quote`,
`escape-block-start-definition`, `escape-block-start-footnote`,
`escape-heading-hash`, `table-pipe-in-code`, `table-pipe-in-header`,
`link-destination-escapes`, `attributes-value-quote`,
`fence-options-quote`, `lists-adjacent`, `table-cell-linebreak`,
`fence-yaml-table-scalars`, `role-index-then-bracket`,
`role-raw-argument-parens` (after the spec decision), `autolink-www`,
`escape-block-end-punctuation` (P2), `escape-delimiter-without-closer`
(P1, a `canonical` block that must *not* contain `\=`).
