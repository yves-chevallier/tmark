//! Print the fixes `tmark::check` attaches to a file's diagnostics:
//! `cargo run -p tmark --example fixes -- FILE`
use std::{env, fs};

fn main() {
    let path = env::args().nth(1).expect("usage: fixes <file>");
    let text = fs::read_to_string(&path).expect("readable file");
    let (_, diagnostics) = tmark::check(
        &text,
        tmark::FileId::default(),
        tmark::Profile::Canonical,
        &tmark::FsLoader,
        &tmark::ResolveOptions {
            path: path.into(),
            ..Default::default()
        },
        &tmark::LintConfig::default(),
    );
    for d in diagnostics {
        if let Some(fix) = d.fix {
            let (s, e) = (fix.span.start as usize, fix.span.end as usize);
            println!("{:?} -> {:?}", &text[s..e], fix.replacement);
        }
    }
}
