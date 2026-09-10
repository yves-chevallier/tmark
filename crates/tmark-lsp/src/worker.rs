//! The analysis worker: resolve and lint off the main loop, debounced.
//!
//! Design: `08-lsp.md` §Shape: "debounce 150 ms: resolve + lint on worker
//! → publish all diagnostics". The main loop parses synchronously and hands
//! the parsed document here; a request never waits for this thread.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use lsp_types::Uri;
use tmark::{Diagnostic, Document, FileId, FsLoader, LintConfig, ResolveOptions, Resolved};

/// Quiet time after the last change before an analysis starts.
pub const DEBOUNCE: Duration = Duration::from_millis(150);

pub enum Job {
    Check {
        uri: Uri,
        version: i32,
        path: PathBuf,
        text: String,
        document: Document,
        lint: LintConfig,
    },
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
    let mut pending: HashMap<Uri, Job> = HashMap::new();
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
                    for job in pending.drain().map(|(_, job)| job) {
                        if let Some(analysis) = analyse(job) {
                            if results.send(analysis).is_err() {
                                return;
                            }
                        }
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        };
        match job {
            Job::Drop(uri) => {
                pending.remove(&uri);
            }
            Job::Check { ref uri, .. } => {
                pending.insert(uri.clone(), job);
            }
        }
    }
}

fn analyse(job: Job) -> Option<Analysis> {
    let Job::Check {
        uri,
        version,
        path,
        text,
        document,
        lint,
    } = job
    else {
        return None;
    };
    let options = ResolveOptions {
        path,
        ..Default::default()
    };
    let resolved = tmark::resolve(&document, &FsLoader, &options);
    let mut diagnostics = resolved.diagnostics.clone();
    diagnostics.extend(tmark::lint(&document, &resolved, &text, &lint));
    debug_assert_eq!(document.file, FileId::default());
    Some(Analysis {
        uri,
        version,
        resolved,
        diagnostics,
    })
}
