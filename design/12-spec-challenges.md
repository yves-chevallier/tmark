# 12 — Challenges to the spec

Points where implementing draft 3 exposed an ambiguity, an inconsistency or a
gap. Each entry names the section, the problem, the resolution taken by the
design (so that code and fixtures can proceed), and who closes it: **spec**
(the wording must change) or **code** (the design decided; the spec is fine).
Resolving an entry means editing `spec/tmark.md` and its fixture, then
deleting the entry from this file.

| # | Section | Problem | Resolution taken | Closes |
| - | ------- | ------- | ---------------- | ------ |
| C1 | §Lexical grammar, attribute list | The value alternative `\S+` can swallow the closing `}` (`{width=60%}` matches, but `{a=b}c` and `{k=v}` at end of line are fine only by luck; `{k=a}b}` is ambiguous). | Values are `"[^"]*"` or `[^\s}]+`. | spec |
| C2 | §Lexical grammar, role head | Same `\S+` flaw for key values in role heads; positional `[^\s=}]+` is right. | Same fix. | spec |
| C3 | §Cite, `@doi:` | The bare-reference key grammar `[A-Za-z][\w:.-]*[A-Za-z0-9]` cannot contain `/`, yet `@doi:10.1002/andp.19053221004` is the canonical example. | A `doi:` key accepts `[^\s\[\]()]*` ending on a non-punctuation character. Trailing `.`/`,`/`;`/`:` stay out. | spec |
| C4 | §Roles | "an unknown name is literal text" versus the closed registry: nothing says whether `{qty}[…]` should raise a diagnostic. Silent literal text hides typos (`{asid}[…]`). | Literal text (spec) plus lint hint `role-unknown` when the head is followed by `[` or `(`. | code |
| C5 | §IndexEntry | `#[**term**]` as sugar for `main=true` conflicts with "brackets hold content parsed as Markdown": a bold term that is not a main entry becomes unrepresentable in sugar. | Sugar kept; the canonical form `{index main=true}[term]` disambiguates; the printer never emits the bold sugar. Documented as an accepted lossy sugar. | spec (say it is lossy) |
| C6 | §Two sigils vs §IndexEntry/§CounterItem | The sigil section presents `#[…]` and `#(…)` as *the* forms; the catalogue says the roles `{index}` and `{counter}` are canonical and the sigils sugar. The printer must pick one. | Roles are canonical (matches the catalogue and P6's "one spelling per mechanism"; sigils stay the finger-friendly sugar). Revisit if authors find printed roles noisy. | spec (state it once, in §Two sigils) |
| C8 | §Attributes (zero-width nodes) | "whitespace on both sides collapses to a single space, and disappears before punctuation" is a rendering rule but is stated in the syntax section; the IR must decide whether the `Space` nodes exist. | Spaces stay in the IR (round-trip); writers apply the collapse. | code |
| C9 | §Front matter | "unknown keys fail at parse time" conflicts with the `press` namespace being shared with site generators that own other keys, and with TeXSmith owning `template`, `callouts`, … that TMark does not know. | TMark validates only the keys it reads (`declare`, `sources`, `features`, and the metadata keys) and preserves the rest; TeXSmith validates its own. Unknown keys *inside* a TMark-owned group are errors. | spec |
| C10 | §Include | "A block include is the include role alone on its line" — inline `{include}(…)` in a paragraph is not defined (error? inline splice?). | Inline includes are a diagnostic `include-inline` and render as literal text. | spec |
| C11 | §Data directives | `mermaid code` restores the listing, but what does `mermaid raw` mean? `raw` requires a backend name, and `mermaid` is not one. | `raw` is valid only when the language is a backend name (`latex`, `typst`, `html`); otherwise `fence-unknown-node-word`. | spec |
| C12 | §Ref | `[](other.md)` "section number of another document's main heading" needs an inventory; nothing says which alias. | Resolved only when `sources.crossrefs` has an entry whose `source` matches the path; else `ref-unresolved`. | spec |
| C13 | §Math | "No space directly after the opening delimiter" for `$…$` is the PyMdownX rule; `$$` on one line with attributes (`$$x$$ {#eq:a}`) is not covered. | Allowed; the attribute list attaches to the display math like on its own line. | spec |
| C14 | Appendix PyMdownX | Critic markup "fires even inside code spans" is a PyMdownX behaviour; TMark says nothing fires in code. | TMark never fires in code; critic markup is milestone 5 and follows TMark's rule. | spec |
| C15 | §Conformance, class E table | `pymdownx.betterem`, `smartsymbols`, `emoji`, `magiclink` are listed as extensions in the standard set, but the catalogue never assigns an IR node to emoji or smart symbols beyond `Str`. Fine, but magic links (bare URLs) are class C and produce `Link`; say so in §Inline. | `Link` with `target = Url`, autolink flag for the printer. | spec |
| C16 | §Two sigils | `#{…}` withdrawn but TeXSmith today ships `#{prefix:key}` and Ruby-style `#{user.name}` in prose must stay literal; the deprecated form's guard is not stated. | Deprecated `#{prefix:key}` is recognised only when the prefix is declared; otherwise literal. | spec |
| C17 | File extension | The spec assumes `.md`. Product discussions mention `.tm`. P3 (degradation on GitHub) argues for keeping `.md` primary. | `.md` primary; `.tm`/`.tmd`/`.tmark` accepted as explicit markers. ADR 0006. | spec (add a line) |

Open questions the spec already lists (Appendix "Open questions") are not
repeated here; their answers, when taken, go in the same appendix.

Closed: C7 (attachment rule written into §Caption, including the `::: figure` case).
