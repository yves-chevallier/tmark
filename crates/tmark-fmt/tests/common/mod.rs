//! Fixture access shared by the printer's integration tests.
#![allow(dead_code)]
use std::{fs, path::Path};

/// The `## canonical` block of a fixture, if any.
pub fn canonical(text: &str) -> Option<String> {
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

/// Every conformance fixture with a canonical block: `(name, canonical)`,
/// in file order.
pub fn fixtures() -> Vec<(String, String)> {
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

/// A few lines around the first differing line of two texts.
pub fn first_difference(a: &str, b: &str) -> String {
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
