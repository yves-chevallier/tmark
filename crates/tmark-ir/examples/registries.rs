//! Writes the registry tables the editor grammar needs to
//! `editors/vscode/scripts/registries.json`: role names with their
//! canonical replacement, node words, counter prefixes, admonition names,
//! fragment contracts and keystroke labels.
//! Run with `cargo run -p tmark-ir --example registries` and commit the
//! output, then regenerate the grammar (`npm run build:grammar`). One
//! definition of the names, in `tmark_ir::registry` (AGENTS.md, SSOT).

use std::path::Path;

use serde_json::json;
use tmark_ir::registry::{ADMONITIONS, FRAGMENTS, KEY_LABELS, NODE_WORDS, PREFIXES, ROLES};

fn main() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../editors/vscode/scripts/registries.json");
    let value = json!({
        "generated_by": "cargo run -p tmark-ir --example registries; do not edit",
        "roles": ROLES.iter().map(|r| json!({
            "name": r.name,
            "node": r.node,
            "replaced_by": r.replaced_by,
        })).collect::<Vec<_>>(),
        "node_words": NODE_WORDS.iter().map(|w| w.word).collect::<Vec<_>>(),
        "prefixes": PREFIXES.iter().map(|p| p.name).collect::<Vec<_>>(),
        "admonitions": ADMONITIONS.iter().map(|a| a.name).collect::<Vec<_>>(),
        "fragments": FRAGMENTS.iter().map(|f| json!({
            "name": f.name,
            "provides": f.provides,
            "packages": f.packages,
            "shell_escape": f.shell_escape,
            "description": f.description,
        })).collect::<Vec<_>>(),
        "key_labels": KEY_LABELS.iter().map(|k| json!({
            "name": k.name,
            "label": k.label,
        })).collect::<Vec<_>>(),
    });
    let json = serde_json::to_string_pretty(&value).expect("serialises");
    std::fs::write(&path, json + "\n").unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    println!("wrote {}", path.display());
}
