//! The MkDocs profile (design 04 §Profiles): a snapshot per fixture, and the
//! D0 gate of the TeXSmith migration: `tmark fmt --profile mkdocs` on a
//! canonical file gives text that re-parses to the same IR (modulo the
//! `deprecated` diagnostics the shipping spellings raise) and that the
//! profile prints again unchanged.
mod common;

use common::{first_difference, fixtures};
use tmark_fmt::{format, print_node_with, Profile};
use tmark_ir::{structural_json, Block, FileId, Inline, NodeRef};
use tmark_syntax::parse;

/// Fixtures whose shipping spelling the parser does not read back yet:
/// the `[^key]` / `^[k1,k2]` citations (decision X7), `[](gls:term)`,
/// `{index}[…]{b}`, `/// latex` and `/// caption` (challenge C20). The
/// parser side lands in the `fixes` wave; the profile prints the spellings
/// now so that the gate goes green on merge. A fixture listed here must
/// still fail: remove it from the list when it passes.
/// Fixtures the parser cannot yet read back under the Mkdocs spelling.
const PENDING: &[&str] = &[];

fn structural(text: &str) -> String {
    let doc = parse(text, FileId::default()).document;
    serde_json::to_string_pretty(&structural_json(&doc)).unwrap()
}

fn mkdocs(text: &str) -> String {
    format(&parse(text, FileId::default()).document, Profile::Mkdocs)
}

#[test]
fn snapshots_over_the_fixture_corpus() {
    for (name, canonical) in fixtures() {
        insta::assert_snapshot!(format!("mkdocs__{name}"), mkdocs(&canonical));
    }
}

/// `parse(format(doc, Mkdocs)) == doc` and `format` is idempotent under
/// the profile, over every fixture.
#[test]
fn mkdocs_text_reparses_to_the_same_ir() {
    let mut failures = Vec::new();
    let mut unexpected_passes = Vec::new();
    for (name, canonical) in fixtures() {
        let printed = mkdocs(&canonical);
        let mut problems = Vec::new();
        let (a, b) = (structural(&canonical), structural(&printed));
        if a != b {
            problems.push(format!("IR differs\n{}", first_difference(&a, &b)));
        }
        let again = mkdocs(&printed);
        if again != printed {
            problems.push(format!(
                "not idempotent\n{}",
                first_difference(&printed, &again)
            ));
        }
        let pending = PENDING.contains(&name.as_str());
        match (problems.is_empty(), pending) {
            (true, false) | (false, true) => {}
            (true, true) => unexpected_passes.push(name),
            (false, false) => failures.push(format!(
                "{name}:\n--- mkdocs\n{printed}--- {}",
                problems.join("\n--- ")
            )),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    assert!(
        unexpected_passes.is_empty(),
        "now passing, remove from PENDING: {}",
        unexpected_passes.join(", ")
    );
}

#[test]
fn spellings_and_fallbacks() {
    let text = "\
---
press:
  declare:
    counters:
      fw: {name: Finding, format: \"FW-{n:02d}\"}
---

# Intro {#intro}

Some {mark}[a b], {del}[x], H{sub}[2]O, mc{sup}[2], {keys}[ctrl+alt+s], {code py}[print(1)], {sc}[caps].

See @intro, @sec:x, @fw:one, @ein05, @[ein05; ko20], @[see ein05, p. 3], @gls:term, @doi:10.1/x.

Define {counter}(fw:one) and {counter}(zz:two); {index registry=phys main=true}[a][b] and {index}[c].

Aside {aside side=left}[note] and {aside}[plain]; raw {raw latex}(\\clearpage) and {raw latex}(a[b]) and {raw foo}(x).

::: note {title=\"Say \\\"hi\\\"\"}
Kept canonical: the title has a quote.
:::

::: tip {#tip:x title=T}
Kept canonical: an id.
:::

::: warning {.wide title=\"A title\" collapsed=true}
Folded, with a class.

- a list
:::

{include}(chapters/a.md)

{include base=chapters}(chapters/b.md)

```latex raw
\\clearpage
```

```typst raw
#pagebreak()
```

![Boot](boot.png){#fig:boot}

Figure: The boot. {#fig:boot}

| A | B |
| - | - |
| 1 | 2 |

Table: Stock. {#tbl:stock}

Text @ein05[^1] and a footnote.

[^1]: Note.
";
    let doc = parse(text, FileId::default()).document;
    let printed = format(&doc, Profile::Mkdocs);
    insta::assert_snapshot!("mkdocs__spellings_and_fallbacks", printed);
    // A node printed on its own knows no labels: a bare key is a citation.
    let mut refs = Vec::new();
    tmark_ir::walk(&doc, &mut |n: NodeRef| {
        if let NodeRef::Inline(Inline::Ref(_)) = n {
            refs.push(print_node_with(n, Profile::Mkdocs));
        }
    });
    assert_eq!(refs[0], "[^intro]");
    assert_eq!(refs[1], "@sec:x");
    assert_eq!(refs[3], "[^ein05]");
    let admonition = doc
        .blocks
        .iter()
        .find(|b| matches!(b, Block::Admonition(a) if a.kind == "warning"))
        .unwrap();
    assert_eq!(
        print_node_with(NodeRef::Block(admonition), Profile::Mkdocs),
        "??? warning wide \"A title\"\n    Folded, with a class.\n\n    - a list"
    );
}
