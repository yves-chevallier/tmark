//! The parser is total: any input yields a document whose spans are within
//! bounds and on character boundaries, and never panics (design/10-testing.md).
use proptest::prelude::*;
use tmark_ir::{walk, FileId, NodeRef};
use tmark_syntax::parse;

fn check(text: &str) {
    let parsed = parse(text, FileId::default());
    let len = text.len() as u32;
    walk(&parsed.document, &mut |node: NodeRef| {
        let span = node.span();
        assert!(span.start <= span.end, "span {span:?} inverted in {text:?}");
        assert!(span.end <= len, "span {span:?} out of bounds in {text:?}");
        assert!(
            text.is_char_boundary(span.start as usize) && text.is_char_boundary(span.end as usize),
            "span {span:?} splits a character in {text:?}"
        );
    });
    for d in &parsed.diagnostics {
        assert!(
            d.span.end <= len,
            "diagnostic span out of bounds in {text:?}"
        );
    }
}

/// Bytes drawn from the alphabet that triggers the TMark constructs.
fn tmark_soup() -> impl Strategy<Value = String> {
    let atom = prop_oneof![
        Just("{".to_string()),
        Just("}".to_string()),
        Just("[".to_string()),
        Just("]".to_string()),
        Just("(".to_string()),
        Just(")".to_string()),
        Just("@".to_string()),
        Just("#".to_string()),
        Just(":::".to_string()),
        Just("!!!".to_string()),
        Just("???".to_string()),
        Just(":   ".to_string()),
        Just("```".to_string()),
        Just("$$".to_string()),
        Just("\\(".to_string()),
        Just("\\[".to_string()),
        Just("==".to_string()),
        Just("^".to_string()),
        Just("~".to_string()),
        Just("++".to_string()),
        Just("__".to_string()),
        Just("**".to_string()),
        Just("\n".to_string()),
        Just("\n\n".to_string()),
        Just(" ".to_string()),
        Just("    ".to_string()),
        Just("aside".to_string()),
        Just("index".to_string()),
        Just("raw latex".to_string()),
        Just("#id".to_string()),
        Just("k=v".to_string()),
        Just("sec:intro".to_string()),
        Just("Table: ".to_string()),
        Just("é".to_string()),
        Just("😀".to_string()),
        Just("x".to_string()),
        "[a-z ]{1,6}",
    ];
    prop::collection::vec(atom, 0..40).prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn never_panics_on_tmark_soup(text in tmark_soup()) {
        check(&text);
    }

    #[test]
    fn never_panics_on_any_string(text in "\\PC{0,80}") {
        check(&text);
    }
}

#[test]
fn every_line_of_the_spec_parses_alone() {
    let spec = include_str!("../../../spec/tmark.md");
    for line in spec.lines() {
        check(line);
    }
    // And every window of three lines, which exercises the block passes.
    let lines: Vec<&str> = spec.lines().collect();
    for window in lines.windows(3) {
        check(&window.join("\n"));
    }
}
