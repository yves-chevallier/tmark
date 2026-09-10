//! The analysis worker: resolve and lint off the main loop, debounced.
//!
//! Design: `08-lsp.md` §Shape: "debounce 150 ms: resolve + lint on worker
//! → publish all diagnostics". The main loop parses synchronously and hands
//! the parsed document here; a request never waits for this thread.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use lsp_types::Uri;
use tmark::ir::LineIndex;
use tmark::{Diagnostic, Document, FileId, FsLoader, LintConfig, ResolveOptions, Resolved};

/// Quiet time after the last change before an analysis starts.
pub const DEBOUNCE: Duration = Duration::from_millis(150);

pub struct Check {
    pub uri: Uri,
    pub version: i32,
    pub options: ResolveOptions,
    pub text: String,
    pub document: Document,
    pub lint: LintConfig,
}

pub enum Job {
    Check(Box<Check>),
    /// The document was closed: forget any pending analysis.
    Drop(Uri),
}

pub struct Analysis {
    pub uri: Uri,
    pub version: i32,
    pub resolved: Resolved,
    /// Resolve and lint diagnostics; parse diagnostics stay with the
    /// document.
    pub diagnostics: Vec<Diagnostic>,
    /// Diagnostics of the included files, converted here (their text is
    /// read again from disk for the positions), one entry per file
    /// (design `08-lsp.md`: "published per file, including for included
    /// files that are not open").
    pub included: Vec<(Uri, Vec<lsp_types::Diagnostic>)>,
}

/// Runs until the job channel closes. Jobs for the same document replace
/// each other; the batch runs once `DEBOUNCE` passes without a new job.
pub fn run(jobs: Receiver<Job>, results: Sender<Analysis>) {
    // Keyed by the URI text: `Uri` has interior mutability (clippy's
    // `mutable_key_type`).
    let mut pending: HashMap<String, Box<Check>> = HashMap::new();
    loop {
        let job = if pending.is_empty() {
            match jobs.recv() {
                Ok(job) => job,
                Err(_) => return,
            }
        } else {
            match jobs.recv_timeout(DEBOUNCE) {
                Ok(job) => job,
                Err(RecvTimeoutError::Timeout) => {
                    for check in pending.drain().map(|(_, check)| check) {
                        if results.send(analyse(*check)).is_err() {
                            return;
                        }
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        };
        match job {
            Job::Drop(uri) => {
                pending.remove(uri.as_str());
            }
            Job::Check(check) => {
                pending.insert(check.uri.as_str().to_string(), check);
            }
        }
    }
}

fn analyse(check: Check) -> Analysis {
    let Check {
        uri,
        version,
        options,
        text,
        document,
        lint,
    } = check;
    let (resolved, diagnostics) = tmark::analyse(&document, &text, &FsLoader, &options, &lint);
    debug_assert_eq!(document.file, FileId::default());
    let base = options.path.parent().unwrap_or(Path::new("")).to_path_buf();
    let mut included = Vec::new();
    for (id, path) in &resolved.files {
        let absolute = if path.is_absolute() {
            path.clone()
        } else {
            base.join(path)
        };
        let Some(file_uri) = crate::convert::path_to_uri(&absolute) else {
            continue;
        };
        let Ok(file_text) = std::fs::read_to_string(&absolute) else {
            continue;
        };
        let index = LineIndex::new(&file_text);
        let converted: Vec<lsp_types::Diagnostic> = diagnostics
            .iter()
            .filter_map(|d| crate::convert::diagnostic(&file_uri, &index, *id, d))
            .collect();
        included.push((file_uri, converted));
    }
    Analysis {
        uri,
        version,
        resolved,
        diagnostics,
        included,
    }
}
