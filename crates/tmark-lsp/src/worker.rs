//! The analysis worker: resolve and lint off the main loop, debounced.
//!
//! Design: `08-lsp.md` §Shape: "debounce 150 ms: resolve + lint on worker
//! → publish all diagnostics". The main loop parses synchronously and hands
//! the parsed document here; a request never waits for this thread.

use std::collections::HashMap;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use lsp_types::Uri;
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
    let resolved = tmark::resolve(&document, &FsLoader, &options);
    let mut diagnostics = resolved.diagnostics.clone();
    diagnostics.extend(tmark::lint(&document, &resolved, &text, &lint));
    debug_assert_eq!(document.file, FileId::default());
    Analysis {
        uri,
        version,
        resolved,
        diagnostics,
    }
}
