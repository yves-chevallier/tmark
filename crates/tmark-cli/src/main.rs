//! `tmark` command-line interface. Design: `design/09-bindings.md`.
//!
//! `parse`, `fmt`, `check`, `lint` and `schema`; `write` arrives with the
//! writers (milestone 4). `lint --fix` is a milestone-3 item (it needs the
//! fixes the LSP also applies).

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tmark::ir::{Code, LineIndex, Severity};
use tmark::{check, format, parse, Config, FileId, FsLoader, LintConfig, Profile, ResolveOptions};

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
        /// `canonical`, `strict` or `mkdocs`; default from `tmark.toml`,
        /// else `canonical`.
        #[arg(long)]
        profile: Option<String>,
        /// Exit 1 when a file is not already in normal form; print nothing.
        #[arg(long)]
        check: bool,
        /// Rewrite the files in place instead of printing.
        #[arg(long)]
        write: bool,
    },
    /// Parse, resolve and lint files; exit 1 on errors (on warnings too
    /// with `--strict`).
    Check {
        /// Files to check; `.bib` files feed the bibliography.
        files: Vec<PathBuf>,
        /// Treat warnings as failures.
        #[arg(long)]
        strict: bool,
        /// Lint levels, `code=off|hint|info|warning|error`, repeatable.
        #[arg(long = "level", value_name = "CODE=LEVEL")]
        levels: Vec<String>,
    },
    /// Alias of `check` (fixes arrive with milestone 3).
    Lint {
        files: Vec<PathBuf>,
        #[arg(long)]
        strict: bool,
        #[arg(long = "level", value_name = "CODE=LEVEL")]
        levels: Vec<String>,
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
        } => cmd_fmt(&files, profile.as_deref(), check, write),
        Command::Check {
            files,
            strict,
            levels,
        }
        | Command::Lint {
            files,
            strict,
            levels,
        } => cmd_check(&files, strict, &levels),
        Command::Schema { name } => cmd_schema(&name),
    }
}

/// The `tmark.toml` above `file`, or the defaults; a file that does not
/// parse is a usage error.
fn config_for(file: &std::path::Path) -> Result<Config, ExitCode> {
    match Config::discover(file) {
        None => Ok(Config::default()),
        Some(Ok(config)) => Ok(config),
        Some(Err(error)) => {
            eprintln!("tmark: {error}");
            Err(ExitCode::from(2))
        }
    }
}

/// `--level` overrides on top of the file's lint configuration.
fn apply_levels(config: &mut LintConfig, levels: &[String]) -> Result<(), ExitCode> {
    for level in levels {
        let Some((code, value)) = level.split_once('=') else {
            eprintln!("tmark: --level takes CODE=LEVEL, got `{level}`");
            return Err(ExitCode::from(2));
        };
        let Some(code) = Code::from_id(code) else {
            eprintln!("tmark: unknown diagnostic code `{code}`");
            return Err(ExitCode::from(2));
        };
        if let Err(error) = config.set(code, value) {
            eprintln!("tmark: {error}");
            return Err(ExitCode::from(2));
        }
    }
    Ok(())
}

fn cmd_check(files: &[PathBuf], strict: bool, levels: &[String]) -> ExitCode {
    let bibliography: Vec<PathBuf> = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "bib"))
        .cloned()
        .collect();
    let mut worst = Severity::Hint;
    let mut failed = false;
    for file in files
        .iter()
        .filter(|f| !f.extension().is_some_and(|e| e == "bib"))
    {
        let text = match read(file) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("tmark: {}: {error}", file.display());
                failed = true;
                continue;
            }
        };
        let workspace = match config_for(file) {
            Ok(config) => config,
            Err(code) => return code,
        };
        let mut config = workspace.lint.clone();
        if let Err(code) = apply_levels(&mut config, levels) {
            return code;
        }
        let mut options: ResolveOptions = workspace.resolve_options(file);
        options.bibliography.extend(bibliography.iter().map(|b| {
            // `.bib` paths are given relative to the working directory;
            // the loader resolves them from the document's directory.
            let base = file.parent().unwrap_or(std::path::Path::new(""));
            pathdiff(b, base)
        }));
        let (_, diagnostics) = check(&text, FileId::default(), &FsLoader, &options, &config);
        let index = LineIndex::new(&text);
        let name = file.display().to_string();
        for d in &diagnostics {
            if d.span.file != FileId::default() {
                continue;
            }
            let at = index.line_col(d.span.start);
            eprintln!(
                "{name}:{}:{}: {} {}: {}",
                at.line + 1,
                at.col + 1,
                d.severity.as_str(),
                d.code.id(),
                d.message
            );
            if d.severity > worst {
                worst = d.severity;
            }
        }
    }
    if failed {
        ExitCode::from(2)
    } else if worst == Severity::Error || (strict && worst == Severity::Warning) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `path` expressed relative to `base` when both are relative, else as is.
fn pathdiff(path: &std::path::Path, base: &std::path::Path) -> PathBuf {
    if path.is_absolute() || base.as_os_str().is_empty() {
        return path.to_path_buf();
    }
    let ups = base.components().count();
    let mut out = PathBuf::new();
    for _ in 0..ups {
        out.push("..");
    }
    out.join(path)
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
        errors |= d.severity == Severity::Error;
        eprintln!(
            "{name}:{}:{}: {} {}: {}",
            at.line + 1,
            at.col + 1,
            d.severity.as_str(),
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

fn cmd_fmt(files: &[PathBuf], profile: Option<&str>, check: bool, write: bool) -> ExitCode {
    let flag = match profile {
        None => None,
        Some("canonical") => Some(Profile::Canonical),
        Some("strict") => Some(Profile::Strict),
        Some("mkdocs") => Some(Profile::Mkdocs),
        Some(other) => {
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
        let profile = match flag {
            Some(profile) => profile,
            None => match config_for(file) {
                Ok(config) => config.profile,
                Err(code) => return code,
            },
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
