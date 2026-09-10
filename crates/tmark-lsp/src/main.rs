//! `tmark-lsp`: language server over stdio (`lsp-server`, ADR 0007).
//!
//! Design: `design/08-lsp.md`. The server itself lives in the library so
//! that tests drive it over an in-memory connection.

use std::process::ExitCode;

fn main() -> ExitCode {
    let (connection, io_threads) = lsp_server::Connection::stdio();
    let result = tmark_lsp::run(connection);
    let joined = io_threads.join().map_err(|e| e.to_string());
    match result.map_err(|e| e.to_string()).and(joined) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tmark-lsp: {error}");
            ExitCode::from(1)
        }
    }
}
