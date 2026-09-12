//! Localised label words (design 06 §Site-wide resolution,
//! `tmark_ir::registry::PREFIX_NAMES`): the `ref` template of a
//! predeclared series renders the word of `WriterOptions.lang`, else the
//! resolution's (itself `ResolveOptions.lang` else the front matter's
//! `lang`), else the front matter's, else English — in references, in
//! HTML caption labels and in the web lowering. A `declare.counters`
//! `name` is never localised away. LaTeX's own `\caption` numbering stays
//! babel's.

use tmark_registry::{resolve, MemoryLoader, Numbering, ResolveOptions};
use tmark_writers::{lower_web, write, Backend, Media, WebOptions, WriterOptions};

const BODY: &str = "| A |\n| - |\n| 1 |\n\nTable: Stock. {#tbl:x}\n\n![Trace](t.png)\n\nFigure: Trace. {#fig:y}\n\n$$\nE = mc^2\n$$ {#eq:m}\n\nSee @tbl:x, @fig:y and @eq:m.\n";

fn render(
    front: &str,
    resolve_lang: Option<&str>,
    writer_lang: Option<&str>,
    backend: Backend,
) -> String {
    let text = format!("{front}{BODY}");
    let parsed = tmark_syntax::parse(&text, tmark_ir::FileId::default());
    let options = ResolveOptions {
        numbering: Numbering::All,
        lang: resolve_lang.map(str::to_string),
        ..ResolveOptions::default()
    };
    let resolved = resolve(&parsed.document, &MemoryLoader::new(), &options);
    let opts = WriterOptions {
        media: match backend {
            Backend::Html => Media::Web,
            _ => Media::Print,
        },
        lang: writer_lang.map(str::to_string),
        ..WriterOptions::default()
    };
    write(&parsed.document, &resolved, backend, &opts).text
}

/// The web lowering, whose `@eq:m` becomes `[Équation 1](#eq:m)`.
fn lower(front: &str, web_lang: Option<&str>) -> String {
    let text = format!("{front}{BODY}");
    let parsed = tmark_syntax::parse(&text, tmark_ir::FileId::default());
    let loader = MemoryLoader::new();
    let resolved = resolve(
        &parsed.document,
        &loader,
        &ResolveOptions {
            numbering: Numbering::All,
            ..ResolveOptions::default()
        },
    );
    let web = WebOptions {
        lang: web_lang.map(str::to_string),
        ..WebOptions::default()
    };
    lower_web(&text, &parsed.document, &resolved, &loader, &web).text
}

#[test]
fn the_web_lowering_localises_its_link_text() {
    let fr = lower("---\nlang: fr\n---\n\n", None);
    assert!(fr.contains("[Équation 1](#eq:m)"), "{fr}");
    assert!(fr.contains("[Table 1](#tbl:x)"), "{fr}");
    let en = lower("", None);
    assert!(en.contains("[Equation 1](#eq:m)"), "{en}");
    // The web option beats the front matter.
    let de = lower("---\nlang: fr\n---\n\n", Some("de"));
    assert!(de.contains("[Tabelle 1](#tbl:x)"), "{de}");
    assert!(de.contains("[Gleichung 1](#eq:m)"), "{de}");
}

#[test]
fn french_front_matter() {
    // `PREFIX_NAMES` follows babel, whose French `\tablename` and
    // `\figurename` are "Table" and "Figure": those two words coincide
    // with English, which is what keeps a reference agreeing with the
    // caption babel prints. `eq` is where French parts company.
    let latex = render("---\nlang: fr\n---\n\n", None, None, Backend::Latex);
    assert!(latex.contains("\\hyperref[tbl:x]{Table~1}"), "{latex}");
    assert!(latex.contains("\\hyperref[fig:y]{Figure~1}"), "{latex}");
    assert!(latex.contains("Équation~1"), "{latex}");
    // LaTeX's own caption numbering is babel's: no word in `\caption`.
    assert!(
        latex.contains("\\caption{Stock.}\n\\label{tbl:x}"),
        "{latex}"
    );
    let typst = render("---\nlang: fr\n---\n\n", None, None, Backend::Typst);
    assert!(typst.contains("Équation 1"), "{typst}");
    let html = render("---\nlang: fr\n---\n\n", None, None, Backend::Html);
    assert!(html.contains(">Table 1</a>"), "{html}");
    assert!(html.contains(">Équation 1</a>"), "{html}");
    assert!(
        html.contains("<span class=\"caption-label\">Figure 1</span>"),
        "{html}"
    );
}

#[test]
fn english_by_default() {
    let latex = render("", None, None, Backend::Latex);
    assert!(latex.contains("\\hyperref[tbl:x]{Table~1}"), "{latex}");
    assert!(latex.contains("\\hyperref[fig:y]{Figure~1}"), "{latex}");
    assert!(latex.contains("Equation~1"), "{latex}");
    let html = render("", None, None, Backend::Html);
    assert!(html.contains(">Table 1</a>"), "{html}");
    assert!(html.contains(">Equation 1</a>"), "{html}");
}

#[test]
fn writer_option_wins_and_resolution_language_is_used() {
    // The writer option beats the front matter.
    let latex = render("---\nlang: fr\n---\n\n", None, Some("de"), Backend::Latex);
    assert!(latex.contains("\\hyperref[tbl:x]{Tabelle~1}"), "{latex}");
    assert!(latex.contains("\\hyperref[fig:y]{Abbildung~1}"), "{latex}");
    // A resolution language alone (TeXSmith passes it to `resolve`) is
    // honoured when the writer got none; the region subtag is ignored.
    let latex = render("", Some("de-CH"), None, Backend::Latex);
    assert!(latex.contains("\\hyperref[tbl:x]{Tabelle~1}"), "{latex}");
    // An unknown language falls back to English.
    let latex = render("---\nlang: xx\n---\n\n", None, None, Backend::Latex);
    assert!(latex.contains("\\hyperref[tbl:x]{Table~1}"), "{latex}");
}

#[test]
fn a_declared_name_is_not_localised_away() {
    // `declare.counters` names a predeclared series: the word is the
    // author's, in every language, while its untouched neighbours are
    // still localised.
    let front =
        "---\nlang: fr\npress:\n  declare:\n    counters:\n      tbl: {name: Tableau}\n---\n\n";
    let latex = render(front, None, None, Backend::Latex);
    assert!(latex.contains("\\hyperref[tbl:x]{Tableau~1}"), "{latex}");
    assert!(latex.contains("Équation~1"), "{latex}");
    // Even when the writer is asked for another language.
    let latex = render(front, None, Some("de"), Backend::Latex);
    assert!(latex.contains("\\hyperref[tbl:x]{Tableau~1}"), "{latex}");
    assert!(latex.contains("\\hyperref[fig:y]{Abbildung~1}"), "{latex}");
}
