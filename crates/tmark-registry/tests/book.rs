//! Site-wide resolution (design 06 §Site-wide resolution): numbering every
//! series across the documents of a book, and references between them.
//! The conformance fixtures cannot express options, so this lives here.
use std::collections::BTreeMap;
use std::path::PathBuf;

use tmark_ir::{Code, FileId};
use tmark_registry::{
    resolve, BookLabel, Host, MemoryLoader, Numbering, Resolution, ResolveOptions, Resolved,
};
use tmark_syntax::parse;

const DECLARE: &str = "---\npress:\n  declare:\n    counters:\n      fw: {name: Finding, format: \"FW-{n:02d}\"}\n---\n\n";

fn run(text: &str, path: &str, options: ResolveOptions) -> Resolved {
    let doc = parse(text, FileId::default()).document;
    let options = ResolveOptions {
        path: PathBuf::from(path),
        ..options
    };
    resolve(&doc, &MemoryLoader::new(), &options)
}

fn page_b(numbering: Numbering, start: BTreeMap<String, u32>) -> Resolved {
    let text = format!(
        "{DECLARE}## Boot {{#sec:boot}}\n\n#(fw:a) One.\n\n![x](a.png)\n\nFigure: Crash. {{#fig:crash}}\n\n| a |\n|---|\n| 1 |\n\nTable: Stock. {{#tbl:stock}}\n\n::: theorem {{#thm:t}}\nT.\n:::\n"
    );
    run(
        &text,
        "docs/b.md",
        ResolveOptions {
            numbering,
            start,
            ..Default::default()
        },
    )
}

#[test]
fn backend_numbering_leaves_the_backend_series_alone() {
    let b = page_b(Numbering::Backend, BTreeMap::new());
    assert_eq!(b.diagnostics, []);
    assert_eq!(b.next_start(), BTreeMap::from([("fw".to_string(), 2)]));
    assert_eq!(b.counters.get("fig").unwrap().label("crash"), None);
    let book = b.book_labels("b.md");
    let fig = book.iter().find(|l| l.key == "fig:crash").unwrap();
    assert_eq!(fig.number, None);
    assert_eq!(fig.title.as_deref(), Some("Crash."));
}

#[test]
fn all_numbers_every_series_continuously_through_start() {
    // Page A ends with two figures, a table and a finding already counted.
    let a = run(
        &format!("{DECLARE}![a](a.png)\n\nFigure: A. {{#fig:a}}\n\n![b](b.png)\n\nFigure: B. {{#fig:b}}\n\n| a |\n|---|\n| 1 |\n\nTable: T. {{#tbl:a}}\n\n#(fw:z) Z.\n\n## Intro {{#sec:intro}}\n"),
        "docs/a.md",
        ResolveOptions {
            numbering: Numbering::All,
            ..Default::default()
        },
    );
    assert_eq!(a.diagnostics, []);
    let next = a.next_start();
    assert_eq!(next.get("fig"), Some(&3));
    assert_eq!(next.get("tbl"), Some(&2));
    assert_eq!(next.get("fw"), Some(&2));
    assert_eq!(next.get("sec"), Some(&2));
    assert_eq!(
        next.get("lst"),
        Some(&1),
        "an empty series still starts at 1"
    );
    assert!(!next.contains_key("gls") && !next.contains_key("doi"));

    let b = page_b(Numbering::All, next);
    assert_eq!(b.diagnostics, []);
    let fig = b.counters.get("fig").unwrap();
    assert!(fig.tmark_numbered);
    assert_eq!(fig.label("crash").as_deref(), Some("3"));
    assert_eq!(fig.reference_text("crash").as_deref(), Some("Figure 3"));
    assert_eq!(
        b.counters.get("tbl").unwrap().label("stock").as_deref(),
        Some("2")
    );
    assert_eq!(
        b.counters.get("fw").unwrap().label("a").as_deref(),
        Some("FW-02")
    );
    assert_eq!(
        b.counters
            .get("thm")
            .unwrap()
            .reference_text("t")
            .as_deref(),
        Some("Theorem 1")
    );
    assert_eq!(
        b.counters
            .get("sec")
            .unwrap()
            .reference_text("boot")
            .as_deref(),
        Some("Section 2")
    );
    assert_eq!(b.next_start().get("fig"), Some(&4));
    let view = b.view();
    assert_eq!(view.numbering, Numbering::All);
    let crash = view
        .labels
        .iter()
        .find(|l| l.label.id == "fig:crash")
        .unwrap();
    assert_eq!(crash.formatted.as_deref(), Some("3"));
    assert_eq!(crash.label.number, Some(3));
}

#[test]
fn lang_localises_the_predeclared_label_words() {
    let de = page_b(Numbering::All, BTreeMap::new());
    assert_eq!(
        de.counters.get("fig").unwrap().name.as_deref(),
        Some("Figure")
    );
    let text = format!("{DECLARE}![x](a.png)\n\nFigure: C. {{#fig:c}}\n");
    let r = run(
        &text,
        "docs/c.md",
        ResolveOptions {
            numbering: Numbering::All,
            lang: Some("de-CH".to_string()),
            ..Default::default()
        },
    );
    let fig = r.counters.get("fig").unwrap();
    assert_eq!(fig.name.as_deref(), Some("Abbildung"));
    assert_eq!(fig.reference_text("c").as_deref(), Some("Abbildung 1"));
    assert_eq!(
        r.counters.get("fw").unwrap().name.as_deref(),
        Some("Finding")
    );
    assert_eq!(r.lang.as_deref(), Some("de-CH"));
    // The front matter's `lang` is the default.
    let r = run(
        "---\nlang: fr\n---\n\n![x](a.png)\n\nFigure: C. {#fig:c}\n",
        "docs/c.md",
        ResolveOptions::default(),
    );
    assert_eq!(
        r.counters.get("tbl").unwrap().name.as_deref(),
        Some("Table")
    );
    assert_eq!(
        r.counters.get("eq").unwrap().name.as_deref(),
        Some("Équation")
    );
    assert_eq!(r.view().lang.as_deref(), Some("fr"));
}

#[test]
fn a_ref_to_a_sibling_page_resolves_with_its_location_and_number() {
    let b = page_b(Numbering::All, BTreeMap::from([("fig".to_string(), 3)]));
    let book = b.book_labels("b.md");
    let crash = book.iter().find(|l| l.key == "fig:crash").unwrap();
    assert_eq!(
        crash,
        &BookLabel {
            key: "fig:crash".to_string(),
            prefix: Some("fig".to_string()),
            number: Some("3".to_string()),
            kind: Host::Figure,
            title: Some("Crash.".to_string()),
            location: "b.md#fig:crash".to_string(),
        }
    );
    assert_eq!(b.view().book[0].location, "docs/b.md#sec:boot");

    let a = run(
        &format!("{DECLARE}See @fig:crash, @Fw:a, @sec:boot, @thm:t and @fig:nope.\n"),
        "docs/a.md",
        ResolveOptions {
            numbering: Numbering::All,
            book,
            ..Default::default()
        },
    );
    let kinds: Vec<_> = a.refs.iter().map(|r| r.resolution.clone()).collect();
    assert_eq!(
        kinds[0],
        Resolution::Sibling {
            label: "3".to_string(),
            location: "b.md#fig:crash".to_string(),
        }
    );
    assert_eq!(
        kinds[1],
        Resolution::Sibling {
            label: "FW-01".to_string(),
            location: "b.md#fw:a".to_string(),
        },
        "sibling keys match case-insensitively"
    );
    assert!(
        matches!(&kinds[2], Resolution::Sibling { label, .. } if label == "1"),
        "sections are numbered under All"
    );
    assert!(matches!(&kinds[3], Resolution::Sibling { label, .. } if label == "1"));
    assert_eq!(kinds[4], Resolution::Unresolved);
    let codes: Vec<_> = a.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [Code::RefUnresolved]);
    let json = serde_json::to_value(a.view()).unwrap();
    assert_eq!(json["refs"][0]["resolution"]["kind"], "sibling");
    assert_eq!(json["refs"][0]["resolution"]["location"], "b.md#fig:crash");
}

#[test]
fn a_sibling_without_a_number_shows_its_title_then_its_key() {
    let b = page_b(Numbering::Backend, BTreeMap::new());
    let a = run(
        "See @sec:boot and @fig:crash.\n",
        "docs/a.md",
        ResolveOptions {
            book: b.book_labels("b.md"),
            ..Default::default()
        },
    );
    assert!(matches!(&a.refs[0].resolution, Resolution::Sibling { label, .. } if label == "Boot"));
    assert!(
        matches!(&a.refs[1].resolution, Resolution::Sibling { label, .. } if label == "Crash.")
    );
    let anchor = vec![BookLabel {
        key: "claim".to_string(),
        prefix: None,
        number: None,
        kind: Host::Anchor,
        title: None,
        location: "c.md#claim".to_string(),
    }];
    let a = run(
        "See @claim.\n",
        "docs/a.md",
        ResolveOptions {
            book: anchor,
            ..Default::default()
        },
    );
    assert!(matches!(&a.refs[0].resolution, Resolution::Sibling { label, .. } if label == "claim"));
}

#[test]
fn a_local_key_shadows_a_sibling_and_a_citation_makes_it_ambiguous() {
    let b = page_b(Numbering::All, BTreeMap::new());
    let book = b.book_labels("b.md");
    let a = run(
        &format!("{DECLARE}## Boot here {{#sec:boot}}\n\nSee @sec:boot and @fig:crash.\n"),
        "docs/a.md",
        ResolveOptions {
            book: book.clone(),
            ..Default::default()
        },
    );
    assert_eq!(a.diagnostics, []);
    assert!(
        matches!(&a.refs[0].resolution, Resolution::Label { .. }),
        "the local definition wins"
    );
    assert!(matches!(&a.refs[1].resolution, Resolution::Sibling { .. }));

    let text = "---\npress:\n  sources:\n    bibliography:\n      stock: {type: misc, title: Stock}\n---\n\nSee @stock.\n";
    let sibling = vec![BookLabel {
        key: "stock".to_string(),
        prefix: None,
        number: None,
        kind: Host::Anchor,
        title: None,
        location: "b.md#stock".to_string(),
    }];
    let a = run(
        text,
        "docs/a.md",
        ResolveOptions {
            book: sibling,
            ..Default::default()
        },
    );
    assert_eq!(a.refs[0].resolution, Resolution::Ambiguous);
    let codes: Vec<_> = a.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [Code::RefAmbiguous]);
    assert_eq!(a.diagnostics[0].span, a.refs[0].span);
}
