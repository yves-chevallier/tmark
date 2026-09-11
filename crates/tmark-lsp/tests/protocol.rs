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
                    },
                    "workspace": { "semanticTokens": { "refreshSupport": true } }
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
    assert_eq!(
        unresolved["range"]["start"]["character"], 20,
        "the key, not the `@`"
    );
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

#[test]
fn tmark_toml_detects_markdown_and_sets_lint_levels() {
    let dir = std::env::temp_dir().join(format!("tmark-lsp-{}", std::process::id()));
    let docs = dir.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        dir.join("tmark.toml"),
        "[lint]\nposition-word = \"off\"\nheading-skip = \"error\"\n",
    )
    .unwrap();
    let file = docs.join("chapter.md");
    let uri = format!("file://{}", file.display());
    let (client, handle) = Client::start();
    // Detected through the tmark.toml above the file: diagnostics flow.
    client.open(
        &uri,
        "markdown",
        "# One\n\n### Three\n\nSee the figure below.\n",
    );
    let (_, parse) = client.diagnostics(&uri);
    assert!(parse.is_empty());
    let (_, all) = client.diagnostics(&uri);
    let skip = all
        .iter()
        .find(|d| d["code"] == "heading-skip")
        .expect("heading-skip");
    assert_eq!(skip["severity"], 1, "raised to error by tmark.toml");
    assert!(
        !all.iter().any(|d| d["code"] == "position-word"),
        "switched off: {all:?}"
    );
    // A broken configuration is reported once, and the file keeps working.
    std::fs::write(dir.join("tmark.toml"), "profile = \"nope\"\n").unwrap();
    client.notify("workspace/didChangeWatchedFiles", json!({"changes": [{"uri": format!("file://{}", dir.join("tmark.toml").display()), "type": 2}]}));
    loop {
        if let Message::Notification(n) = client.recv() {
            if n.method == "window/showMessage" {
                assert!(n.params["message"].as_str().unwrap().contains("nope"));
                break;
            }
        }
    }
    let (_, _) = client.diagnostics(&uri);
    let (_, all) = client.diagnostics(&uri);
    assert!(
        all.iter().any(|d| d["code"] == "position-word"),
        "defaults again: {all:?}"
    );
    client.shutdown(handle);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn semantic_tokens_overlay_resolution_state() {
    let (mut client, handle) = Client::start();
    let uri = "file:///tmp/tmark-test/tokens.tmd";
    client.open(uri, "tmark", DOC);
    let _ = client.diagnostics(uri);
    let _ = client.diagnostics(uri);
    // The analysis asks the client to refresh its tokens.
    loop {
        if let Message::Request(r) = client.recv() {
            assert_eq!(r.method, "workspace/semanticTokens/refresh");
            break;
        }
    }
    let result = client.request(
        "textDocument/semanticTokens/full",
        json!({"textDocument": {"uri": uri}}),
    );
    let data: Vec<u64> = result["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    // Absolute (line, col, len, type) from the deltas.
    let mut tokens = Vec::new();
    let (mut line, mut col) = (0, 0);
    for t in data.chunks(5) {
        line += t[0];
        col = if t[0] == 0 { col + t[1] } else { t[1] };
        tokens.push((line, col, t[2], t[3]));
    }
    // Legend: 0 reference, 1 unresolvedReference, 2 citation, 3 label, 4 unknownRole, 5 deprecated.
    assert!(
        tokens.contains(&(2, 4, 10, 0)),
        "@sec:intro resolved: {tokens:?}"
    );
    assert!(
        tokens.contains(&(2, 19, 12, 1)),
        "@fig:missing unresolved: {tokens:?}"
    );
    assert!(
        tokens.contains(&(2, 51, 10, 0)),
        "after non-ASCII, UTF-16 columns: {tokens:?}"
    );
    assert!(
        tokens.iter().any(|t| t.0 == 5 && t.3 == 4),
        "unknown role: {tokens:?}"
    );
    client.shutdown(handle);
}

#[test]
fn completion_offers_labels_and_roles() {
    let (mut client, handle) = Client::start();
    let uri = "file:///tmp/tmark-test/complete.tmd";
    client.open(uri, "tmark", DOC);
    let _ = client.diagnostics(uri);
    let _ = client.diagnostics(uri);
    // Type `@s` at the end.
    client.change(uri, 2, &format!("{DOC}\nSee @s"));
    let _ = client.diagnostics(uri);
    let line = DOC.lines().count() as u64 + 1;
    let result = client.request(
        "textDocument/completion",
        json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": 6}}),
    );
    let items = result.as_array().unwrap();
    let sec = items
        .iter()
        .find(|i| i["label"] == "sec:intro")
        .expect("sec:intro offered");
    assert_eq!(sec["textEdit"]["range"]["start"]["character"], 5);
    assert_eq!(sec["textEdit"]["range"]["end"]["character"], 6);
    assert!(items.iter().any(|i| i["label"] == "tbl:t"));
    let result = client.request(
        "textDocument/completion",
        json!({"textDocument": {"uri": uri}, "position": {"line": 5, "character": 13}}),
    );
    let items = result.as_array().unwrap();
    assert!(
        items.iter().any(|i| i["label"] == "aside"),
        "role names after `{{`"
    );
    client.shutdown(handle);
}

#[test]
fn navigation_across_an_include() {
    let dir = std::env::temp_dir().join(format!("tmark-lsp-nav-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("part.md"), "## Part {#sec:part}\n\nText.\n").unwrap();
    let main = dir.join("main.tmd");
    let main_text =
        "# Main {#sec:main}\n\n{include}(part.md)\n\nSee @sec:part and @[sec:main; sec:part].\n";
    std::fs::write(&main, main_text).unwrap();
    let uri = format!("file://{}", main.display());
    let part_uri = format!("file://{}", dir.join("part.md").display());
    let (mut client, handle) = Client::start();
    client.open(&uri, "tmark", main_text);
    let _ = client.diagnostics(&uri);
    let (_, all) = client.diagnostics(&uri);
    assert!(all.is_empty(), "every reference resolves: {all:?}");

    // Definition of `@sec:part` (line 4, col 5) is the id in part.md.
    let def = client.request(
        "textDocument/definition",
        json!({"textDocument": {"uri": uri}, "position": {"line": 4, "character": 6}}),
    );
    assert_eq!(def[0]["uri"], part_uri);
    assert_eq!(
        def[0]["range"]["start"],
        json!({"line": 0, "character": 10}),
        "after the `#`"
    );
    assert_eq!(def[0]["range"]["end"], json!({"line": 0, "character": 18}));

    // References of `sec:main` from its definition (line 0, col 9).
    let refs = client.request(
        "textDocument/references",
        json!({"textDocument": {"uri": uri}, "position": {"line": 0, "character": 10}, "context": {"includeDeclaration": true}}),
    );
    let refs = refs.as_array().unwrap();
    assert_eq!(refs.len(), 2, "{refs:?}");
    assert_eq!(
        refs[1]["range"]["start"],
        json!({"line": 4, "character": 20})
    );

    // Rename `sec:part` from the reference: two files change.
    let prepared = client.request(
        "textDocument/prepareRename",
        json!({"textDocument": {"uri": uri}, "position": {"line": 4, "character": 6}}),
    );
    assert_eq!(prepared["placeholder"], "sec:part");
    let edit = client.request(
        "textDocument/rename",
        json!({"textDocument": {"uri": uri}, "position": {"line": 4, "character": 6}, "newName": "sec:appendix"}),
    );
    let changes = edit["changes"].as_object().unwrap();
    assert_eq!(changes[&part_uri][0]["newText"], "sec:appendix");
    assert_eq!(changes[&part_uri][0]["range"]["start"]["character"], 10);
    let main_edits = changes[uri.as_str()].as_array().unwrap();
    assert_eq!(main_edits.len(), 2, "{main_edits:?}");
    assert!(main_edits.iter().all(|e| e["newText"] == "sec:appendix"));

    // Hover on `@[sec:main` names the host and its heading text.
    let hover = client.request(
        "textDocument/hover",
        json!({"textDocument": {"uri": uri}, "position": {"line": 4, "character": 22}}),
    );
    let value = hover["contents"]["value"].as_str().unwrap();
    assert!(
        value.contains("section") && value.contains("Main"),
        "{value}"
    );

    // The include is a document link.
    let links = client.request(
        "textDocument/documentLink",
        json!({"textDocument": {"uri": uri}}),
    );
    assert_eq!(links[0]["target"], part_uri);
    client.shutdown(handle);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn code_actions_fix_deprecated_spellings() {
    let (mut client, handle) = Client::start();
    let uri = "file:///tmp/tmark-test/fix.tmd";
    client.open(uri, "tmark", "Raw {latex}[x] here.\n");
    let (_, diags) = client.diagnostics(uri);
    assert!(diags.iter().any(|d| d["code"] == "deprecated"));
    let actions = client.request(
        "textDocument/codeAction",
        json!({"textDocument": {"uri": uri}, "range": {"start": {"line": 0, "character": 5}, "end": {"line": 0, "character": 5}}, "context": {"diagnostics": []}}),
    );
    let action = &actions[0];
    assert_eq!(action["kind"], "quickfix");
    let edit = &action["edit"]["changes"][uri][0];
    assert_eq!(edit["newText"], "{raw latex}(x)");
    assert_eq!(edit["range"]["start"]["character"], 4);
    assert_eq!(edit["range"]["end"]["character"], 14);
    client.shutdown(handle);
}

#[test]
fn included_files_get_their_own_diagnostics() {
    let dir = std::env::temp_dir().join(format!("tmark-lsp-inc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // A parse diagnostic and an unresolved reference, both inside the
    // included file.
    std::fs::write(
        dir.join("part.md"),
        "## Part\n\nA {unknown}[x] here, see @fig:nowhere.\n",
    )
    .unwrap();
    std::fs::write(dir.join("tmark.toml"), "[press]\nschema = \"press.json\"\n").unwrap();
    std::fs::write(
        dir.join("press.json"),
        r#"{"properties": {"template": {"type": "string"}}}"#,
    )
    .unwrap();
    let main = dir.join("main.md");
    let text = "---\npress:\n  te\n---\n# Main\n\n{include}(part.md)\n";
    std::fs::write(&main, text).unwrap();
    let uri = format!("file://{}", main.display());
    let part_uri = format!("file://{}", dir.join("part.md").display());
    let (mut client, handle) = Client::start();
    client.open(&uri, "markdown", text);
    // Parse wave, then the analysis publishes the main file and the
    // included one (the half-typed front matter is not a concern here).
    let _ = client.diagnostics(&uri);
    let _ = client.diagnostics(&uri);
    let (version, part_diags) = client.diagnostics(&part_uri);
    assert_eq!(version, None);
    assert_eq!(part_diags[0]["code"], "role-unknown", "{part_diags:?}");
    assert_eq!(part_diags[0]["range"]["start"]["line"], 2);
    assert!(
        part_diags.iter().any(|d| d["code"] == "ref-unresolved"),
        "{part_diags:?}"
    );
    // The external press schema completes under `press:`.
    let items = client.request(
        "textDocument/completion",
        json!({"textDocument": {"uri": uri}, "position": {"line": 2, "character": 4}}),
    );
    assert!(
        items
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["label"] == "template"),
        "{items}"
    );
    // Closing clears the included file's diagnostics too.
    client.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": uri}}),
    );
    let (_, cleared) = client.diagnostics(&part_uri);
    assert!(cleared.is_empty());
    client.shutdown(handle);
    let _ = std::fs::remove_dir_all(&dir);
}
