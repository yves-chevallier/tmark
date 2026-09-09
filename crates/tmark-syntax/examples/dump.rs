//! Dump the IR of a file as JSON, diagnostics on stderr:
//! `cargo run -p tmark-syntax --example dump -- path/to/file.md`
use std::{env, fs};

use tmark_ir::{FileId, LineIndex};
use tmark_syntax::parse;

fn main() {
    let path = env::args().nth(1).expect("usage: dump <file>");
    let text = fs::read_to_string(&path).expect("readable file");
    let parsed = parse(&text, FileId::default());
    let index = LineIndex::new(&text);
    for d in &parsed.diagnostics {
        let at = index.line_col(d.span.start);
        eprintln!(
            "{path}:{}:{}: {} {}: {}",
            at.line + 1,
            at.col + 1,
            d.code.default_severity_name(),
            d.code.id(),
            d.message
        );
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&parsed.document).expect("serialises")
    );
}

trait SeverityName {
    fn default_severity_name(self) -> &'static str;
}

impl SeverityName for tmark_ir::Code {
    fn default_severity_name(self) -> &'static str {
        match self.default_severity() {
            tmark_ir::Severity::Error => "error",
            tmark_ir::Severity::Warning => "warning",
            tmark_ir::Severity::Info => "info",
            tmark_ir::Severity::Hint => "hint",
        }
    }
}
