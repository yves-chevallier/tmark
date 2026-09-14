# 08 — Spec consistency review (draft 3, `spec/tmark.md`)

Lens: the spec text as a piece of language design and technical writing —
internal contradictions, ambiguity, completeness, coherence with the
principles, writing quality, fitness for 0.1. Spec-versus-code conformance
(C27–C50) is review 07 and is not repeated here; the CLI was used only to
test whether the spec's own examples parse and which way the implementation
resolved an ambiguity the text leaves open. Line numbers refer to
`spec/tmark.md` at commit `778e539` (1988 lines).

Severity: **blocking** = the text cannot be published as the source of truth
with this in it; **major** = an implementer or an author would get it wrong;
**minor** = fix when the section is next touched.

## 1. Findings

| Id | Section | Severity | Claim | Lines |
| -- | ------- | -------- | ----- | ----- |
| B1 | Preamble, §Catalogue, throughout | blocking | The document says its own normative wording is "aspirational" and marks constructs by what TeXSmith-Python ships; the shipping/proposed axis is stale and foreign to a spec that is the source of truth for tmark. | 14–19, 46–49, 106, 269, 490, 511, 556, 580–581, 655, 870, 926, 1022, 1067, 1126, 1171, 1480 |
| B2 | §Registries (lookup), §Cite, §Anchor, §Header, §Cross-document | blocking | The `@key` lookup rule is stated twice (prefix → counter, otherwise bibliography) and contradicted by three sections: unprefixed labels (`@stock`), implicit heading ids (`@boot-sequence`), three-segment cross-document keys (`@fwrev:fw:x`), and span anchors with an undeclared head (`@claim:one`, C24). | 346–352, 1016–1018 vs 912–914, 617, 1626, 224–226 |
| B3 | §Lexical grammar, §Ref, §Anchor, §Counters | blocking | The grammar of a key / id / prefix is stated seven times, six ways; the canonical DOI example cannot match the bare-reference recogniser; a colon key is legal as an attribute id and illegal as a counter key. | 365, 378, 417, 426, 434, 440, 938–939, 1513; example 1009 |
| B4 | §Cite | blocking | Narrative versus parenthetical is keyed on "the reference contains a space", which is a lexical fact, not a semantic one; `@[ein05]` is then undefined, and the planned change (bare `@key` = short form, front-matter switch to narrative) makes sentence 995–996 false and leaves no per-citation narrative spelling. | 937–941, 995–1002, 1016–1018 |
| B5 | §Two sigils, X5, §CounterItem, App. draft 2, App. deprecations | blocking | `#{prefix:key}` is in three states at once: withdrawn (336–338, 485), accepted-and-deprecated sugar (1037–1038, 1954), "removed, never recommended" (1779–1781); and draft-2 item 1 still says the standalone define is `#[…]` "shared by index entries and counter items". | 336–338, 485, 1037–1038, 1705–1711, 1779–1781, 1954 |
| M1 | §Two sigils vs §IndexEntry / §CounterItem | major | The sigil section presents `#[…]` and `#(…)` as *the* forms; the catalogue makes the roles canonical and the sigils sugar (C6, still open in the text). | 320–338 vs 1035, 1052 |
| M2 | §Attributes (hosts) | major | The host list omits display math, block quotes, paragraphs and lists, yet the spec attaches attributes to all four; whether whitespace may separate an inline host from its `{…}` is unstated (the CLI treats `[x] {#id}` as no host). | 223–231 vs 243, 648, 908, 1256 |
| M3 | §Roles, §Lexical grammar, §Inline | major | "Brackets hold content parsed as Markdown" contradicts `{code py}[…]` and `{keys}[…]` (verbatim); the content recogniser forbids nested brackets, so a link inside an aside is impossible, and whether `\[` survives into the Markdown pass is unstated. | 203–211, 379, 385, 443, 759, 761, 879–880 |
| M4 | §Conformance, §Para, §Features, §Roadmap | major | The `strict` profile is defined three ways (X1+X2 off; "rewrites X-class constructs"; also flips `paragraph.lead` and `compat.pymdownx`), and the base profile is `default` in one section and `canonical` in another. | 490–495, 636–638, 1651, 1673–1676, 1829–1831 |
| M5 | §Front matter | major | "Unknown keys fail at parse time" contradicts the shared-namespace rationale (C9); the list of keys TMark reads omits `lang`, `id`, `epigraph`; fifteen `press.*` keys are cited across the text and none has a declared type or default. | 501–509, 556–557, 567 vs 591, 600, 680, 737, 791, 886, 966–972, 1241, 1282, 1288, 1868 |
| M6 | Diagnostics (everywhere) | major | Twelve diagnostic names are introduced inline with no table; severities are "warns", "hint", "error", "hard warning", "class D error", "warns loudly" with no defined scale. | 621, 734, 738, 788, 845, 1018, 1107, 1350, 1384, 1515, 1592 |
| M7 | §Caption, §Table | major | The canonical table order (table, `table-config` fence, caption) is unreachable under the attachment rule, where a code block is a float; `Figure:`/`Listing:` before their float and a kind/host mismatch are unspecified (C18); "a bare image stays inline" does not say what bare means. | 1095–1108 vs 1197–1198, 1084–1088, 1099–1101 |
| M8 | §Lexical grammar | major | The recognisers, declared normative, disagree with the prose next to them and with §Attributes: `\S+` values (C1/C2), caption line accepts only `{#id}` (C23), container attrs cannot hold a quoted `}`, the info string has no brace attribute list (C28). | 365, 369–370, 378, 394, 409, 440 vs 232–247 |
| M9 | §Anchor | major | A prefix that disagrees with its host is "linted" and, one sentence later, is how a heading is numbered in a custom series; which of the four heading prefixes "agrees" with a heading is unstated. | 912–922, 1475 |
| M10 | §Inline (table) | major | The Class column mixes the class of the canonical spelling and of its sugar; `Underline` is D here and E in the appendix; `'x'` is a canonical `Quoted` here and "left alone" in the appendix; `Span` has three canonical spellings (`[x]{attrs}`, an icon shortcode, critic markup) and the row states one. | 749–763, 843–844, 1890, 1900, 1937–1938 |
| M11 | §CounterItem (example) | major | The example defines `fw:boot-loop` twice (a counter item and a heading anchor) and uses an undeclared `n` prefix; `tmark check` reports `label-duplicate` then `ref-unresolved` on it. The "Boot loop" heading is inside its fence, contrary to appearance. | 1025–1033 |
| M12 | P6, App. PyMdownX, App. deprecations | major | "No sugar is accepted without a horizon" is contradicted by the compatibility table (no horizons), by `\(…\)`/`\[…\]` (no row), and by the sigil sugar (no row). | 95–97, 795, 1249, 1871–1878, 1950–1980 |
| M13 | Pure-core boundary | major | Twelve behaviours that need a file, a process, a clock or the network are stated as language behaviour with no "processor-defined" marker: image conversion, `python image` execution, DOI and Wikipedia fetches, inventory publishing, `date: commit`, `.bib` on the command line, a CLI flag, first-heading promotion, Python's format mini-language. | 521, 591–593, 1121–1123, 1171–1173, 1010–1011, 1500, 1525–1530, 1602–1604, 1614–1615, 1976 |
| M14 | Round-trip, escapes | major | The printer's escaping rules are stated for `\@`, `\#` and brackets in role content only; nothing says how a literal `{aside}[x]`, `#[x]`, `:smile:`, `[=45% "x"]`, `^^x^^` or a prose paragraph beginning `Table:` next to a table is written so that it stays text (the CLI escapes `\{`, `\#`, `\@`, `\^`; the `Table:` case is silent). | 139–148, 443–444, 1102–1107 |
| M15 | §Catalogue (completeness) | major | No node has its IR fields named; `Link`, `Image`, `Str`, `SoftBreak`, `LineBreak`, `Abbr`, `Math`, `Note`, `Table`, `Figure`, `Caption`, `Ref`, `Cite`, `Include` and the wiki-link `Link` have no entry or no fields; `DefinitionList`, lists, `BlockQuote`, `Note`, inline `Math` and acronyms have no backend mapping (or LaTeX only). | 575–581, 651–666, 793–798, 867–872, 1548–1558, 1895 |
| m1 | §Admonition | minor | The built-in type list is presented as closed but `!!! type` sugar takes any type (the CLI accepts `example`); the default title when `title=` is absent is unstated. | 1269–1283, 1884 |
| m2 | §CodeBlock | minor | "Class E" for the whole node, though a plain fenced block is class C. | 1229–1245 |
| m3 | §Note | minor | "class C (GFM) and E" — pick one; the `note` prefix row implies `@note:…` references with no defined key. | 869, 1481 |
| m4 | §Index / §Lexical grammar | minor | `#[…]` nests to three levels, `{index}[…]` unboundedly. | 379, 433, 1046–1052 |
| m5 | §Attributes | minor | "any number of items" versus a recogniser requiring one; `{}` undefined. | 174, 365 |
| m6 | §Features | minor | "Nothing else in the front matter toggles a feature", but `press.details`, `press.comments`, `press.numbered` toggle behaviours; the line between form key and feature is not drawn. | 1636–1638 |
| m7 | §Round-trip, §Front matter | minor | Entities (`&nbsp;`) are "accepted" but the printer's treatment is unstated (C22); emoji normalise to the character, which is fine but should sit next to the escapes list. | 571, 827–829 |
| m8 | §Para | minor | Lead-in HTML is `<b class="lead">` while `Strong` maps to `<strong>`. | 644, 752 |
| m9 | App. PyMdownX | minor | `[[Page Title]]` wiki links have no section, no canonical spelling and rely on project-file resolution (I/O). | 1895 |
| m10 | §Ref | minor | `[](other.md)` needs an inventory (C12) and `[text](other.md)` — textual cross-document reference or plain link — is undefined. | 935, 954–961 |
| m11 | Order | minor | §Tabs cites "the `tsdiv` contract of §Div" before §Div defines it; §Foreign directive sits under Structure and is referenced from Containers and the lexical grammar. | 1344 vs 1373; 724, 398 |
| m12 | §Ref (lint) | minor | "`tmark lint` flags" position words; on the spec itself the rule yields 16 hints on ordinary prose ("the reasoning above"). Say it is a hint and scope it to a sentence that also mentions a float. | 978–980 |
| m13 | §Counters | minor | "not add a prefix that shadows a role name" — the two grammars are disjoint, the reason is missing; case-sensitivity of ids and keys (prefix case-insensitive, `@Fig:` capitalises) is unstated for the key part. | 942–945, 1511–1513 |
| m14 | §Image, Figure | minor | `cols=`/`rows=` defaults; what a `figure` container holding both a caption-line id and image ids does when both name the same float. | 908–910, 1140 |
| m15 | App. critic, X3, smart symbols | minor | Precedence of `{--x--}` / `{~~a~>b~~}` over `-->`, `~~x~~`, `~x~` is not stated (design 02 has it; the spec does not). | 1917–1936, 1899 |
| m16 | App. draft 1 | minor | The include row still says `{include}[file]` was rejected for `--8<--`, the reverse of §Includes; mark it "reversed in draft 3" as the DOI row does. | 1826 |
| m17 | §Image, Figure | minor | Video/audio in print: "its poster frame or first frame with the URL as a textual reference locator" is not an implementable sentence. | 1117–1119 |
| m18 | §Registries | minor | "`@` and `#[…]` resolve against named registries" — `#(…)` is the one that resolves against the counter registry. | 346 |
| m19 | Writing | minor | Non-normative prose to move to `design/`: the introduction (21–49), the underline essay (774–786), the comment rationale (686–695), the textual-reference manifesto (976–980), the includes argument (1457–1460), the roadmap (1665–1694), Appendix draft 2 (1698–1808). | listed |

Counts: 5 blocking, 15 major, 19 minor.

## 2. Blocking and major findings, with a proposed resolution

### B1 — The document disclaims its own normativity

Line 15 says normative wording "is aspirational until the conformance suite
exists"; the suite exists (`spec/conformance/`, review 02). Line 17 defines
*(proposed)* as "not implemented in TeXSmith yet", and the marker is on
`:::` (269), profiles (490), the front-matter layout (511), `Ref` (926),
`CounterItem` (1022), `Caption` (1067), subfigures (1126), `python image`
(1171), `thm` (1480) — most of which tmark implements. Line 580 says node
names are those of `texsmith.ir.nodes`, line 556 that validation is pydantic,
lines 106 and 69 that `tmark fmt` is "roadmap". Lines 46–49 say the document
uses the shipping spelling instead of its own canonical one. A reader cannot
tell which sentences are the language and which are a status report on a
Python code base that no longer defines it.

Resolution: replace the preamble with a two-sentence status ("This is the
normative definition of TMark 0.1. Conformance is `spec/conformance/`; where
a fixture and this text disagree, the text wins and the fixture is a bug.").
Delete every *(proposed)* / "shipping" / "(roadmap)" marker; where a
construct is genuinely not implemented anywhere, say "no implementation
yet" in the deprecation-style table of §Roadmap, not in the section. Point
node names at `tmark schema ir` (or the IR design doc), and drop "pydantic".
Rewrite 46–49 or make the spec source use its own canonical form.

### B2 — One lookup rule, contradicted three times

Lines 346–349: "`@a:b` whose head `a` is a declared counter prefix is a
label or counter reference; `@gls:term` is a glossary reference; any other
`@key` is a bibliography key." Lines 1016–1017 repeat it as a resolution
order. Then: `@stock` (914) resolves to a table because the *host* decided
the counter; `@boot-sequence` (617) resolves to a heading's implicit id;
`@fwrev:fw:x` (1626) resolves through a crossref alias, which is neither a
counter prefix nor the bibliography; and a span anchor `{#claim:one}`
(224–226) is presented as a working anchor while, under the rule as stated,
`@claim:one` is a bibliography key (C24; the CLI reports it unresolved).

Resolution: state the lookup once, in §Registries, as an ordered list over
*registries* rather than over spellings, and delete the copy at 1016:

1. labels — every `#id` on a host, every counter item, every implicit
   heading id, whatever the id's shape;
2. glossary (`gls:`), then DOI (`doi:`);
3. cross-document aliases (`alias:prefix:key`);
4. bibliography.

A key found in two registries is an error `ref-ambiguous`. With labels
first, C24 closes and every example in the spec resolves. If bibliography
must shadow labels for some reason, say so and make `{#claim:one}` on a
span a lint.

### B3 — One key grammar, stated once

Today: attribute id `[\w:.-]+` (365); role name `[A-Za-z][\w-]*` (378); bare
key `[A-Za-z][\w:.-]*[A-Za-z0-9]` (417, 426); counter key `[\w.-]+` after
`prefix:` (434); caption id `[\w:.-]+` (440); prose "`[A-Za-z0-9_:.-]`"
(938); prefix `[A-Za-z][A-Za-z0-9_-]*` (1513). Consequences the spec does not
notice: `@doi:10.1002/andp.19053221004` (1009) contains `/` and cannot match
417 (C3); a Zotero key `1RgTv` cannot be bare (C27; the CLI prints `\@1RgTv`);
`{#fw:a:b}` is a legal id but `#(fw:a:b)` is literal text (CLI), so a
two-colon crossref key can be referenced and never defined by a counter item.

Resolution: add one subsection "Identifiers" to §Lexical grammar with three
named productions and make every other recogniser cite them by name:
`prefix = [A-Za-z][\w-]*`; `key = [\w][\w.-]*` (a segment); `id = prefix
(":" key)+ | key`, plus the `doi:` exception (`[^\s\[\]()]+` ending on an
alphanumeric). State the bare-reference rule as "`@` followed by an `id`,
trailing `.,;:` excluded" and the digit-initial rule explicitly (bare
allowed, or bracketed required — either is fine, once). Then 938–939, 417,
426, 434 and 440 become references to the productions.

### B4 — Citation form must not depend on a space

Lines 937–941 make the brackets a lexical necessity ("required as soon as the
reference contains a space", "brackets are optional"). Lines 995–996 make the
same brackets a semantic switch (`@key` narrative, `@[key, locator]`
parenthetical). Under both sentences `@[ein05]` is undefined, and `@ein05`
in the example "Time is relative @ein05," is used parenthetically while
being defined as narrative. Pandoc's grammar, which the section claims to
adopt, keys the distinction on `[@k]` versus `@k`, i.e. on a spelling that
TMark has deliberately collapsed. Typst's model, which line 1001 invokes,
is: `@key` is the plain citation, narrative is a per-citation form
(`#cite(<k>, form: "prose")`).

With the planned change (bare `@key` = short citation; a front-matter
switch makes it narrative), the section stays coherent only if: (a) sentence
995–996 is replaced by "brackets carry no meaning; a citation's *form* is
short by default (`press.cite.form: short | prose`), and `@[-key]`
suppresses the author"; (b) a per-citation narrative spelling exists,
because a document-wide switch is not what either Pandoc or Typst offers —
propose an item flag mirroring `-`: `@[+ein05]` or `@[ein05 form=prose]`,
whichever fits the item grammar at 426; (c) labels are said to have one form
only. Without (b) the "one-`@`-for-all model" claim is false: Typst offers
both forms per citation.

### B5 — `#{…}` is in three states

"Withdrawn" (336–338), "no further guard is needed now that `#{…}` is gone"
(485), "Sugar: `#{fw:boot-loop}` (shipping, deprecated)" (1037), a
deprecation row with horizon `fmt` (1954), and "except `#{…}` … which were
never the recommended spelling" and therefore removed rather than deprecated
(1779–1781). C16 adds a guard (recognised only when the prefix is declared)
that the text does not have. Draft-2 item 1 (1707) says the standalone
define is `#[…]` for counter items too, contradicting §Two sigils.

Resolution: one state. Recommended: deprecated sugar, recognised only when
`prefix` is a declared counter (so `#{user.name}` stays literal), horizon
`fmt`, row 1954 kept; rewrite 336–338 and 485 to say "deprecated, guarded";
delete it from the 1779 exception list; fix 1707 to "`#[…]` for index
entries and `#(…)` for counter items".

### M1 — Sigils are sugar, and §Two sigils should say so

§Two sigils reads as if `#[term]` and `#(fw:key)` were the forms of the
language; §IndexEntry and §CounterItem make `{index}[…]` and
`{counter}(…)` canonical and the sigils sugar (C6, resolution "roles are
canonical", still not written). Add one sentence at 330: "The sigil forms
are sugar; the canonical spellings are the roles `{index}` and `{counter}`
(§Catalogue), which the printer emits." Then the table @tbl:sigils column
"Before brackets (node)" should read "sugar for".

### M2 — Hosts: list them all, and say where the braces go

Line 223 lists headings, images, links, fenced blocks, tables, caption
lines and the span. The spec then attaches `{#eq:x}` to display math (908,
1256), `{.epigraph}` to a block quote (648), `media=` to "a paragraph" and
"a callout" (243) and `{title=…}` to a container fence (266). Block quotes,
paragraphs and lists have no stated attribute position at all. Nor does the
text say whether `[x] {#id}` (space) or `![a](b) {#id}` is an attachment;
the lexical grammar says "on its line or at end of line", which covers
blocks only.

Resolution: one table of hosts with the position of the attribute list per
host (heading: end of line; image/link/span/code span: immediately after,
no whitespace; fenced block: on the info string, in braces; container: on
the opening fence; display math: after the closing `$$`, same line; table:
on the caption line, or a trailing `{…}` line — choose; block quote and
paragraph: not hosts, or a trailing attribute line — choose). Delete
"a paragraph" from 243 if paragraphs are not hosts.

### M3 — What is inside role brackets

"Brackets hold *content*, parsed as Markdown" (203–205) versus `{code
py}[print(1)]` and `{keys}[ctrl+s]`, whose content must be verbatim (the
CLI keeps `a*b*c _x_` literal inside `{code}`). The recogniser at 379
forbids an unescaped `[` or `]` in content, so `{aside}[see [x](u)]` cannot
be written; the example at 879–880 shows only emphasis. Line 443 admits the
escape but not whether the backslash is stripped before the Markdown pass
(if it is, `\[x\](u)` becomes a link; if not, literal brackets).

Resolution: in §Roles, add "Content is inline Markdown except for roles
declared *verbatim* (`code`, `keys`), where it is a string"; state that a
backslash-escaped bracket is removed by the role recogniser and the
remaining text is then parsed as inline Markdown, so `\[x\](u)` is a link;
mark this in each verbatim role's entry. Alternatively allow balanced
brackets in content (as parentheses are balanced in arguments), which
removes the escape question for the common case.

### M4 — Profiles: one definition

§Conformance (490–495): profiles `default` and `strict`; strict disables X1
and X2 and rejects the appendix sugar. §Roadmap (1673–1676): profiles
`canonical`, `strict`, `mkdocs`; strict "rewrites X-class constructs"
(which would include X4 and X5, i.e. the reference system). §Para (638):
strict turns off `paragraph.lead`. Feature table (1651): strict turns off
`compat.pymdownx`. Open question 2 asks about X3. Resolution: a §Profiles
subsection (in §Conformance) with a three-row table — profile, deviations
active, sugar accepted, features overridden — and every other mention
becomes a cross-reference. Pick `canonical` as the base name (the design and
the CLI use it).

### M5 — Front matter: declare the keys

Line 556 ("unknown keys fail at parse time") contradicts 505–509 (the root
is owned by MkDocs/Hugo/Jekyll) and C9. The list at 501–503 of "every key
TMark reads" omits `lang` (567), `id` (522) and `epigraph` (523). The text
relies on `press.aside`, `press.comments`, `press.details`, `press.numbered`,
`press.toc`, `press.code.engine`, `press.code.inline`, `press.callouts.*`,
`press.refs.textual.*`, `press.slots`, `press.template`, `press.base_level`,
`press.slugs` (open), `press.cite.form` (planned) — none with a type or a
default, and the layout at 515–548 shows some and not others. Resolution:
one table per group (`meta`, `press` form keys, `declare`, `sources`,
`features`) with key, type, default, and "read by: tmark | TeXSmith";
replace 556 with C9's rule (TMark validates the groups it reads and
preserves the rest).

### M6 — Diagnostics: a table and a scale

Names introduced inline: `ref-implicit-id`, `container-unknown`,
`container-unclosed`, `directive-foreign`, `feature-off`, `icon-web-only`,
`caption-no-host`, `container-orphan`, `ref-unresolved`; C-file names not
yet in the text: `role-unknown`, `include-inline`, `caption-kind-mismatch`,
`label-duplicate`, `prefix-unknown`, `prefix-host-mismatch`,
`attr-no-host`, `caption-id-off-convention` (all emitted by the CLI today).
Severity words used: warn, hint, lint hint, error, "hard warning" (1018),
"class D error" (1384), "warns loudly" (1515). Resolution: a §Diagnostics
appendix — name, severity (`error | warning | hint`), when it fires, which
section — and a one-line severity scale (error = check fails; warning =
fails under `--strict`; hint = informational). Every section then cites the
name only.

### M7 — Caption attachment cannot produce the canonical table

The canonical order at 1197–1198 is table, `table-config` fence, caption
line. The attachment rule at 1102–1106 says a caption attaches to "the block
before it when that block is a float (a table, a code block, …)". The
`table-config` fence is a code block, so the caption attaches to the fence
and the fence becomes a listing — or an implementer special-cases it, and
the text does not say. Also unspecified: whether `Figure:`/`Listing:` before
their float are accepted like `Table:` before (1086–1088 mentions `Table:`
only, while the attachment rule accepts any kind after any float);
kind/host mismatch (C18); and "a bare image stays inline" (1100) — bare
meaning "no caption and no anchor", or "inside a paragraph with text"?
Resolution: say that a `table-config` fence is part of its table (never a
float, never a host); say "Kind before the float is sugar for every kind";
add `caption-kind-mismatch`; define "bare" as "no caption line and no
anchor, whatever its paragraph".

### M8 — The recognisers disagree with the prose beside them

They are declared "the normative recognisers" (356). Value alternative
`\S+` (365, 378) swallows `}`; the prose two lines later (369–370) says a
bare value ends at `}` (C1/C2). The caption line (440) accepts only `{#id}`
while 232 makes `lang=`/`media=` universal and C23 says the full list. The
container fence's `\{[^}]*\}` (394) cannot hold a quoted `}` and disagrees
with 365. The info string (409) has no brace attribute list (C28).
Resolution: fix the four regexes to match the prose (`[^\s}]+`; reuse the
attribute-list production by name in 394, 409 and 440).

### M9 — A prefix that disagrees with its host

Line 917–919: "when a prefix is present it must agree with the host, and a
mismatch is linted". Line 919–922: a prefix is required "to number [a
heading or image] in a custom series instead of its own (`## Boot loop
{#fw:boot-loop}`)". The second sentence is a mismatch by the first
sentence's definition. The CLI lints (`prefix-host-mismatch`) and does not
renumber. Also undefined: which of `part chap sec app` agrees with a
heading (the level-to-prefix map is the template's). Resolution: "A
predeclared prefix must agree with the host (any heading prefix agrees with
any heading); a *user-declared* prefix on any host numbers it in that
series." Or drop the custom-series feature for headings and images.

### M10 — The inline table's Class column and the `Span` row

For `SmallCaps`, `Subscript` the class shown is the sugar's (X); for
`Underline` it is the role's (D) while the appendix gives its sugar E; the
canonical role spelling of every one of these is D. Say which the column
means (recommend: canonical spelling's class; sugar classes in the
appendix). `Quoted`: 762 makes `'x'` canonical; 1900 says single quotes are
never paired — delete `'x'`. `Span`: the printer has three spellings for
one node type depending on class and content (`[x]{attrs}`; icon shortcode
when the class is `icon` and `media=web` and the text is a shortcode;
critic markup when the class is `critic`). The one-canonical-spelling
principle survives if the row says so explicitly: "printed as the shortcode
/ as critic markup when …; otherwise `[x]{attrs}`".

### M11 — The CounterItem example does not check

Lines 1030–1031 define `fw:boot-loop` twice (item, then heading anchor), and
1028 uses `n:` undeclared. On the example alone `tmark check` reports
`prefix-unknown` twice, `label-duplicate`, then `ref-unresolved`. (The
heading is inside its fence; the suspicion that it escaped is unfounded.)
Resolution: use two keys (`#(fw:boot-loop)` … `## Watchdog {#fw:watchdog}`)
and either declare `n` in a front-matter snippet above the example or use
`fw` in the table too. Same fix for the `n:joy` row.

### M12 — P6 overclaims

"No sugar is accepted without a horizon" (95–97): the compatibility table
(1880–1905) has no horizon column; `\(…\)`, `\[…\]` (795, 1249) are "a
compatibility layer" with no row in the deprecation table; `#[…]`/`#(…)`
and `_x_` have none either. Resolution: reword P6 to "every sugar is listed
in the deprecation table or in the compatibility table, with a horizon of
`fmt` or `indefinite`"; add rows for `\(…\)`, `\[…\]`, the sigil sugar
(indefinite) and `$$x$$ {…}` on one line if it is accepted (C13).

### M13 — Mark processor-defined behaviour

Design 00 draws the purity line (Loader, TeXSmith). The spec states, as if
they were language rules: conversion of `.mmd`/`.drawio` to PDF
(1121–1123), execution of `python image` (1171–1173), DOI and Wikipedia
fetches (1010, 1529, 1602), publishing `doc.refs.json` (1614), `date:
commit` (521), `.bib` "on the command line" (1525), `--no-promote-title`
(592, 1976), first-heading promotion (591–593), and `format` as a "Python
format string" (1500) that a Rust core must reimplement. Resolution: one
paragraph in §IR: "Behaviours marked *processor* need a file, a process or
the network; the IR records the request (an `Image` with a `.mmd` source, a
`CodeBlock` with node `image`, a `sources.bibliography` entry) and a
processor fulfils it. tmark never performs them." Then tag each sentence
*(processor)*. Replace "Python format string" with the accepted subset
(`{n}`, `{n:0Nd}`, `{prefix}`, `{key}`, roman/alpha if wanted).

### M14 — Escaping: say what the printer writes

The round-trip section promises a fixed point; §Lexical grammar lists `\@`,
`\#` and brackets in role content (443). Not stated: how a `Str` that
happens to spell `{aside}[x]`, `#[x]`, `#(a:b)`, `:smile:`, `[=45% "x"]`,
`^^x^^`, `--8<--`, `[x]{#id}` or a paragraph beginning `Table:` next to a
table is printed so that it is still text after re-parsing (the CLI emits
`\{`, `\#`, `\@`, `\^`; `Table\:` works by CommonMark but is nowhere in the
text, and a prose paragraph "Table: the following…" after a pipe table is
silently captured — no diagnostic). Resolution: one list under §Round-trip:
"the printer backslash-escapes the first character of any run that a
recogniser of §Lexical grammar would otherwise match", plus the caption-kind
escape, and say all of them are ordinary CommonMark escapes (`@`, `#`, `{`,
`[`, `:`, `^`, `=` are ASCII punctuation).

### M15 — The catalogue is not yet the IR

§IR says the IR is the definition; §Catalogue names nodes but never their
fields, and a dozen nodes that exist (`Link`, `Image`, `Str`, `SoftBreak`,
`LineBreak`, `Abbr`, `Math`, `Note`, `Table`'s model, `Figure`, `Caption`,
`Ref`, `Cite`, `Include`) have no entry or no field list; `DefinitionList`,
lists, `BlockQuote`, `Note`, inline `Math` and acronyms (`\acrshort` only)
have no three-backend mapping. Resolution for 0.1: keep the prose entries
and make `tmark schema ir` the normative field list, cited once at 577;
add a compact "backend" line to the six entries that lack one, or write
"backend-defined" explicitly.

## 3. Sections in good shape

- §Round-trip and source spans (112–155): precise, three guarantees, says
  what is lost and what is not.
- §Four families, the attribute/role disjointness argument (174–218): the
  best-designed rule in the document; only the host list (M2) and verbatim
  roles (M3) are missing.
- §HorizontalRule (697–722) after C48: one node, two contracts, stated
  once, with the reason.
- §Comment (667–683, minus the rationale paragraph).
- §Foreign directive (724–740).
- §Header, implicit ids (605–622): the slug rule is complete and states
  what is stored (nothing).
- §Emoji and icon shortcodes, §TeX logos (825–863).
- §Image, Figure — the sub-figure numbering paragraph (1142–1153).
- §Tabs and §Div (1328–1399): closed registry, contract, unknown-name
  behaviour, `md_in_html` sugar, all stated.
- §Raw passthrough (1401–1430).
- §Includes — the snippet marker and `;` escape (1447–1457).
- §Glossary and acronyms (1560–1600): both spellings and the mixing rule.
- Appendix "Critic markup" (1915–1940).
- The deprecation table (1950–1980): every row has a replacement and a
  horizon.

## 4. Fitness for 0.1

Must be fixed in the text before it is the source of truth: B1–B5, M2, M5,
M6, M8, M11. They are all edits to existing sections; the two largest (a
key-grammar subsection and a front-matter key table) are each under a page.

Can follow in 0.1.x without changing any accepted document: M1, M3 (if
"verbatim roles" is stated), M4, M7, M9, M10, M12–M15, and all minors.

The rationale prose (m19) should move to `design/` in the same pass that
deletes the *(proposed)* markers, since both belong to the same
"this is a definition, not a diary" edit.

## 5. Verdict

Not publishable as the source of truth in its current text. The language
design is coherent — the four families, the two sigils, the IR-first
stance, the sugar/canonical split — and most catalogue entries are precise.
What fails is the *reference system as written*: it has two lookup rules,
seven key grammars, a citation form keyed on a space, and a marker in three
states; and the document itself declines to be normative in its first
paragraph. Fix those five things and the hosts, front-matter and
diagnostic tables, and draft 3 becomes draft 4, publishable.

## 6. Triage

Disposition of every finding, on branch `fix/spec-text` (draft 4 of
`spec/tmark.md`). Commits: `4cbedba` (normativity, markers, profiles,
processor), `3d66d60` (identifiers, lookup order, sigils, hosts, roles),
`331bc1e` (front-matter keys, diagnostics, captions, escapes, minors).

| Id | Disposition |
| -- | ----------- |
| B1 | fixed (`4cbedba`): normative preamble; every `(proposed)`, "shipping", "roadmap", "pydantic", "ships today" marker removed; node names point at `tmark schema ir`; §Tooling is a status table; 46–49 rewritten. |
| B2 | fixed (`3d66d60`): one ordered lookup over registries in §Registries (alias, `doi:`/`gls:`, labels, bibliography), `ref-ambiguous` stated; §Ref and §Cite refer to it; the copy at §Cite deleted. Labels are looked up for any unprefixed key and any declared prefix; a colon key with an undeclared head stays a bibliography key, as the resolver does (C24 stated, the span example is `{#claim-one}`). |
| B3 | fixed (`3d66d60`): §Identifiers defines `prefix`, `key`, `id`, `doi` once; every recogniser cites them; the digit-initial rule stated (bare needs a letter, bracketed accepts a digit). |
| B4 | fixed (C51, verified): §Cite is one rule (bare = default form, `@[…]` = item list read as written, `+`/`-` per item); the "brackets appear when there is a space" sentence removed; labels have one form (§Ref). |
| B5 | fixed (`3d66d60`): `#{prefix:key}` is deprecated, guarded sugar in §Two sigils, X5, §CounterItem, Appendix draft 2 and the deprecation table; draft-2 item 1 corrected. |
| M1 | fixed (`3d66d60`): sigils are sugar for the roles, stated in §Two sigils; the column reads "sugar for a role". |
| M2 | fixed (`3d66d60`): Table "The attribute hosts" with the position per host; links, code spans, paragraphs and lists are not hosts (the IR has no field for them); no whitespace before the list. |
| M3 | fixed (`3d66d60`): verbatim roles `code` and `keys`; balanced brackets nest; `\[` is a literal bracket, not re-read. |
| M4 | fixed (`4cbedba`): §Profiles, one table; `canonical` is the base name; `strict` = X1, X3 and critic off, no feature change. |
| M5 | fixed (`331bc1e`): Table "The metadata keys" and Table "The `press` groups" with type, default and reader; C9's rule replaces the pydantic sentence. |
| M6 | fixed (`331bc1e`): Appendix "Diagnostics" with the four-step scale; the body cites names only. `strict-x-construct` exists in the code and is emitted nowhere: omitted from the table, reported to the code side. |
| M7 | fixed (`331bc1e`): `table-config` transparent to attachment; a caption before its float is sugar for every kind; "bare" defined; `caption-kind-mismatch` defined as a hint (C18 → code to emit it). |
| M8 | fixed (`3d66d60`): `[^\s}]+` values, caption line takes the full list, the container fence and the info string reuse `(?&attrs)`. |
| M9 | fixed (`3d66d60`): predeclared prefix must agree (any heading prefix agrees with any heading), mismatch warned and numbered in the prefix's series; user-declared prefix numbers the host in that series. |
| M10 | fixed (`331bc1e`): class column is the canonical spelling's; `'x'` removed; the `Span` row states its three printed spellings; `Str`/`SoftBreak`/`LineBreak`, `Link`, `Math`, `Abbr` rows added. |
| M11 | fixed (`3d66d60`): `fw` declared in the example, `#(fw:joy)`, `{#fw:watchdog}`. |
| M12 | fixed (`331bc1e`): P6 reworded; rows for `\(…\)`/`\[…\]`, the sigils and one-line `$$` added (indefinite); the PyMdownX rows declared indefinite unless listed. |
| M13 | fixed (`4cbedba`): one paragraph in §IR; each sentence tagged `(processor)`; `format` is the `{n}`, `{n:0Nd}`, `{prefix}`, `{key}` subset the code implements. |
| M14 | fixed (`331bc1e`): the escape list under §Round-trip, `Kind\:` included; entities print as their character (C22 decided that way). |
| M15 | fixed (`4cbedba`, `331bc1e`): `tmark schema ir` is the normative field list, cited once; backend lines for block quotes, lists, definition lists, footnotes, inline links, math, acronyms; "backend-defined" stated for the rest. |
| m1 | fixed (`331bc1e`): default titles; `!!! type` accepts any word, `::: type` does not. |
| m2 | fixed (`4cbedba`): class C for a plain fence, E for its options. |
| m3 | fixed (`4cbedba`): footnotes are class C; the `note` row says no `@note:…` key exists. |
| m4 | fixed (`4cbedba`): three levels in both spellings, a fourth group is literal. |
| m5 | fixed (`3d66d60`): "one or more items"; `{}` is literal text. |
| m6 | fixed (`331bc1e`): the line between a form key and a feature drawn in §Feature registry. |
| m7 | fixed (`331bc1e`): entities next to the escapes. |
| m8 | fixed (`331bc1e`): `<strong class="lead">`. |
| m9 | fixed (`331bc1e`): wiki links are kept as typed and `compat-unsupported`; resolution is the site's `(processor)`. |
| m10 | fixed (`4cbedba`, `331bc1e`): `[](other.md)` needs an alias (C12); `[text](other.md)` is a plain link. |
| m11 | fixed in part (`4cbedba`: §Tabs says the contract is defined in §Div); the section order is deferred (C52). |
| m12 | fixed in part (`3d66d60`: stated as a hint, `position-word`); the narrower scope is a lint change, deferred (C53). |
| m13 | fixed (`3d66d60`): the reason for not shadowing a role name; the key part is compared case-insensitively. |
| m14 | fixed (`331bc1e`): `cols=`/`rows=` defaults; one anchor per figure container. |
| m15 | fixed (`331bc1e`): the brace group is recognised before the smart symbols and the same-character sugar. |
| m16 | fixed (`331bc1e`): "Reversed in draft 3" on the include row. |
| m17 | fixed (`331bc1e`): video in print renders the alt text and the URL, `poster=` names an image. |
| m18 | fixed (`3d66d60`): §Registries no longer says `#[…]` resolves. |
| m19 | deferred (C52): the rationale prose moves to `design/` in an editorial pass after 0.1. |
