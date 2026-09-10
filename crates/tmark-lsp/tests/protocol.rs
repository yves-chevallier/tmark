//! Drives the server over an in-memory connection: JSON in, notifications
//! and responses out. Design `08-lsp.md`, handoff step 1.

use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{json, Value};

struct Client {
    conn: Connection,
    next_id: i32,
}

impl Client {
    fn start() -> (Client, std::thread::JoinHandle<Result<(), String>>) {
        let (server, client) = Connection::memory();
        let handle = std::thread::spawn(move || tmark_lsp::run(server).map_err(|e| e.to_string()));
        let mut client = Client {
            conn: client,
            next_id: 0,
        };
        let init = client.request(
            "initialize",
            json!({
                "processId": null,
                "rootUri": null,
                "capabilities": {
                    "textDocument": {
                        "documentSymbol": { "hierarchicalDocumentSymbolSupport": true }
                    }
                }
            }),
        );
        assert!(init["capabilities"]["documentSymbolProvider"]
            .as_bool()
            .unwrap());
        client.notify("initialized", json!({}));
        (client, handle)
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        self.conn
            .sender
            .send(Request::new(id.clone(), method.into(), params).into())
            .unwrap();
        loop {
            match self.recv() {
                Message::Response(Response {
                    id: got,
                    result,
                    error,
                }) if got == id => {
                    assert!(error.is_none(), "{method}: {error:?}");
                    return result.unwrap_or(Value::Null);
                }
                _ => continue,
            }
        }
    }

    fn notify(&self, method: &str, params: Value) {
        self.conn
            .sender
            .send(Notification::new(method.into(), params).into())
            .unwrap();
    }

    fn recv(&self) -> Message {
        self.conn
            .receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("the server answers within 10 s")
    }

    /// The next `publishDiagnostics` for `uri`, as (version, diagnostics).
    fn diagnostics(&self, uri: &str) -> (Option<i64>, Vec<Value>) {
        loop {
            if let Message::Notification(n) = self.recv() {
                if n.method == "textDocument/publishDiagnostics" && n.params["uri"] == uri {
                    let diagnostics = n.params["diagnostics"].as_array().unwrap().clone();
                    return (n.params["version"].as_i64(), diagnostics);
                }
            }
        }
    }

    fn open(&self, uri: &str, language: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": language, "version": 1, "text": text}}),
        );
    }

    fn change(&self, uri: &str, version: i32, text: &str) {
        self.notify(
            "textDocument/didChange",
            json!({"textDocument": {"uri": uri, "version": version}, "contentChanges": [{"text": text}]}),
        );
    }

    fn shutdown(mut self, handle: std::thread::JoinHandle<Result<(), String>>) {
        self.request("shutdown", Value::Null);
        self.notify("exit", Value::Null);
        handle.join().unwrap().unwrap();
    }
}

const DOC: &str = "\
# Intro {#sec:intro}

See @sec:intro and @fig:missing. Now €uro ünïcode: @sec:intro.

::: note
A note with {unknown}[x].
:::

## Details

Table: Caption {#tbl:t}

| a | b |
| - | - |
| 1 | 2 |
";

#[test]
fn diagnostics_arrive_in_two_waves() {
    let (client, handle) = Client::start();
    let uri = "file:///tmp/tmark-test/doc.tmd";
    client.open(uri, "tmark", DOC);
    // Wave 1: parse diagnostics, immediately (the unknown role hint).
    let (version, first) = client.diagnostics(uri);
    assert_eq!(version, Some(1));
    let codes: Vec<&str> = first.iter().map(|d| d["code"].as_str().unwrap()).collect();
    assert!(codes.contains(&"role-unknown"), "{codes:?}");
    assert!(!codes.contains(&"ref-unresolved"), "{codes:?}");
    // Wave 2: resolve and lint, after the debounce.
    let (version, second) = client.diagnostics(uri);
    assert_eq!(version, Some(1));
    let unresolved = second
        .iter()
        .find(|d| d["code"] == "ref-unresolved")
        .expect("@fig:missing is reported");
    assert_eq!(unresolved["source"], "tmark");
    assert_eq!(unresolved["severity"], 2);
    assert_eq!(unresolved["range"]["start"]["line"], 2);
    assert_eq!(unresolved["range"]["start"]["character"], 19);
    assert!(second.iter().any(|d| d["code"] == "role-unknown"));
    client.shutdown(handle);
}

#[test]
fn stale_analysis_is_not_published() {
    let (client, handle) = Client::start();
    let uri = "file:///tmp/tmark-test/edit.tmd";
    client.open(uri, "tmark", "See @fig:a.\n");
    let (v, _) = client.diagnostics(uri);
    assert_eq!(v, Some(1));
    client.change(uri, 2, "See @fig:b.\n");
    let (v, _) = client.diagnostics(uri);
    assert_eq!(v, Some(2));
    // Every later wave carries the latest version.
    let (v, diags) = client.diagnostics(uri);
    assert_eq!(v, Some(2));
    assert!(diags
        .iter()
        .any(|d| d["message"].as_str().unwrap().contains("fig:b")));
    client.shutdown(handle);
}

#[test]
fn undetected_markdown_stays_quiet() {
    let (mut client, handle) = Client::start();
    let readme = "file:///tmp/tmark-test/README.md";
    client.open(readme, "markdown", "# Readme\n\nSee @fig:missing.\n");
    let pressed = "file:///tmp/tmark-test/pressed.md";
    client.open(
        pressed,
        "markdown",
        "---\npress:\n  template: article\n---\n# Doc\n\nSee @fig:missing.\n",
    );
    // Only the detected file publishes; if the README published first we
    // would see it before this one.
    let (_, diags) = client.diagnostics(pressed);
    assert!(diags.is_empty());
    let (_, diags) = client.diagnostics(pressed);
    assert!(diags.iter().any(|d| d["code"] == "ref-unresolved"));
    // The README still answers structural requests.
    let symbols = client.request(
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": readme}}),
    );
    assert_eq!(symbols[0]["name"], "Readme");
    client.shutdown(handle);
}

#[test]
fn symbols_folds_and_formatting() {
    let (mut client, handle) = Client::start();
    let uri = "file:///tmp/tmark-test/outline.tmd";
    client.open(uri, "tmark", DOC);
    let symbols = client.request(
        "textDocument/documentSymbol",
        json!({"textDocument": {"uri": uri}}),
    );
    let intro = &symbols[0];
    assert_eq!(intro["name"], "Intro");
    assert_eq!(intro["detail"], "#sec:intro");
    assert_eq!(intro["range"]["start"]["line"], 0);
    assert_eq!(intro["range"]["end"]["line"], 14, "{intro}");
    let children = intro["children"].as_array().unwrap();
    let names: Vec<&str> = children
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["note", "Details"]);
    let details = &children[1];
    assert_eq!(details["children"][0]["name"], "Caption");
    assert_eq!(details["children"][0]["detail"], "Table #tbl:t");

    let folds = client.request(
        "textDocument/foldingRange",
        json!({"textDocument": {"uri": uri}}),
    );
    let folds: Vec<(u64, u64)> = folds
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["startLine"].as_u64().unwrap(),
                f["endLine"].as_u64().unwrap(),
            )
        })
        .collect();
    assert!(folds.contains(&(0, 14)), "section Intro: {folds:?}");
    assert!(folds.contains(&(4, 6)), "admonition: {folds:?}");
    assert!(folds.contains(&(8, 14)), "section Details: {folds:?}");
    assert!(folds.contains(&(12, 14)), "table: {folds:?}");

    let edits = client.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": uri}, "options": {"tabSize": 4, "insertSpaces": true}}),
    );
    let edits = edits.as_array().unwrap();
    assert_eq!(
        edits.len(),
        1,
        "the caption-before spelling is not canonical"
    );
    let text = edits[0]["newText"].as_str().unwrap();
    let table = text.find("| 1 ").expect("the table is printed");
    let caption = text
        .find("Table: Caption {#tbl:t}")
        .expect("the caption is printed");
    assert!(
        caption > table,
        "caption after the table (canonical): {text}"
    );
    assert_eq!(
        edits[0]["range"]["start"],
        json!({"line": 0, "character": 0})
    );
    client.change(uri, 2, text);
    let (_, _) = client.diagnostics(uri);
    let edits = client.request(
        "textDocument/formatting",
        json!({"textDocument": {"uri": uri}, "options": {"tabSize": 4, "insertSpaces": true}}),
    );
    assert_eq!(
        edits.as_array().unwrap().len(),
        0,
        "formatting is idempotent"
    );
    client.shutdown(handle);
}
