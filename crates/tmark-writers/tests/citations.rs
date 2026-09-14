//! Citation forms (spec §Cite, C51): a bare `@key` is the short citation
//! unless `citations.narrative` — the writer option, else the front
//! matter's feature, the precedence of `lang` — says narrative; `@[key]`
//! is always short, `+key` always narrative, `-key` the year alone. The
//! web has one author-year form and reads none of it.

use tmark_registry::{resolve, MemoryLoader, ResolveOptions};
use tmark_writers::{write, Backend, Body, CitationOptions, WriterOptions};

const BIB: &str = "press:\n  sources:\n    bibliography:\n      ein05: {type: article, author: \"Einstein, Albert\", year: 1905, title: Zur Elektrodynamik}\n";
const NARRATIVE: &str = "  features:\n    citations.narrative: true\n";
const BODY: &str = "Cite @ein05, @[ein05], @[+ein05], @[-ein05] and @[+ein05, p. 3; ein05].\n";

fn render(features: &str, option: Option<bool>, body: &str, backend: Backend) -> Body {
    let text = format!("---\n{BIB}{features}---\n\n{body}");
    let parsed = tmark_syntax::parse(&text, tmark_ir::FileId::default());
    let resolved = resolve(
        &parsed.document,
        &MemoryLoader::new(),
        &ResolveOptions::default(),
    );
    let opts = WriterOptions {
        citations: CitationOptions { narrative: option },
        ..WriterOptions::default()
    };
    write(&parsed.document, &resolved, backend, &opts)
}

const SHORT_LATEX: &str = "Cite \\cite{ein05}, \\cite{ein05}, \\textcite{ein05}, \\citeyear{ein05} and \\textcite[p. 3]{ein05}; \\cite{ein05}.";
const NARRATIVE_LATEX: &str = "Cite \\textcite{ein05}, \\cite{ein05}, \\textcite{ein05}, \\citeyear{ein05} and \\textcite[p. 3]{ein05}; \\cite{ein05}.";

#[test]
fn a_bare_key_is_the_short_citation_by_default() {
    let latex = render("", None, BODY, Backend::Latex);
    assert_eq!(latex.text.trim_end(), SHORT_LATEX);
    // `ts-bibliography` is the fallback for `\textcite`: named only when
    // one is written.
    assert!(latex.requires.fragments.contains("ts-bibliography"));
    let plain = render("", None, "Cite @ein05 and @[ein05].\n", Backend::Latex);
    assert_eq!(
        plain.text.trim_end(),
        "Cite \\cite{ein05} and \\cite{ein05}."
    );
    assert!(plain.requires.fragments.is_empty(), "{:?}", plain.requires);
    assert_eq!(plain.requires.citations, ["ein05"]);

    let typst = render("", None, BODY, Backend::Typst).text;
    assert!(
        typst.starts_with("Cite #cite(<ein05>), #cite(<ein05>), #cite(<ein05>, form: \"prose\"), #cite(<ein05>, form: \"year\")"),
        "{typst}"
    );
    assert!(
        typst.contains("#cite(<ein05>, supplement: [p. 3], form: \"prose\")#cite(<ein05>)"),
        "{typst}"
    );
}

#[test]
fn the_front_matter_switch_makes_the_bare_key_narrative() {
    let latex = render(NARRATIVE, None, BODY, Backend::Latex);
    assert_eq!(latex.text.trim_end(), NARRATIVE_LATEX);
    let typst = render(NARRATIVE, None, BODY, Backend::Typst).text;
    assert!(
        typst.starts_with(
            "Cite #cite(<ein05>, form: \"prose\"), #cite(<ein05>), #cite(<ein05>, form: \"prose\")"
        ),
        "{typst}"
    );
}

#[test]
fn the_writer_option_beats_the_front_matter() {
    let short = render(NARRATIVE, Some(false), BODY, Backend::Latex);
    assert_eq!(short.text.trim_end(), SHORT_LATEX);
    let narrative = render("", Some(true), BODY, Backend::Latex);
    assert_eq!(narrative.text.trim_end(), NARRATIVE_LATEX);
    let typst = render("", Some(true), "Cite @ein05.\n", Backend::Typst).text;
    assert_eq!(typst.trim_end(), "Cite #cite(<ein05>, form: \"prose\").");
}

#[test]
fn consecutive_plain_citations_merge_per_form() {
    // One `\cite{k1,k2}` per run of a form: a `+` item breaks the run.
    let latex = render(
        "",
        None,
        "See @[ein05; ein05; +ein05; ein05].\n",
        Backend::Latex,
    );
    assert_eq!(
        latex.text.trim_end(),
        "See \\cite{ein05,ein05}; \\textcite{ein05}; \\cite{ein05}."
    );
}

#[test]
fn the_web_has_one_form() {
    let short = render("", None, BODY, Backend::Html).text;
    let narrative = render(NARRATIVE, Some(true), BODY, Backend::Html).text;
    assert_eq!(short, narrative);
    assert!(
        short.contains(
            "Cite <span class=\"citation\">(<a href=\"#ref-ein05\">Einstein 1905</a>)</span>"
        ),
        "{short}"
    );
    assert!(
        short.contains("(<a href=\"#ref-ein05\">1905</a>)"),
        "{short}"
    );
}
