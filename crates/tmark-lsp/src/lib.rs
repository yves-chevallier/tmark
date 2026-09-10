//! The TMark language server: one synchronous main loop over a store of
//! open documents, a worker thread for resolve and lint (ADR 0007).
//!
//! Design: `design/08-lsp.md`. Milestone 3 features land here in the order
//! of `design/13-handoff.md`; this file holds the loop and the document
//! store, the features live in their modules.

#![forbid(unsafe_code)]

mod convert;
mod outline;
mod worker;

use std::collections::HashMap;
use std::path::PathBuf;

use crossbeam_channel::{select, Receiver, Sender};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    Notification as _, PublishDiagnostics, ShowMessage,
};
use lsp_types::request::{DocumentSymbolRequest, FoldingRangeRequest, Formatting, Request as _};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentFormattingParams, DocumentSymbolParams, DocumentSymbolResponse, FoldingRangeParams,
    FoldingRangeProviderCapability, InitializeParams, InitializeResult, MessageType, OneOf,
    PublishDiagnosticsParams, ServerCapabilities, ServerInfo, ShowMessageParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Uri,
};
use tmark::ir::LineIndex;
use tmark::{Config, Diagnostic, Document, FileId, Profile, Resolved};

use worker::{Analysis, Job};

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Serve `connection` until the client asks to exit.
pub fn run(connection: Connection) -> Result<(), Error> {
    let (id, params) = connection.initialize_start()?;
    let init: InitializeParams = serde_json::from_value(params)?;
    let result = InitializeResult {
        capabilities: capabilities(),
        server_info: Some(ServerInfo {
            name: "tmark-lsp".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
    };
    connection.initialize_finish(id, serde_json::to_value(result)?)?;

    let (jobs, job_rx) = crossbeam_channel::unbounded();
    let (result_tx, results) = crossbeam_channel::unbounded();
    let worker = std::thread::Builder::new()
        .name("tmark-analysis".into())
        .spawn(move || worker::run(job_rx, result_tx))?;

    let mut server = Server::new(connection.sender.clone(), jobs, &init);
    let outcome = server.main_loop(&connection, &results);
    drop(server);
    let _ = worker.join();
    outcome
}

fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        document_symbol_provider: Some(OneOf::Left(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    }
}

/// One open document: its text, the parse of that text, and the latest
/// analysis that the worker finished (possibly of an older version).
struct Doc {
    text: String,
    version: i32,
    index: LineIndex,
    document: Document,
    parse_diagnostics: Vec<Diagnostic>,
    /// Design ADR 0006: `.md` files that are not detected as TMark get no
    /// diagnostics.
    detected: bool,
    language_id: String,
    path: PathBuf,
    /// The nearest `tmark.toml`, or the defaults.
    config: Config,
    analysis: Option<Analysis>,
}

impl Doc {
    /// The resolution matching the current text, if the worker has caught
    /// up.
    #[allow(dead_code)] // navigation and completion arrive next
    fn resolved(&self) -> Option<&Resolved> {
        self.analysis
            .as_ref()
            .filter(|a| a.version == self.version)
            .map(|a| &a.resolved)
    }
}

struct Server {
    sender: Sender<Message>,
    jobs: Sender<Job>,
    docs: HashMap<Uri, Doc>,
    hierarchical_symbols: bool,
    /// The last configuration error shown, so that it is shown once.
    config_error: Option<String>,
}

impl Server {
    fn new(sender: Sender<Message>, jobs: Sender<Job>, init: &InitializeParams) -> Self {
        let hierarchical_symbols = init
            .capabilities
            .text_document
            .as_ref()
            .and_then(|t| t.document_symbol.as_ref())
            .and_then(|d| d.hierarchical_document_symbol_support)
            .unwrap_or(false);
        Server {
            sender,
            jobs,
            docs: HashMap::new(),
            hierarchical_symbols,
            config_error: None,
        }
    }

    fn main_loop(
        &mut self,
        connection: &Connection,
        results: &Receiver<Analysis>,
    ) -> Result<(), Error> {
        loop {
            select! {
                recv(connection.receiver) -> msg => match msg? {
                    Message::Request(req) => {
                        if connection.handle_shutdown(&req)? {
                            return Ok(());
                        }
                        let response = self.handle_request(req);
                        self.sender.send(response.into())?;
                    }
                    Message::Notification(n) => self.handle_notification(n)?,
                    Message::Response(_) => {}
                },
                recv(results) -> analysis => {
                    let analysis = analysis?;
                    self.on_analysis(analysis)?;
                }
            }
        }
    }

    // --- Documents ---

    fn handle_notification(&mut self, n: Notification) -> Result<(), Error> {
        match n.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let p: DidOpenTextDocumentParams = serde_json::from_value(n.params)?;
                let d = p.text_document;
                self.update(d.uri, d.version, d.text, d.language_id)?;
            }
            DidChangeTextDocument::METHOD => {
                let p: DidChangeTextDocumentParams = serde_json::from_value(n.params)?;
                // Full sync: the last change carries the whole text.
                let Some(change) = p.content_changes.into_iter().last() else {
                    return Ok(());
                };
                let uri = p.text_document.uri;
                let language_id = self
                    .docs
                    .get(&uri)
                    .map_or("markdown".to_string(), |doc| doc.language_id.clone());
                self.update(uri, p.text_document.version, change.text, language_id)?;
            }
            DidChangeWatchedFiles::METHOD => {
                // `tmark.toml` or a `.bib` changed: every document re-reads
                // its configuration and re-analyses.
                let uris: Vec<Uri> = self.docs.keys().cloned().collect();
                for uri in uris {
                    let Some(doc) = self.docs.get(&uri) else {
                        continue;
                    };
                    let (version, text, language_id) =
                        (doc.version, doc.text.clone(), doc.language_id.clone());
                    self.update(uri, version, text, language_id)?;
                }
            }
            DidCloseTextDocument::METHOD => {
                let p: DidCloseTextDocumentParams = serde_json::from_value(n.params)?;
                let uri = p.text_document.uri;
                if self.docs.remove(&uri).is_some() {
                    let _ = self.jobs.send(Job::Drop(uri.clone()));
                    self.send_diagnostics(uri, Vec::new(), None)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Parse now, publish parse diagnostics now, queue the analysis.
    fn update(
        &mut self,
        uri: Uri,
        version: i32,
        text: String,
        language_id: String,
    ) -> Result<(), Error> {
        let path = convert::uri_to_path(&uri).unwrap_or_else(|| PathBuf::from(uri.as_str()));
        let (config, has_config) = match Config::discover(&path) {
            None => (Config::default(), false),
            Some(Ok(config)) => (config, true),
            Some(Err(error)) => {
                if self.config_error.as_deref() != Some(&error) {
                    self.show_message(MessageType::WARNING, error.clone())?;
                    self.config_error = Some(error);
                }
                (Config::default(), true)
            }
        };
        let detected = is_tmark(&language_id, &text, has_config);
        let parsed = if config.profile == Profile::Strict {
            tmark::parse_strict(&text, FileId::default())
        } else {
            tmark::parse(&text, FileId::default())
        };
        let index = LineIndex::new(&text);
        let analysis = self.docs.remove(&uri).and_then(|old| old.analysis);
        let doc = Doc {
            index,
            document: parsed.document,
            parse_diagnostics: parsed.diagnostics,
            detected,
            language_id,
            path,
            config,
            analysis,
            version,
            text,
        };
        if detected {
            let _ = self.jobs.send(Job::Check {
                uri: uri.clone(),
                version,
                options: doc.config.resolve_options(&doc.path),
                text: doc.text.clone(),
                document: doc.document.clone(),
                lint: doc.config.lint.clone(),
            });
        }
        self.docs.insert(uri.clone(), doc);
        self.publish(&uri)
    }

    fn on_analysis(&mut self, analysis: Analysis) -> Result<(), Error> {
        let uri = analysis.uri.clone();
        let Some(doc) = self.docs.get_mut(&uri) else {
            return Ok(());
        };
        if analysis.version != doc.version {
            // Stale: a newer version is queued or running.
            return Ok(());
        }
        doc.analysis = Some(analysis);
        self.publish(&uri)
    }

    /// Diagnostics of the current text: parse ones, plus resolve and lint
    /// ones when the analysis matches the version.
    fn publish(&self, uri: &Uri) -> Result<(), Error> {
        let Some(doc) = self.docs.get(uri) else {
            return Ok(());
        };
        if !doc.detected {
            return Ok(());
        }
        let file = doc.document.file;
        let analysed = doc
            .analysis
            .as_ref()
            .filter(|a| a.version == doc.version)
            .map(|a| a.diagnostics.as_slice())
            .unwrap_or_default();
        let diagnostics = doc
            .parse_diagnostics
            .iter()
            .chain(analysed)
            .filter_map(|d| convert::diagnostic(uri, &doc.index, file, d))
            .collect();
        self.send_diagnostics(uri.clone(), diagnostics, Some(doc.version))
    }

    fn show_message(&self, typ: MessageType, message: String) -> Result<(), Error> {
        let params = ShowMessageParams { typ, message };
        self.sender
            .send(Notification::new(ShowMessage::METHOD.into(), params).into())?;
        Ok(())
    }

    fn send_diagnostics(
        &self,
        uri: Uri,
        diagnostics: Vec<lsp_types::Diagnostic>,
        version: Option<i32>,
    ) -> Result<(), Error> {
        let params = PublishDiagnosticsParams::new(uri, diagnostics, version);
        self.sender
            .send(Notification::new(PublishDiagnostics::METHOD.into(), params).into())?;
        Ok(())
    }

    // --- Requests ---

    fn handle_request(&mut self, req: Request) -> Response {
        let id = req.id.clone();
        let result = match req.method.as_str() {
            DocumentSymbolRequest::METHOD => with_params(req, |p: DocumentSymbolParams| {
                self.document_symbols(&p.text_document.uri)
            }),
            FoldingRangeRequest::METHOD => with_params(req, |p: FoldingRangeParams| {
                self.folding_ranges(&p.text_document.uri)
            }),
            Formatting::METHOD => with_params(req, |p: DocumentFormattingParams| {
                self.formatting(&p.text_document.uri)
            }),
            _ => Err(Response::new_err(
                id.clone(),
                ErrorCode::MethodNotFound as i32,
                format!("unsupported request `{}`", req.method),
            )),
        };
        match result {
            Ok(value) => Response::new_ok(id, value),
            Err(response) => response,
        }
    }

    fn document_symbols(&self, uri: &Uri) -> serde_json::Value {
        let Some(doc) = self.docs.get(uri) else {
            return serde_json::Value::Null;
        };
        let response = if self.hierarchical_symbols {
            DocumentSymbolResponse::Nested(outline::document_symbols(&doc.document, &doc.index))
        } else {
            DocumentSymbolResponse::Flat(outline::symbol_information(
                &doc.document,
                &doc.index,
                uri,
            ))
        };
        serde_json::to_value(response).expect("symbols serialise")
    }

    fn folding_ranges(&self, uri: &Uri) -> serde_json::Value {
        let Some(doc) = self.docs.get(uri) else {
            return serde_json::Value::Null;
        };
        serde_json::to_value(outline::folding_ranges(&doc.document, &doc.index))
            .expect("folds serialise")
    }

    /// Whole-document formatting: one edit replacing everything, or none
    /// when the text is already in normal form.
    fn formatting(&self, uri: &Uri) -> serde_json::Value {
        let Some(doc) = self.docs.get(uri) else {
            return serde_json::Value::Null;
        };
        let formatted = tmark::format(&doc.document, doc.config.profile);
        let edits: Vec<TextEdit> = if formatted == doc.text {
            Vec::new()
        } else {
            vec![TextEdit::new(convert::full_range(&doc.index), formatted)]
        };
        serde_json::to_value(edits).expect("edits serialise")
    }
}

/// Deserialise the request's params and run `f`, or answer with an
/// `InvalidParams` error.
fn with_params<P: serde::de::DeserializeOwned>(
    req: Request,
    f: impl FnOnce(P) -> serde_json::Value,
) -> Result<serde_json::Value, Response> {
    match serde_json::from_value::<P>(req.params) {
        Ok(p) => Ok(f(p)),
        Err(error) => Err(Response::new_err(
            req.id,
            ErrorCode::InvalidParams as i32,
            error.to_string(),
        )),
    }
}

/// ADR 0006: `tmark` files always; Markdown when the front matter has a
/// `press` key or a `tmark.toml` sits in a directory above the file.
fn is_tmark(language_id: &str, text: &str, has_config: bool) -> bool {
    match language_id {
        "tmark" => true,
        "markdown" => has_config || has_press_key(text),
        _ => false,
    }
}

/// A top-level `press:` line inside the front matter.
fn has_press_key(text: &str) -> bool {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return false;
    }
    for line in lines {
        if line.trim_end() == "---" || line.trim_end() == "..." {
            return false;
        }
        if line.starts_with("press:") {
            return true;
        }
    }
    false
}

/// Requests need an id; notifications do not. Kept for the tests.
#[doc(hidden)]
pub fn request_id(n: i32) -> RequestId {
    RequestId::from(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_key_detection() {
        assert!(has_press_key(
            "---\ntitle: x\npress:\n  template: a\n---\n# H\n"
        ));
        assert!(!has_press_key("---\ntitle: x\n---\npress: no\n"));
        assert!(!has_press_key("# H\npress: no\n"));
    }
}
