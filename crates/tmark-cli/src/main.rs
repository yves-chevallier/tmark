//! `tmark` command-line interface. Design: `design/09-bindings.md`.
//!
//! `parse`, `fmt`, `check`, `lint`, `write` and `schema`.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tmark::ir::{Code, LineIndex, Severity};
use tmark::{
    check, format, parse, Backend, Config, FileId, FsLoader, LintConfig, Media, Profile,
    ResolveOptions, WriterOptions,
};

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
        /// Apply the safe fixes (deprecated spellings): the files are
        /// rewritten in place unless `--stdout` or `--diff` is given.
        #[arg(long)]
        fix: bool,
        /// With `--fix`: print the fixed text on stdout, write nothing.
        #[arg(long, requires = "fix", conflicts_with = "diff")]
        stdout: bool,
        /// With `--fix`: print a unified diff of the fixes, write nothing.
        #[arg(long, requires = "fix")]
        diff: bool,
    },
    /// Alias of `check`.
    Lint {
        files: Vec<PathBuf>,
        #[arg(long)]
        strict: bool,
        #[arg(long = "level", value_name = "CODE=LEVEL")]
        levels: Vec<String>,
        /// Apply the safe fixes (deprecated spellings): the files are
        /// rewritten in place unless `--stdout` or `--diff` is given.
        #[arg(long)]
        fix: bool,
        /// With `--fix`: print the fixed text on stdout, write nothing.
        #[arg(long, requires = "fix", conflicts_with = "diff")]
        stdout: bool,
        /// With `--fix`: print a unified diff of the fixes, write nothing.
        #[arg(long, requires = "fix")]
        diff: bool,
    },
    /// Write the body of a file for a backend (design 07-writers.md).
    Write {
        /// The file to write; `-` for standard input.
        file: PathBuf,
        /// `latex`, `typst` or `html`.
        #[arg(long, value_name = "BACKEND")]
        to: String,
        /// `print` (default for latex and typst) or `web` (default for html).
        #[arg(long)]
        media: Option<String>,
        /// Print the whole `Body` as JSON (`text`, `map`, `requires`)
        /// instead of the text alone.
        #[arg(long)]
        map: bool,
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
            fix,
            stdout,
            diff,
        }
        | Command::Lint {
            files,
            strict,
            levels,
            fix,
            stdout,
            diff,
        } => {
            let mode = match (fix, stdout, diff) {
                (false, _, _) => FixMode::Off,
                (true, true, _) => FixMode::Stdout,
                (true, _, true) => FixMode::Diff,
                (true, false, false) => FixMode::Write,
            };
            cmd_check(&files, strict, &levels, mode)
        }
        Command::Write {
            file,
            to,
            media,
            map,
        } => cmd_write(&file, &to, media.as_deref(), map),
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

/// Splice every fix into `text`, last first so that earlier spans stay
/// valid; overlapping fixes after the first are skipped.
fn apply_fixes(text: &str, diagnostics: &[tmark::Diagnostic]) -> (String, usize) {
    let mut fixes: Vec<&tmark::ir::Fix> = diagnostics
        .iter()
        .filter_map(|d| d.fix.as_ref())
        .filter(|f| f.span.file == FileId::default())
        .collect();
    fixes.sort_by_key(|f| std::cmp::Reverse(f.span.start));
    let mut out = text.to_string();
    let mut applied = 0;
    let mut limit = text.len() as u32;
    for fix in fixes {
        if fix.span.end > limit {
            continue;
        }
        out.replace_range(
            fix.span.start as usize..fix.span.end as usize,
            &fix.replacement,
        );
        limit = fix.span.start;
        applied += 1;
    }
    (out, applied)
}

/// What `--fix` does with the fixed text (design 09 §CLI).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FixMode {
    /// No `--fix`: report every diagnostic.
    Off,
    /// Rewrite the file in place (the default of `--fix`).
    Write,
    /// `--stdout`: print the fixed text, write nothing.
    Stdout,
    /// `--diff`: print a unified diff, write nothing.
    Diff,
}

/// A unified diff of `before` → `after` with three lines of context, in the
/// `diff -u` layout (`--- name`, `+++ name`, `@@ -a,b +c,d @@` hunks).
fn unified_diff(name: &str, before: &str, after: &str) -> String {
    let before: Vec<&str> = before.lines().collect();
    let after: Vec<&str> = after.lines().collect();
    let changes = diff::slice(&before, &after);
    // Line-level diff over `changes`, with each line's position on both sides.
    let mut old_line = 0usize;
    let mut new_line = 0usize;
    let mut lines: Vec<(char, usize, usize, &str)> = Vec::with_capacity(changes.len());
    for change in &changes {
        match change {
            diff::Result::Left(l) => {
                old_line += 1;
                lines.push(('-', old_line, new_line, l));
            }
            diff::Result::Right(r) => {
                new_line += 1;
                lines.push(('+', old_line, new_line, r));
            }
            diff::Result::Both(l, _) => {
                old_line += 1;
                new_line += 1;
                lines.push((' ', old_line, new_line, l));
            }
        }
    }
    const CONTEXT: usize = 3;
    let mut out = format!(
        "--- {name}
+++ {name}
"
    );
    let mut i = 0;
    while i < lines.len() {
        if lines[i].0 == ' ' {
            i += 1;
            continue;
        }
        // A hunk: from `CONTEXT` lines before this change to `CONTEXT`
        // lines after the last change that is within `2 * CONTEXT` of it.
        let start = i.saturating_sub(CONTEXT);
        let mut last_change = i;
        let mut probe = i;
        while probe < lines.len() {
            if lines[probe].0 != ' ' {
                last_change = probe;
            } else if probe - last_change > 2 * CONTEXT {
                break;
            }
            probe += 1;
        }
        let end = (last_change + CONTEXT + 1).min(lines.len());
        let hunk = &lines[start..end];
        let old_count = hunk.iter().filter(|l| l.0 != '+').count();
        let new_count = hunk.iter().filter(|l| l.0 != '-').count();
        let old_start = hunk
            .iter()
            .find(|l| l.0 != '+')
            .map_or(hunk[0].1, |l| l.1)
            .max(usize::from(old_count > 0));
        let new_start = hunk
            .iter()
            .find(|l| l.0 != '-')
            .map_or(hunk[0].2, |l| l.2)
            .max(usize::from(new_count > 0));
        out.push_str(&format!(
            "@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"
        ));
        for (sign, _, _, text) in hunk {
            out.push(*sign);
            out.push_str(text);
            out.push('\n');
        }
        i = end;
    }
    out
}

fn cmd_check(files: &[PathBuf], strict: bool, levels: &[String], fix: FixMode) -> ExitCode {
    let fix_on = fix != FixMode::Off;
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
        let (_, diagnostics) = check(
            &text,
            FileId::default(),
            workspace.profile,
            &FsLoader,
            &options,
            &config,
        );
        let index = LineIndex::new(&text);
        let name = file.display().to_string();
        match fix {
            FixMode::Off => {}
            FixMode::Write => {
                let (fixed, applied) = apply_fixes(&text, &diagnostics);
                if applied > 0 && file.as_os_str() != "-" {
                    if let Err(error) = std::fs::write(file, &fixed) {
                        eprintln!("tmark: {}: {error}", file.display());
                        failed = true;
                    }
                    eprintln!("{name}: {applied} fix(es) applied");
                }
            }
            FixMode::Stdout => {
                let (fixed, _) = apply_fixes(&text, &diagnostics);
                let mut out = io::stdout().lock();
                let _ = out.write_all(fixed.as_bytes());
            }
            FixMode::Diff => {
                let (fixed, applied) = apply_fixes(&text, &diagnostics);
                if applied > 0 {
                    let mut out = io::stdout().lock();
                    let _ = out.write_all(unified_diff(&name, &text, &fixed).as_bytes());
                }
            }
        }
        for d in &diagnostics {
            if d.span.file != FileId::default() {
                continue;
            }
            if fix_on && d.fix.is_some() {
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
        let parsed = tmark::parse_with(&text, FileId::default(), profile);
        if parsed
            .diagnostics
            .iter()
            .any(|d| d.code == Code::ParseInternal)
        {
            eprintln!(
                "tmark: {}: the tokenizer failed on this file; not formatted",
                file.display()
            );
            failed = true;
            continue;
        }
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

fn cmd_write(file: &PathBuf, to: &str, media: Option<&str>, map: bool) -> ExitCode {
    let Some(backend) = Backend::parse(to) else {
        eprintln!("tmark: unknown backend `{to}` (latex, typst, html)");
        return ExitCode::from(2);
    };
    let media = match media {
        None => match backend {
            Backend::Html => Media::Web,
            Backend::Latex | Backend::Typst => Media::Print,
        },
        Some("print") => Media::Print,
        Some("web") => Media::Web,
        Some(other) => {
            eprintln!("tmark: unknown media `{other}` (print, web)");
            return ExitCode::from(2);
        }
    };
    let text = match read(file) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("tmark: {}: {error}", file.display());
            return ExitCode::from(2);
        }
    };
    let workspace = match config_for(file) {
        Ok(config) => config,
        Err(code) => return code,
    };
    let parsed = tmark::parse_with(&text, FileId::default(), workspace.profile);
    let options = workspace.resolve_options(file);
    let resolved = tmark::resolve(&parsed.document, &FsLoader, &options);
    let index = LineIndex::new(&text);
    let name = file.display().to_string();
    for d in parsed.diagnostics.iter().chain(&resolved.diagnostics) {
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
    }
    let opts = WriterOptions {
        media,
        lang: parsed.document.front_matter.keys.lang.clone(),
        source_map: map,
        ..WriterOptions::default()
    };
    let body = tmark::write(&parsed.document, &resolved, backend, &opts);
    let mut out = io::stdout().lock();
    if map {
        let json = serde_json::to_string_pretty(&body).expect("the body serialises");
        let _ = writeln!(out, "{json}");
    } else {
        let _ = out.write_all(body.text.as_bytes());
    }
    ExitCode::SUCCESS
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

#[cfg(test)]
mod tests {
    use super::unified_diff;

    #[test]
    fn unified_diff_has_hunks_with_context() {
        let before = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n";
        let after = "a\nb\nc\nD\ne\nf\ng\nh\ni\nJ\n";
        let diff = unified_diff("x.md", before, after);
        assert_eq!(
            diff,
            "--- x.md\n+++ x.md\n@@ -1,10 +1,10 @@\n a\n b\n c\n-d\n+D\n e\n f\n g\n h\n i\n-j\n+J\n"
        );
        let far = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\n";
        let far_after = "A\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nM\n";
        let diff = unified_diff("x.md", far, far_after);
        assert!(diff.contains("@@ -1,4 +1,4 @@\n-a\n+A\n b\n c\n d\n"));
        assert!(diff.contains("@@ -10,4 +10,4 @@\n j\n k\n l\n-m\n+M\n"));
        assert_eq!(unified_diff("x.md", "a\n", "a\n"), "--- x.md\n+++ x.md\n");
    }
}
