//! One `insta` snapshot per conformance fixture and per backend (design
//! 10-testing.md §4; `spec/conformance/README.md`: "Backend blocks are
//! `insta` snapshots"). The snapshot holds the body text followed by the
//! `Requires` as JSON, so the set of fragments and packages a construct
//! needs is part of its conformance (design 07 §Tests).
//!
//! Review a change with `cargo insta review` (or `INSTA_UPDATE=always
//! cargo test -p tmark-writers --test fixtures` and read the diff).

use std::fs;
use std::path::Path;

use tmark_registry::{resolve, MemoryLoader, ResolveOptions};
use tmark_writers::{write, Backend, Media, WriterOptions};

/// Backends with a writer; extend as they land.
const BACKENDS: &[Backend] = &[Backend::Html];

/// The fenced block under `## canonical`, else the first under `## input`.
fn source(text: &str) -> Option<String> {
    let mut section = "";
    let mut fence: Option<(usize, String)> = None;
    let mut input = None;
    let mut canonical = None;
    for line in text.lines() {
        if let Some((len, content)) = fence.as_mut() {
            let trimmed = line.trim_end();
            if trimmed.starts_with('`')
                && trimmed.chars().all(|c| c == '`')
                && trimmed.len() >= *len
            {
                let (_, content) = fence.take().unwrap();
                match section {
                    "input" if input.is_none() => input = Some(content),
                    "canonical" if canonical.is_none() => canonical = Some(content),
                    _ => {}
                }
            } else {
                content.push_str(line);
                content.push('\n');
            }
            continue;
        }
        if let Some(title) = line.strip_prefix("## ") {
            section = match title.trim() {
                "input" => "input",
                "canonical" => "canonical",
                _ => "",
            };
        } else if line.starts_with("```") && !section.is_empty() {
            let len = line.chars().take_while(|c| *c == '`').count();
            fence = Some((len, String::new()));
        }
    }
    canonical.or(input)
}

#[test]
fn fixtures() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/conformance");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "md") && p.file_name().unwrap() != "README.md"
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    let loader = MemoryLoader::new();
    for path in paths {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let text = fs::read_to_string(&path).unwrap();
        let Some(source) = source(&text) else {
            continue;
        };
        let parsed = tmark_syntax::parse(&source, tmark_ir::FileId::default());
        let resolved = resolve(&parsed.document, &loader, &ResolveOptions::default());
        for backend in BACKENDS {
            let opts = WriterOptions {
                media: match backend {
                    Backend::Html => Media::Web,
                    _ => Media::Print,
                },
                lang: parsed.document.front_matter.keys.lang.clone(),
                ..WriterOptions::default()
            };
            let body = write(&parsed.document, &resolved, *backend, &opts);
            let requires = serde_json::to_string_pretty(&body.requires).unwrap();
            let snapshot = format!("{}\n---requires---\n{requires}\n", body.text);
            insta::assert_snapshot!(format!("{name}@{}", backend.as_str()), snapshot);
        }
    }
}
