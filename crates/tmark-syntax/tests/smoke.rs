//! Smoke test: the extension's sample and the spec parse without panicking,
//! and a few constructs land in the expected nodes.
use tmark_ir::{Block, FileId, Inline};
use tmark_syntax::parse;

fn blocks(text: &str) -> Vec<Block> {
    parse(text, FileId::default()).document.blocks
}

#[test]
fn sample_parses() {
    let sample = include_str!("../../../editors/vscode/test/sample.md");
    let parsed = parse(sample, FileId::default());
    assert!(!parsed.document.blocks.is_empty());
    let json = serde_json::to_string_pretty(&parsed.document).unwrap();
    assert!(json.contains("\"type\": \"Aside\""));
    for d in &parsed.diagnostics {
        eprintln!("{}: {} @ {:?}", d.code.id(), d.message, d.span);
    }
}

#[test]
fn spec_parses() {
    let spec = include_str!("../../../spec/tmark.md");
    let parsed = parse(spec, FileId::default());
    assert!(parsed.document.blocks.len() > 50);
}

#[test]
fn roles_and_attrs() {
    let blocks = blocks("## Title {#sec:a .x k=v}\n\nSee @sec:a and {aside side=left}[x] here.\n");
    let Block::Header(h) = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert_eq!(h.attrs.id.as_deref(), Some("sec:a"));
    assert_eq!(h.attrs.classes, vec!["x"]);
    assert_eq!(tmark_syntax_test_helpers::plain(&h.content), "Title");
    let Block::Para(p) = &blocks[1] else {
        panic!("{blocks:?}")
    };
    assert!(matches!(&p.content[1], Inline::Ref(r) if r.items[0].key == "sec:a"));
    assert!(matches!(&p.content[3], Inline::Aside(a) if a.side == Some(tmark_ir::Side::Left)));
}

#[test]
fn containers_and_captions() {
    let blocks = blocks("::: warning {title=\"T\"}\nBody @xy.\n:::\n\n| a |\n| - |\n| 1 |\n\nTable: Cap. {#tbl:x}\n");
    let Block::Admonition(a) = &blocks[0] else {
        panic!("{blocks:?}")
    };
    assert_eq!(a.kind, "warning");
    assert_eq!(
        a.title
            .as_ref()
            .map(|t| tmark_syntax_test_helpers::plain(t))
            .as_deref(),
        Some("T")
    );
    assert!(matches!(&a.content[0], Block::Para(p) if matches!(&p.content[1], Inline::Ref(_))));
    assert!(matches!(&blocks[1], Block::Table(_)));
    let Block::Caption(c) = &blocks[2] else {
        panic!("{blocks:?}")
    };
    assert_eq!(c.attrs.id.as_deref(), Some("tbl:x"));
    assert_eq!(tmark_syntax_test_helpers::plain(&c.content), "Cap.");
}

mod tmark_syntax_test_helpers {
    use tmark_ir::Inline;
    pub fn plain(inlines: &[Inline]) -> String {
        inlines
            .iter()
            .map(|i| match i {
                Inline::Str(s) => s.text.clone(),
                Inline::SoftBreak(_) => " ".into(),
                _ => String::new(),
            })
            .collect()
    }
}

#[test]
fn compat_spellings() {
    let blocks = blocks(
        "Pandoc [see @ein05, p. 33; -@AI2027].\n\n\\[\nx^2\n\\]\n\n--8<-- \"snippets/file.md\"\n",
    );
    let Block::Para(p) = &blocks[0] else {
        panic!("{blocks:?}")
    };
    let Inline::Ref(r) = &p.content[1] else {
        panic!("{:?}", p.content)
    };
    assert!(r.bracketed);
    assert_eq!(r.items[0].prefix.as_deref(), Some("see"));
    assert_eq!(r.items[0].key, "ein05");
    assert!(r.items[1].suppress_author);
    let Block::MathBlock(m) = &blocks[1] else {
        panic!("{blocks:?}")
    };
    assert_eq!(m.text, "x^2");
    let Block::Include(i) = &blocks[2] else {
        panic!("{blocks:?}")
    };
    assert_eq!(i.path, "snippets/file.md");
}
