use tmark_ir::FileId;
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
        codes("**Note.** Promoted.\n\n{lead}[Explicit.] Not reported.\n"),
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
