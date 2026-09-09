//! `tmark` command-line interface. Design: `design/09-bindings.md`.
//!
//! Milestone 1: `parse`, `fmt` and `schema`; `lint`, `check` and `write`
//! arrive with their crates.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tmark::ir::{LineIndex, Severity};
use tmark::{format, parse, FileId, Profile};

#[derive(Parser)]
#[command(name = "tmark", version, about = "The TMark language toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a file and print its IR as JSON (diagnostics on stderr).
    Parse {
        /// The file to parse; `-` for standard input.
        file: PathBuf,
        /// Compact JSON on one line.
        #[arg(long)]
        compact: bool,
    },
    /// Rewrite files in their normal form (to stdout by default).
    Fmt {
        /// The files to format; `-` for standard input.
        files: Vec<PathBuf>,
        /// `canonical` (default), `strict` or `mkdocs`.
        #[arg(long, default_value = "canonical")]
        profile: String,
        /// Exit 1 when a file is not already in normal form; print nothing.
        #[arg(long)]
        check: bool,
        /// Rewrite the files in place instead of printing.
        #[arg(long)]
        write: bool,
    },
    /// Print a JSON schema: `ir` or `frontmatter`.
    Schema { name: String },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Parse { file, compact } => cmd_parse(&file, compact),
        Command::Fmt {
            files,
            profile,
            check,
            write,
        } => cmd_fmt(&files, &profile, check, write),
        Command::Schema { name } => cmd_schema(&name),
    }
}

fn read(file: &PathBuf) -> io::Result<String> {
    if file.as_os_str() == "-" {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        Ok(text)
    } else {
        std::fs::read_to_string(file)
    }
}

fn cmd_parse(file: &PathBuf, compact: bool) -> ExitCode {
    let text = match read(file) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("tmark: {}: {error}", file.display());
            return ExitCode::from(2);
        }
    };
    let parsed = parse(&text, FileId::default());
    let index = LineIndex::new(&text);
    let mut errors = false;
    let name = file.display().to_string();
    for d in &parsed.diagnostics {
        let at = index.line_col(d.span.start);
        let severity = match d.code.default_severity() {
            Severity::Error => {
                errors = true;
                "error"
            }
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };
        eprintln!(
            "{name}:{}:{}: {severity} {}: {}",
            at.line + 1,
            at.col + 1,
            d.code.id(),
            d.message
        );
    }
    let json = if compact {
        serde_json::to_string(&parsed.document)
    } else {
        serde_json::to_string_pretty(&parsed.document)
    }
    .expect("the IR serialises");
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{json}");
    if errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_fmt(files: &[PathBuf], profile: &str, check: bool, write: bool) -> ExitCode {
    let profile = match profile {
        "canonical" => Profile::Canonical,
        "strict" => Profile::Strict,
        "mkdocs" => Profile::Mkdocs,
        other => {
            eprintln!("tmark: unknown profile `{other}` (canonical, strict, mkdocs)");
            return ExitCode::from(2);
        }
    };
    let mut findings = false;
    let mut failed = false;
    for file in files {
        let text = match read(file) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("tmark: {}: {error}", file.display());
                failed = true;
                continue;
            }
        };
        let parsed = if profile == Profile::Strict {
            tmark::parse_strict(&text, FileId::default())
        } else {
            parse(&text, FileId::default())
        };
        let formatted = format(&parsed.document, profile);
        if check {
            if formatted != text {
                eprintln!("{}: not in normal form", file.display());
                findings = true;
            }
        } else if write && file.as_os_str() != "-" {
            if formatted != text {
                if let Err(error) = std::fs::write(file, &formatted) {
                    eprintln!("tmark: {}: {error}", file.display());
                    failed = true;
                }
            }
        } else {
            let mut out = io::stdout().lock();
            let _ = out.write_all(formatted.as_bytes());
        }
    }
    if failed {
        ExitCode::from(2)
    } else if findings {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_schema(name: &str) -> ExitCode {
    match tmark::schema(name) {
        Some(schema) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schema).expect("schema serialises")
            );
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("tmark: unknown schema `{name}` (ir, frontmatter)");
            ExitCode::from(2)
        }
    }
}
