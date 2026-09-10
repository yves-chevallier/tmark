//! Dump the IR of a file as JSON, diagnostics on stderr:
//! `cargo run -p tmark-syntax --example dump -- [--structural] path/to/file.md`
//!
//! `--structural` prints `tmark_ir::structural_json`: no ids, spans or
//! sugar fields, which is what conformance fixtures store.
use std::{env, fs};

use tmark_ir::{structural_json, FileId, LineIndex};
use tmark_syntax::parse;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let structural = args.iter().any(|a| a == "--structural");
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .expect("usage: dump [--structural] <file>");
    let text = fs::read_to_string(path).expect("readable file");
    let parsed = parse(&text, FileId::default());
    let index = LineIndex::new(&text);
    for d in &parsed.diagnostics {
        let at = index.line_col(d.span.start);
        eprintln!(
            "{path}:{}:{}: {} {}: {}",
            at.line + 1,
            at.col + 1,
            d.code.default_severity().as_str(),
            d.code.id(),
            d.message
        );
    }
    let json = if structural {
        serde_json::to_string_pretty(&structural_json(&parsed.document))
    } else {
        serde_json::to_string_pretty(&parsed.document)
    };
    println!("{}", json.expect("serialises"));
}
