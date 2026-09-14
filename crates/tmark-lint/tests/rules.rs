use tmark_ir::{Diagnostic, FileId};
use tmark_lint::{lint, Config};
use tmark_registry::{resolve, MemoryLoader, ResolveOptions};
use tmark_syntax::parse;

fn codes(text: &str) -> Vec<String> {
    let doc = parse(text, FileId::default()).document;
    let resolved = resolve(&doc, &MemoryLoader::new(), &ResolveOptions::default());
    let mut v: Vec<String> = lint(&doc, &resolved, text, &Config::default())
        .iter()
        .map(|d| d.code.id().to_string())
        .collect();
    v.sort();
    v
}

fn diagnostics(text: &str) -> Vec<Diagnostic> {
    let doc = parse(text, FileId::default()).document;
    let resolved = resolve(&doc, &MemoryLoader::new(), &ResolveOptions::default());
    lint(&doc, &resolved, text, &Config::default())
}

/// The one diagnostic of `code` fired on `text` (a rule fires once here).
fn only(text: &str, code: &str) -> Diagnostic {
    let mut diags: Vec<_> = diagnostics(text)
        .into_iter()
        .filter(|d| d.code.id() == code)
        .collect();
    assert_eq!(diags.len(), 1, "{code} on {text:?}: {diags:?}");
    diags.remove(0)
}

/// The source slice a diagnostic's span covers.
fn at<'a>(text: &'a str, d: &Diagnostic) -> &'a str {
    &text[d.span.start as usize..d.span.end as usize]
}

#[test]
fn catalogue() {
    assert_eq!(
        codes("As Figure 3 shows, see the table below and the notes above.\n"),
        vec!["hardcoded-number", "position-word", "position-word"]
    );
    assert_eq!(
        codes("Table 3. is a sentence end; Figure 3b is fine; tables 33 too; belowground.\n"),
        vec!["hardcoded-number"],
        "only a bare number after the word"
    );
    assert_eq!(codes("# A\n\n### C\n\n## B\n"), vec!["heading-skip"]);
    assert_eq!(
        codes("**Note.**\n\n{lead}[Explicit.] Not reported.\n"),
        vec!["lead-promotion"]
    );
    assert_eq!(
        codes("| a |\n| - |\n| 1 |\n\nTable: T. {#fig:t}\n"),
        vec!["caption-id-off-convention"]
    );
    assert_eq!(
        codes("| a |\n| - |\n| 1 |\n\nTable: T. {#tbl:t}\n"),
        Vec::<String>::new()
    );
}

#[test]
fn config_switches_rules_off_and_relevels() {
    let text = "See the table below.\n";
    let doc = parse(text, FileId::default()).document;
    let resolved = resolve(&doc, &MemoryLoader::new(), &ResolveOptions::default());
    let mut config = Config::default();
    config.set(tmark_ir::Code::PositionWord, "error").unwrap();
    let diags = lint(&doc, &resolved, text, &config);
    assert_eq!(diags[0].severity, tmark_ir::Severity::Error);
    config.set(tmark_ir::Code::PositionWord, "off").unwrap();
    assert!(lint(&doc, &resolved, text, &config).is_empty());
    assert!(config.set(tmark_ir::Code::PositionWord, "loud").is_err());
}

// One test per rule the catalogue test above does not already exercise
// (spec/conformance fixtures: directive-foreign.md, inline-insert-off.md,
// inline-icon.md, diag-table-placement.md, diag-table-width.md,
// diag-table-width-sum.md).

#[test]
fn directive_foreign_fires_on_a_dotted_directive_not_on_a_known_container() {
    // spec §Foreign directive: a dotted `::: a.b` is kept verbatim as a
    // `RawBlock` for another processor (mkdocstrings).
    let text = "::: texsmith.core.config\n    options:\n      members: true\n";
    let hit = only(text, "directive-foreign");
    assert_eq!(at(text, &hit), text.trim_end());

    // Near miss: an ordinary, known admonition name is a structured `Div`,
    // never a foreign `RawBlock`.
    assert!(!codes("::: note\nBody.\n:::\n").contains(&"directive-foreign".to_string()));
}

#[test]
fn feature_off_fires_on_caret_insert_not_on_a_blank_run() {
    // spec §Inline text: `^^x^^` without `inline.insert` (off by default)
    // stays literal text.
    let text = "Now ^^inserted^^ text.\n";
    let hit = only(text, "feature-off");
    assert_eq!(at(text, &hit), "^^inserted^^");

    // Near miss: an empty run between the markers is never a construct.
    assert!(!codes("Blank ^^ ^^ markers.\n").contains(&"feature-off".to_string()));
}

#[test]
fn icon_web_only_fires_on_a_material_shortcode_not_on_an_emoji() {
    // spec §Emoji and icon shortcodes: a Material icon shortcode lowers to
    // `Span{.icon media=web}`, which print drops.
    let text = "Click :material-cog: Settings.\n";
    let hit = only(text, "icon-web-only");
    assert_eq!(at(text, &hit), ":material-cog:");

    // Near miss: a plain emoji shortcode expands to the character itself,
    // never a `Span`.
    assert!(!codes("Click :smile: Settings.\n").contains(&"icon-web-only".to_string()));
}

#[test]
fn table_placement_fires_outside_the_float_letters() {
    // design 05 `table-placement`: `placement` must be made of `hHtbpT!`.
    let text = "```yaml table\ntable: {placement: xyz}\ncolumns: [A, B]\nrows: [[1, 2]]\n```\n";
    let hit = only(text, "table-placement");
    assert_eq!(at(text, &hit), text.trim_end());

    // Near miss: a valid combination of float letters.
    let ok = "```yaml table\ntable: {placement: htbp}\ncolumns: [A, B]\nrows: [[1, 2]]\n```\n";
    assert!(!codes(ok).contains(&"table-placement".to_string()));
}

#[test]
fn table_width_fires_outside_0_to_100_percent() {
    // spec §Table: a percentage width outside (0, 100].
    let text = "```yaml table\ntable: {width: 150%}\ncolumns: [A, B]\nrows: [[1, 2]]\n```\n";
    let hit = only(text, "table-width");
    assert_eq!(at(text, &hit), text.trim_end());

    // Near miss: a width inside the range.
    let ok = "```yaml table\ntable: {width: 50%}\ncolumns: [A, B]\nrows: [[1, 2]]\n```\n";
    assert!(!codes(ok).contains(&"table-width".to_string()));
}

#[test]
fn table_width_sum_fires_over_100_percent() {
    // design 05 `table-width-sum`: column percentages add up to more than
    // the table.
    let text =
        "```yaml table\ncolumns: [{name: A, width: 60%}, {name: B, width: 50%}]\nrows: [[1, 2]]\n```\n";
    let hit = only(text, "table-width-sum");
    assert_eq!(at(text, &hit), text.trim_end());

    // Near miss: columns that add up to 100% or less.
    let ok = "```yaml table\ncolumns: [{name: A, width: 40%}, {name: B, width: 50%}]\nrows: [[1, 2]]\n```\n";
    assert!(!codes(ok).contains(&"table-width-sum".to_string()));
}
