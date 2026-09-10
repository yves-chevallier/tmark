//! Conformance fixtures (`spec/conformance/*.md`): every `## input` block
//! parses to the `## ir` block, modulo ids and spans. Format in
//! `spec/conformance/README.md`.
use std::{fs, path::Path};

use serde_json::Value;
use tmark::{parse, FileId};

struct Fixture {
    name: String,
    inputs: Vec<String>,
    canonical: Option<String>,
    ir: Option<Value>,
    /// `code @ L:C-L:C` lines, expected on the first input.
    diagnostics: Vec<String>,
    /// Same, for the resolve and lint stages (`## resolution`); checked
    /// when the section exists.
    resolution: Option<Vec<String>>,
}

/// Fenced blocks under each `## section`, in order.
fn sections(text: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut fence: Option<(String, String, usize)> = None; // (lang, content, fence length)
    for line in text.lines() {
        if let Some((_, content, len)) = fence.as_mut() {
            let trimmed = line.trim_end();
            if trimmed.starts_with('`')
                && trimmed.chars().all(|c| c == '`')
                && trimmed.len() >= *len
            {
                let (lang, content, _) = fence.take().unwrap();
                if let Some(section) = out.last_mut() {
                    section.1.push((lang, content));
                }
            } else {
                content.push_str(line);
                content.push('\n');
            }
            continue;
        }
        if let Some(title) = line.strip_prefix("## ") {
            out.push((title.trim().to_string(), Vec::new()));
        } else if line.starts_with("```") {
            let len = line.chars().take_while(|c| *c == '`').count();
            let lang = line[len..].trim().to_string();
            fence = Some((lang, String::new(), len));
        }
    }
    out
}

fn lines(blocks: Vec<(String, String)>) -> Vec<String> {
    blocks
        .into_iter()
        .flat_map(|(_, c)| c.lines().map(str::to_string).collect::<Vec<_>>())
        .filter(|l| !l.trim().is_empty())
        .collect()
}

fn load(path: &Path) -> Fixture {
    let text = fs::read_to_string(path).unwrap();
    let mut fixture = Fixture {
        name: path.file_stem().unwrap().to_string_lossy().to_string(),
        inputs: Vec::new(),
        canonical: None,
        ir: None,
        diagnostics: Vec::new(),
        resolution: None,
    };
    for (title, blocks) in sections(&text) {
        match title.as_str() {
            "input" => fixture.inputs.extend(blocks.into_iter().map(|(_, c)| c)),
            "canonical" => fixture.canonical = blocks.into_iter().next().map(|(_, c)| c),
            "ir" => {
                fixture.ir = blocks
                    .into_iter()
                    .next()
                    .map(|(_, c)| serde_json::from_str(&c).expect("valid ir json"))
            }
            "diagnostics" => {
                fixture.diagnostics = lines(blocks);
            }
            "resolution" => {
                fixture.resolution = Some(lines(blocks));
            }
            _ => {}
        }
    }
    fixture
}

/// A fixture may spell absent values explicitly; `structural_json` never
/// does.
fn strip_absent(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, strip_absent(v)))
                .filter(|(_, v)| match v {
                    Value::Null => false,
                    Value::Array(a) => !a.is_empty(),
                    Value::Object(o) => !o.is_empty(),
                    Value::String(s) => !s.is_empty(),
                    Value::Bool(_) | Value::Number(_) => true,
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(strip_absent).collect()),
        other => other,
    }
}

fn fixtures() -> Vec<Fixture> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/conformance");
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "md") && p.file_name().unwrap() != "README.md"
        })
        .collect();
    paths.sort();
    paths.iter().map(|p| load(p)).collect()
}

#[test]
fn inputs_parse_to_the_ir() {
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let Some(expected) = &fixture.ir else {
            continue;
        };
        let expected = strip_absent(expected.clone());
        let mut cases = fixture.inputs.clone();
        cases.extend(fixture.canonical.clone());
        for (index, input) in cases.iter().enumerate() {
            let parsed = parse(input, FileId::default());
            let actual = tmark::ir::structural_json(&parsed.document);
            if actual != expected {
                failures.push(format!(
                    "{} input #{}:\n--- expected\n{}\n--- actual\n{}\n--- diagnostics\n{}",
                    fixture.name,
                    index + 1,
                    serde_json::to_string_pretty(&expected).unwrap(),
                    serde_json::to_string_pretty(&actual).unwrap(),
                    parsed
                        .diagnostics
                        .iter()
                        .map(|d| format!("{} {}", d.code.id(), d.message))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// `code @ L:C-L:C` for each diagnostic of the main file, sorted.
fn describe(input: &str, diagnostics: &[tmark::Diagnostic]) -> Vec<String> {
    let index = tmark::ir::LineIndex::new(input);
    let mut out: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.span.file == FileId::default())
        .map(|d| {
            let a = index.line_col(d.span.start);
            let b = index.line_col(d.span.end);
            format!(
                "{} @ {}:{}-{}:{}",
                d.code.id(),
                a.line + 1,
                a.col + 1,
                b.line + 1,
                b.col + 1
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn first_input_yields_the_resolution_diagnostics() {
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let (Some(input), Some(expected)) = (fixture.inputs.first(), &fixture.resolution) else {
            continue;
        };
        let parsed = parse(input, FileId::default());
        let resolved = tmark::resolve(
            &parsed.document,
            &tmark::MemoryLoader::new(),
            &Default::default(),
        );
        let mut diagnostics = resolved.diagnostics.clone();
        diagnostics.extend(tmark::lint(
            &parsed.document,
            &resolved,
            input,
            &Default::default(),
        ));
        let actual = describe(input, &diagnostics);
        let mut expected = expected.clone();
        expected.sort();
        if actual != expected {
            failures.push(format!(
                "{}: expected resolution {:?}, got {:?}",
                fixture.name, expected, actual
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn first_input_yields_the_diagnostics() {
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let Some(input) = fixture.inputs.first() else {
            continue;
        };
        let parsed = parse(input, FileId::default());
        let index = tmark::ir::LineIndex::new(input);
        let mut actual: Vec<String> = parsed
            .diagnostics
            .iter()
            .map(|d| {
                let a = index.line_col(d.span.start);
                let b = index.line_col(d.span.end);
                format!(
                    "{} @ {}:{}-{}:{}",
                    d.code.id(),
                    a.line + 1,
                    a.col + 1,
                    b.line + 1,
                    b.col + 1
                )
            })
            .collect();
        actual.sort();
        let mut expected = fixture.diagnostics.clone();
        expected.sort();
        if actual != expected {
            failures.push(format!(
                "{}: expected diagnostics {:?}, got {:?}",
                fixture.name, expected, actual
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
