//! Rough timing of the parser on a file repeated to about 1 MB:
//! `cargo run --release -p tmark-syntax --example bench -- spec/tmark.md`
use std::{env, fs, time::Instant};

use tmark_ir::FileId;
use tmark_syntax::parse;

fn main() {
    let path = env::args().nth(1).expect("usage: bench <file>");
    let unit = fs::read_to_string(&path).expect("readable file");
    let mut text = String::new();
    while text.len() < 1_000_000 {
        text.push_str(&unit);
        text.push('\n');
    }
    let start = Instant::now();
    let tree = tmark_markdown::to_mdast(&text, &tmark_markdown::ParseOptions::gfm());
    let gfm = start.elapsed();
    drop(tree);
    println!("tokenizer + mdast, gfm options: {gfm:?}");
    let start = Instant::now();
    let tree = tmark_markdown::to_mdast(&text, &tmark_markdown::ParseOptions::tmark());
    let tokenize = start.elapsed();
    drop(tree);
    let start = Instant::now();
    let parsed = parse(&text, FileId::default());
    let elapsed = start.elapsed();
    println!("tokenizer + mdast alone: {tokenize:?}");
    println!(
        "{} bytes, {} blocks, {} diagnostics: {:?}",
        text.len(),
        parsed.document.blocks.len(),
        parsed.diagnostics.len(),
        elapsed
    );
}
