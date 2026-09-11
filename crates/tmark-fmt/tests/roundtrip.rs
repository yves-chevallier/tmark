//! The printer's contract (design 04): the canonical block of every fixture
//! is a fixed point; formatting a real document and parsing it back gives the
//! same IR; formatting is idempotent.
mod common;

use common::{first_difference, fixtures};
use tmark_fmt::{format, Profile};
use tmark_ir::FileId;
use tmark_syntax::parse;

#[test]
fn canonical_blocks_are_fixed_points() {
    let mut failures = Vec::new();
    for (name, canonical) in fixtures() {
        let printed = format(
            &parse(&canonical, FileId::default()).document,
            Profile::Canonical,
        );
        if printed != canonical {
            failures.push(format!(
                "{name}:\n--- canonical\n{canonical}--- printed\n{printed}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn roundtrip(name: &str, text: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let first = parse(text, FileId::default()).document;
    let printed = format(&first, Profile::Canonical);
    let second = parse(&printed, FileId::default()).document;
    // Compare modulo ids, spans and the fields that record which spelling
    // was used (the printer normalises them), as the conformance runner does.
    let a = serde_json::to_string_pretty(&tmark_ir::structural_json(&first)).unwrap();
    let b = serde_json::to_string_pretty(&tmark_ir::structural_json(&second)).unwrap();
    if a != b {
        let diff = first_difference(&a, &b);
        failures.push(format!("{name}: parse(format(doc)) != doc\n{diff}"));
    }
    let again = format(&second, Profile::Canonical);
    if again != printed {
        let diff = first_difference(&printed, &again);
        failures.push(format!("{name}: format is not idempotent\n{diff}"));
    }
    failures
}

#[test]
fn sample_round_trips() {
    let sample = include_str!("../../../editors/vscode/test/sample.md");
    let failures = roundtrip("sample.md", sample);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn spec_round_trips() {
    let spec = include_str!("../../../spec/tmark.md");
    let failures = roundtrip("tmark.md", spec);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
