//! Navigation: go to definition, references, rename, hover, document
//! links. Design: `08-lsp.md` §Features ("`Ref` ↔ host with `#id`, counter
//! item, footnote"; "rename: labels and counter keys").
//!
//! Everything here reads the last analysis (`Resolved`) and the sub-spans
//! the parser records (`RefItem.key_span`, `Label.id_span`): a rename is a
//! set of text edits on those tokens, never a reprint of the host node.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::{
    DocumentLink, Hover, HoverContents, Location, MarkupContent, MarkupKind, PrepareRenameResponse,
    TextEdit, Uri, WorkspaceEdit,
};
use tmark::ir::{find, nodes_at, plain_text, Block, Document, FileId, Inline, LineIndex, Span};
use tmark::{Host, Label, RefResolution, Resolution, Resolved};

use crate::convert::{self, range};

/// What the language server knows about one open document, borrowed for
/// a request.
pub struct View<'a> {
    pub uri: &'a Uri,
    pub path: &'a Path,
    pub text: &'a str,
    pub index: &'a LineIndex,
    pub doc: &'a Document,
    pub resolved: Option<&'a Resolved>,
}

fn touches(span: Span, offset: u32) -> bool {
    span.start <= offset && offset <= span.end
}

/// The reference item under the cursor.
fn ref_at(resolved: &Resolved, offset: u32) -> Option<&RefResolution> {
    resolved
        .refs
        .iter()
        .filter(|r| r.span.file == FileId::default() && touches(r.span, offset))
        .min_by_key(|r| r.span.len())
}

/// The label whose id token is under the cursor.
fn label_at(resolved: &Resolved, offset: u32) -> Option<&Label> {
    resolved.labels.in_order.iter().find(|l| {
        l.id_span
            .is_some_and(|s| s.file == FileId::default() && touches(s, offset))
    })
}

/// The label a reference resolves to. Looked up by id: node ids are
/// per file, so `target` alone would not tell an included file's label
/// from the document's.
fn target_of<'a>(resolved: &'a Resolved, r: &RefResolution) -> Option<&'a Label> {
    match &r.resolution {
        Resolution::Label { target, .. } => resolved.labels.get(&r.key).or_else(|| {
            resolved
                .labels
                .in_order
                .iter()
                .find(|l| l.node == *target && l.span.file == r.span.file)
        }),
        _ => None,
    }
}

/// The URI of a file of the resolution: the document itself, or an
/// included file by its path.
fn file_uri(view: &View, resolved: &Resolved, file: FileId) -> Option<Uri> {
    if file == FileId::default() {
        return Some(view.uri.clone());
    }
    let (_, path) = resolved.files.iter().find(|(id, _)| *id == file)?;
    let absolute = if path.is_absolute() {
        path.clone()
    } else {
        view.path.parent().unwrap_or(Path::new("")).join(path)
    };
    convert::path_to_uri(&absolute)
}

/// A line index for a span's file: the document's own, or the included
/// file read from disk (the edge may read files).
fn index_for(view: &View, resolved: &Resolved, file: FileId) -> Option<(Uri, LineIndex)> {
    let uri = file_uri(view, resolved, file)?;
    if file == FileId::default() {
        return Some((uri, view.index.clone()));
    }
    let path = convert::uri_to_path(&uri)?;
    let text = std::fs::read_to_string(path).ok()?;
    Some((uri, LineIndex::new(&text)))
}

fn location(view: &View, resolved: &Resolved, span: Span) -> Option<Location> {
    let (uri, index) = index_for(view, resolved, span.file)?;
    Some(Location::new(uri, range(&index, span)))
}

// --- definition ---

pub fn definition(view: &View, offset: u32) -> Vec<Location> {
    let Some(resolved) = view.resolved else {
        return Vec::new();
    };
    if let Some(r) = ref_at(resolved, offset) {
        return match target_of(resolved, r) {
            Some(label) => location(view, resolved, label.id_span.unwrap_or(label.span))
                .into_iter()
                .collect(),
            None => Vec::new(),
        };
    }
    // `[^label]` → its definition.
    for node in nodes_at(view.doc, offset).into_iter().rev() {
        if let tmark::ir::NodeRef::Inline(Inline::Note(note)) = node {
            if let Some(label) = &note.label {
                return view
                    .doc
                    .footnotes
                    .iter()
                    .filter(|f| &f.label == label)
                    .map(|f| Location::new(view.uri.clone(), range(view.index, f.meta.span)))
                    .collect();
            }
        }
    }
    Vec::new()
}

// --- references ---

/// The key under the cursor, from a reference or from a definition.
fn key_at(resolved: &Resolved, offset: u32) -> Option<(&str, Option<&Label>)> {
    if let Some(r) = ref_at(resolved, offset) {
        return Some((r.key.as_str(), target_of(resolved, r)));
    }
    let label = label_at(resolved, offset)?;
    Some((label.id.as_str(), Some(label)))
}

pub fn references(view: &View, offset: u32, include_declaration: bool) -> Vec<Location> {
    let Some(resolved) = view.resolved else {
        return Vec::new();
    };
    let Some((key, label)) = key_at(resolved, offset) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if include_declaration {
        if let Some(label) = label {
            out.extend(location(
                view,
                resolved,
                label.id_span.unwrap_or(label.span),
            ));
        }
    }
    out.extend(
        resolved
            .refs
            .iter()
            .filter(|r| r.key.eq_ignore_ascii_case(key))
            .filter_map(|r| location(view, resolved, r.span)),
    );
    out
}

// --- rename ---

pub fn prepare_rename(view: &View, offset: u32) -> Option<PrepareRenameResponse> {
    let resolved = view.resolved?;
    let (key, label) = key_at(resolved, offset)?;
    // Only labels of this document rename; citations and inventories live
    // elsewhere.
    let label = label?;
    let span = if let Some(r) = ref_at(resolved, offset) {
        r.span
    } else {
        label.id_span?
    };
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range: range(view.index, span),
        placeholder: key.to_string(),
    })
}

/// Every occurrence of the label under the cursor becomes `new_name`. A
/// counter item's definition holds the key only, so its edit drops the
/// prefix; changing the prefix of a counter item is refused.
pub fn rename(view: &View, offset: u32, new_name: &str) -> Result<WorkspaceEdit, String> {
    let resolved = view
        .resolved
        .ok_or("the document is still being analysed")?;
    let (key, label) = key_at(resolved, offset).ok_or("nothing to rename here")?;
    let label = label.ok_or("only labels defined in this document can be renamed")?;
    let definition = label
        .id_span
        .ok_or("the definition's position is unknown")?;
    let definition_text = if label.host == Host::CounterItem {
        let prefix = label.prefix.as_deref().unwrap_or_default();
        let rest = new_name
            .strip_prefix(prefix)
            .and_then(|r| r.strip_prefix(':'))
            .ok_or_else(|| format!("a `{prefix}:` counter item keeps its prefix"))?;
        rest.to_string()
    } else {
        new_name.to_string()
    };
    // Keyed by the URI text (`Uri` has interior mutability); shaped for
    // the protocol at the end.
    let mut changes: HashMap<String, (Uri, Vec<TextEdit>)> = HashMap::new();
    let mut add = |span: Span, text: String| -> Result<(), String> {
        let (uri, index) = index_for(view, resolved, span.file)
            .ok_or_else(|| format!("cannot locate file {}", span.file.0))?;
        changes
            .entry(uri.as_str().to_string())
            .or_insert_with(|| (uri, Vec::new()))
            .1
            .push(TextEdit::new(range(&index, span), text));
        Ok(())
    };
    add(definition, definition_text)?;
    for r in resolved
        .refs
        .iter()
        .filter(|r| r.key.eq_ignore_ascii_case(key))
    {
        add(r.span, new_name.to_string())?;
    }
    Ok(WorkspaceEdit {
        changes: Some(changes.into_values().collect()),
        ..Default::default()
    })
}

// --- hover ---

fn host_word(host: Host) -> &'static str {
    match host {
        Host::Header => "section",
        Host::Table => "table",
        Host::Figure => "figure",
        Host::Listing => "listing",
        Host::Equation => "equation",
        Host::Admonition => "admonition",
        Host::CounterItem => "counter item",
        Host::Anchor => "anchor",
    }
}

/// The text a label's host shows: a heading or caption's content.
fn host_text(doc: &Document, resolved: &Resolved, label: &Label) -> Option<String> {
    let doc = if label.span.file == doc.file {
        doc
    } else {
        // An included file's node: in the included document.
        &resolved
            .included
            .iter()
            .find(|(id, _)| *id == label.span.file)?
            .1
    };
    match find(doc, label.node)? {
        tmark::ir::NodeRef::Block(Block::Header(h)) => Some(plain_text(&h.content)),
        tmark::ir::NodeRef::Block(Block::Caption(c)) => Some(plain_text(&c.content)),
        _ => None,
    }
}

fn markdown(value: String, span: Span, index: &LineIndex) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(range(index, span)),
    }
}

pub fn hover(view: &View, offset: u32) -> Option<Hover> {
    let resolved = view.resolved?;
    if let Some(r) = ref_at(resolved, offset) {
        let value = match &r.resolution {
            Resolution::Label { number, .. } => {
                let label = target_of(resolved, r)?;
                let mut s = format!("**{}** `#{}`", host_word(label.host), label.id);
                if let Some(n) = number {
                    s.push_str(&format!(" · {n}"));
                }
                if let Some(text) = host_text(view.doc, resolved, label).filter(|t| !t.is_empty()) {
                    s.push_str(&format!("\n\n{text}"));
                }
                s
            }
            Resolution::Citation { key } => {
                let entry = resolved.bibliography.get(key)?;
                let mut s = format!("**{}** `{key}`", entry.entry_type);
                for field in ["author", "title", "year"] {
                    if let Some(v) = entry.fields.get(field) {
                        s.push_str(&format!("\n\n{v}"));
                    }
                }
                s
            }
            Resolution::Glossary { term } => {
                let definition = resolved.glossary.get(term)?;
                format!("**{term}**\n\n{definition}")
            }
            Resolution::Doi { doi } => format!("DOI `{doi}`"),
            Resolution::External { alias, label, page } => match page {
                Some(p) => format!("`{alias}` → {label}, page {p}"),
                None => format!("`{alias}` → {label}"),
            },
            Resolution::Sibling { label, location } => format!("{label} → `{location}`"),
            Resolution::Ambiguous => format!("`{}` is in two registries", r.key),
            Resolution::Unresolved => format!("`{}` does not resolve", r.key),
        };
        return Some(markdown(value, r.span, view.index));
    }
    if let Some(label) = label_at(resolved, offset) {
        let mut s = format!("**{}** `#{}`", host_word(label.host), label.id);
        if let Some(n) = label.number {
            s.push_str(&format!(" · {n}"));
        }
        let uses = resolved
            .refs
            .iter()
            .filter(|r| r.key.eq_ignore_ascii_case(&label.id))
            .count();
        s.push_str(&format!(
            "\n\n{uses} reference{}",
            if uses == 1 { "" } else { "s" }
        ));
        return Some(markdown(s, label.id_span?, view.index));
    }
    // A role head: what the registry says.
    let text = view.text;
    for node in nodes_at(view.doc, offset).into_iter().rev() {
        let span = node.span();
        let src = &text[span.start as usize..span.end as usize];
        let Some(rest) = src.strip_prefix('{') else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let Some(role) = tmark::ir::registry::role(&name) else {
            continue;
        };
        let mut s = format!("**role** `{{{}}}` → `{}`", role.name, role.node);
        if !role.keys.is_empty() {
            s.push_str(&format!("\n\nkeys: {}", role.keys.join(", ")));
        }
        if let Some(canonical) = role.replaced_by {
            s.push_str(&format!("\n\ndeprecated, write `{{{canonical}}}`"));
        }
        return Some(markdown(s, span, view.index));
    }
    None
}

// --- document links ---

/// Includes and images with a relative path, as links to the files.
pub fn document_links(view: &View) -> Vec<DocumentLink> {
    let base: PathBuf = view.path.parent().unwrap_or(Path::new("")).to_path_buf();
    let mut out = Vec::new();
    let mut push = |span: Span, target: &str, tooltip: &str| {
        if target.is_empty() || target.contains("://") || target.starts_with('#') {
            return;
        }
        let path = base.join(target);
        if let Some(uri) = convert::path_to_uri(&path) {
            out.push(DocumentLink {
                range: range(view.index, span),
                target: Some(uri),
                tooltip: Some(tooltip.to_string()),
                data: None,
            });
        }
    };
    tmark::ir::walk(view.doc, &mut |node| match node {
        tmark::ir::NodeRef::Block(Block::Include(i)) => push(i.meta.span, &i.path, "include"),
        tmark::ir::NodeRef::Inline(Inline::Image(i)) => push(i.meta.span, &i.src, "image"),
        _ => {}
    });
    out
}
