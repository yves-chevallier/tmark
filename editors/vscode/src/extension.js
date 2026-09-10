// TMark for VS Code: starts `tmark-lsp` over stdio and wires it to the
// `tmark` and `markdown` languages. The server decides which Markdown files
// are TMark (ADR 0006); this client only finds the binary.
//
// Design: design/08-lsp.md §The VS Code extension.

"use strict";

const path = require("node:path");
const fs = require("node:fs");
const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

/** @type {LanguageClient | undefined} */
let client;

const BINARY = process.platform === "win32" ? "tmark-lsp.exe" : "tmark-lsp";

/**
 * Where the server binary is, in order: the `tmark.serverPath` setting, the
 * binary bundled under `bin/` of the extension, `tmark-lsp` on the PATH.
 * @param {vscode.ExtensionContext} context
 */
function serverCommand(context) {
  const configured = vscode.workspace.getConfiguration("tmark").get("serverPath");
  if (typeof configured === "string" && configured.trim() !== "") {
    return { command: configured.trim(), source: "tmark.serverPath" };
  }
  const bundled = context.asAbsolutePath(path.join("bin", BINARY));
  if (fs.existsSync(bundled)) {
    return { command: bundled, source: "bundled" };
  }
  return { command: BINARY, source: "PATH" };
}

/** @param {vscode.ExtensionContext} context */
async function start(context) {
  const { command, source } = serverCommand(context);
  const outputChannel = vscode.window.createOutputChannel("TMark");
  outputChannel.appendLine(`starting ${command} (${source})`);
  client = new LanguageClient(
    "tmark",
    "TMark Language Server",
    {
      run: { command, transport: TransportKind.stdio },
      debug: { command, transport: TransportKind.stdio, options: { env: { ...process.env, RUST_BACKTRACE: "1" } } },
    },
    {
      documentSelector: [
        { language: "tmark" },
        { language: "markdown", scheme: "file" },
        { language: "markdown", scheme: "untitled" },
      ],
      outputChannel,
      synchronize: {
        fileEvents: vscode.workspace.createFileSystemWatcher("**/{tmark.toml,*.bib}"),
      },
    },
  );
  try {
    await client.start();
  } catch (error) {
    client = undefined;
    const message = error instanceof Error ? error.message : String(error);
    const choice = await vscode.window.showErrorMessage(
      `TMark: cannot start the language server (${command}): ${message}`,
      "Open settings",
    );
    if (choice === "Open settings") {
      vscode.commands.executeCommand("workbench.action.openSettings", "tmark.serverPath");
    }
  }
}

async function stop() {
  if (client) {
    const running = client;
    client = undefined;
    await running.stop();
  }
}

/** @param {vscode.ExtensionContext} context */
async function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("tmark.restartServer", async () => {
      await stop();
      await start(context);
    }),
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration("tmark.serverPath")) {
        await stop();
        await start(context);
      }
    }),
  );
  await start(context);
}

function deactivate() {
  return stop();
}

module.exports = { activate, deactivate };
