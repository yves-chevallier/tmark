//! The printer's contract (design 04): the canonical block of every fixture
//! is a fixed point; formatting a real document and parsing it back gives the
//! same IR; formatting is idempotent.
use std::{fs, path::Path};

use tmark_fmt::{format, Profile};
use tmark_ir::FileId;
use tmark_syntax::parse;

/// The `## canonical` block of a fixture, if any.
fn canonical(text: &str) -> Option<String> {
    let mut lines = text.lines();
    lines.find(|l| l.trim() == "## canonical")?;
    let mut fence: Option<usize> = None;
    let mut body = String::new();
    for line in lines {
        match fence {
            None => {
                if line.starts_with("```") {
                    fence = Some(line.chars().take_while(|c| *c == '`').count());
                } else if line.starts_with("## ") {
                    return None;
                }
            }
            Some(len) => {
                let trimmed = line.trim_end();
                if trimmed.starts_with('`')
                    && trimmed.chars().all(|c| c == '`')
                    && trimmed.len() >= len
                {
                    return Some(body);
                }
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    None
}

fn fixtures() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/conformance");
    let mut out = Vec::new();
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    for path in paths {
        if path.file_name().unwrap() == "README.md" {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        if let Some(c) = canonical(&text) {
            out.push((path.file_stem().unwrap().to_string_lossy().to_string(), c));
        }
    }
    out
}

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

/// A few lines around the first differing line of two texts.
fn first_difference(a: &str, b: &str) -> String {
    let a: Vec<&str> = a.lines().collect();
    let b: Vec<&str> = b.lines().collect();
    let at = a
        .iter()
        .zip(&b)
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let from = at.saturating_sub(4);
    let show = |v: &[&str]| v[from..(at + 6).min(v.len())].join("\n");
    format!(
        "line {}\n<<< first\n{}\n>>> second\n{}",
        at + 1,
        show(&a),
        show(&b)
    )
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

/// A deprecated citation hugging the word before it prints with a space,
/// because the X4 guard keeps `@` from firing after a word character or
/// `.` (spec §Lexical grammar); the printed text parses back to the same
/// references.
#[test]
fn hugging_citations_get_a_space() {
    let text = "Slow to talk,^[ein05,AI2027] he said.[^ein05] Then^[10.1007/x] more.\n";
    let doc = parse(text, FileId::default()).document;
    let printed = format(&doc, Profile::Canonical);
    assert_eq!(
        printed,
        "Slow to talk,@[ein05; AI2027] he said. @ein05 Then @doi:10.1007/x more.\n"
    );
    // The inserted space changes the `Str` text, so the sugar is not a
    // strict round trip; the printed text is a fixed point that parses back
    // to the same references.
    let failures = roundtrip("citations", &printed);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
