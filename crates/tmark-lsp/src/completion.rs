//! Completion. Design: `08-lsp.md` §Features: `@` → labels, bibliography
//! keys, glossary terms, inventories; `{` → role names, then the role's
//! keys; `:::` → container and admonition names; a fence info string →
//! node words; the front matter → keys from the JSON schema.
//!
//! The context comes from the text around the cursor, not from the IR: a
//! key being typed is not a node yet. Replacement ranges cover the partial
//! word already typed.

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, MarkupContent,
    MarkupKind, Range, TextEdit,
};
use std::path::Path;
use tmark::ir::registry::{self, ADMONITIONS, NODE_WORDS, ROLES};
use tmark::ir::{Document, LineIndex};

use tmark::{Host, Resolved};

use crate::convert;

/// Characters the client should send a request on.
pub const TRIGGERS: &[&str] = &["@", "{", ":", ";", "[", " ", ".", "(", "/"];

/// What completion needs beyond the document: the edge supplies it.
pub struct Extra<'a> {
    /// TeXSmith's `press` JSON schema (`tmark.toml` `[press] schema`),
    /// merged under `press:` in the front matter.
    pub press_schema: Option<&'a serde_json::Value>,
    /// The document's directory, for image and include paths.
    pub dir: &'a Path,
    /// Entries of a directory, directories with a trailing `/`.
    pub list_dir: &'a dyn Fn(&Path) -> Vec<String>,
}

fn is_key_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Items for the cursor at byte `offset`, or `None` when nothing applies.
pub fn complete(
    text: &str,
    index: &LineIndex,
    doc: &Document,
    resolved: Option<&Resolved>,
    offset: u32,
    extra: &Extra,
) -> Option<Vec<CompletionItem>> {
    let offset = (offset as usize).min(text.len());
    let line_start = index.line_start(index.line_col(offset as u32).line)? as usize;
    let line = &text[line_start..offset];

    let fm = doc.front_matter.meta.span;
    if !doc.front_matter.raw.is_empty() && fm.contains(offset as u32) {
        return Some(front_matter(
            text,
            index,
            line_start,
            line,
            offset,
            extra.press_schema,
        ));
    }
    if let Some(items) = moustache(index, line, line_start, offset, doc) {
        return Some(items);
    }
    if let Some(items) = paths(index, line, line_start, offset, extra) {
        return Some(items);
    }
    if let Some(items) = reference(index, line, line_start, offset, resolved) {
        return Some(items);
    }
    if let Some(items) = classes(index, line, line_start, offset, text) {
        return Some(items);
    }
    if let Some(items) = role(index, line, line_start, offset) {
        return Some(items);
    }
    if let Some(items) = container(index, line, line_start, offset) {
        return Some(items);
    }
    fence(index, line, line_start, offset)
}

fn edit_range(index: &LineIndex, start: usize, end: usize) -> Range {
    Range::new(
        convert::position(index, start as u32),
        convert::position(index, end as u32),
    )
}

fn item(label: &str, kind: CompletionItemKind, range: Range, new_text: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
            range,
            new_text.to_string(),
        ))),
        ..Default::default()
    }
}

fn markdown(text: String) -> Option<Documentation> {
    (!text.is_empty()).then_some(Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: text,
    }))
}

/// The partial word before the cursor: its start offset within `line`.
fn word_start(line: &str, is_char: fn(char) -> bool) -> usize {
    line.char_indices()
        .rev()
        .take_while(|(_, c)| is_char(*c))
        .last()
        .map_or(line.len(), |(i, _)| i)
}

// --- `@` ---

/// `@key`, `@[key`, `@[a; key`, `@[see key`: the key being typed.
fn reference(
    index: &LineIndex,
    line: &str,
    line_start: usize,
    offset: usize,
    resolved: Option<&Resolved>,
) -> Option<Vec<CompletionItem>> {
    let start = word_start(line, is_key_char);
    let before = &line[..start];
    let bare = before.ends_with('@');
    let bracketed = !bare && {
        // An unclosed `@[` earlier on the line.
        match before.rfind("@[") {
            Some(at) => !before[at..].contains(']'),
            None => false,
        }
    };
    if !bare && !bracketed {
        return None;
    }
    if bracketed {
        // After `@[` the key comes first, or after `;`, or after a prefix
        // word (`see`); a suffix (`p. 33`) is not a key.
        let items_before = before.rsplit_once("@[").map_or("", |(_, rest)| rest);
        let current = items_before.rsplit(';').next().unwrap_or("");
        if current.contains(',') {
            return None;
        }
    }
    let range = edit_range(index, line_start + start, offset);
    let mut out = Vec::new();
    let Some(resolved) = resolved else {
        return Some(out);
    };
    for label in &resolved.labels.in_order {
        let kind = match label.host {
            Host::Header => CompletionItemKind::MODULE,
            Host::CounterItem => CompletionItemKind::ENUM_MEMBER,
            Host::Anchor => CompletionItemKind::REFERENCE,
            _ => CompletionItemKind::VALUE,
        };
        let mut it = item(&label.id, kind, range, &label.id);
        it.detail = Some(match (&label.number, &label.prefix) {
            (Some(n), Some(p)) => format!("{p} {n}"),
            (None, Some(p)) => format!("{p} ({})", host_name(label.host)),
            _ => host_name(label.host).to_string(),
        });
        out.push(it);
    }
    for (key, entry) in &resolved.bibliography.entries {
        let mut it = item(key, CompletionItemKind::CONSTANT, range, key);
        it.detail = Some(entry.entry_type.clone());
        let mut doc = String::new();
        for field in ["author", "title", "year"] {
            if let Some(value) = entry.fields.get(field) {
                doc.push_str(value);
                doc.push_str("  \n");
            }
        }
        it.documentation = markdown(doc);
        out.push(it);
    }
    for (term, definition) in &resolved.glossary {
        let key = format!("gls:{term}");
        let mut it = item(&key, CompletionItemKind::KEYWORD, range, &key);
        it.detail = Some("glossary".into());
        it.documentation = markdown(definition.clone());
        out.push(it);
    }
    for (alias, inventory) in &resolved.crossrefs.by_alias {
        for (key, entry) in &inventory.refs {
            let full = format!("{alias}:{key}");
            let mut it = item(&full, CompletionItemKind::INTERFACE, range, &full);
            it.detail = Some(entry.label.clone());
            out.push(it);
        }
    }
    Some(out)
}

fn host_name(host: Host) -> &'static str {
    match host {
        Host::Header => "section",
        Host::Table => "table",
        Host::Figure => "figure",
        Host::Subfigure => "subfigure",
        Host::Listing => "listing",
        Host::Equation => "equation",
        Host::Admonition => "admonition",
        Host::CounterItem => "counter item",
        Host::Anchor => "anchor",
    }
}

// --- `{` ---

/// `{rol` → role names; `{code ` or `{code lang=py ` → the role's keys.
fn role(
    index: &LineIndex,
    line: &str,
    line_start: usize,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let start = word_start(line, is_word_char);
    let before = &line[..start];
    if before.ends_with('{') && !before.ends_with("{{") {
        let range = edit_range(index, line_start + start, offset);
        let items = ROLES
            .iter()
            .filter(|r| r.replaced_by.is_none())
            .map(|r| {
                let mut it = item(r.name, CompletionItemKind::FUNCTION, range, r.name);
                it.detail = Some(r.node.to_string());
                let keys = if r.keys.is_empty() {
                    String::new()
                } else {
                    format!("keys: {}", r.keys.join(", "))
                };
                it.documentation = markdown(keys);
                it
            })
            .collect();
        return Some(items);
    }
    // Inside a role head: `{name …` with no closing brace yet.
    let open = before.rfind('{')?;
    let head = &before[open + 1..];
    if head.contains('}') || head.is_empty() || !head.ends_with(' ') {
        return None;
    }
    let name = head.split_whitespace().next()?;
    let role = registry::role(name)?;
    let range = edit_range(index, line_start + start, offset);
    let items = role
        .keys
        .iter()
        .map(|key| {
            let mut it = item(key, CompletionItemKind::PROPERTY, range, &format!("{key}="));
            it.detail = Some(format!("{name} key"));
            it
        })
        .collect();
    Some(items)
}

// --- `{.` ---

/// `{.cla` inside an attribute list: the classes the document already
/// uses.
fn classes(
    index: &LineIndex,
    line: &str,
    line_start: usize,
    offset: usize,
    text: &str,
) -> Option<Vec<CompletionItem>> {
    let start = word_start(line, |c| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '-')
    });
    let before = &line[..start];
    if !before.ends_with('.') {
        return None;
    }
    let open = before.rfind('{')?;
    if before[open..].contains('}') || before[..open].ends_with('{') {
        return None;
    }
    let mut seen: Vec<String> = Vec::new();
    // Every `.class` token inside a brace group of the text.
    for group in text.split('{').skip(1) {
        // Closed groups only: the one being typed is not a source.
        let Some((inner, _)) = group.split_once('}') else {
            continue;
        };
        for token in inner.split_whitespace() {
            if let Some(class) = token.strip_prefix('.') {
                if !class.is_empty()
                    && class
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                    && !seen.iter().any(|s| s == class)
                {
                    seen.push(class.to_string());
                }
            }
        }
    }
    seen.sort();
    let range = edit_range(index, line_start + start, offset);
    Some(
        seen.iter()
            .map(|class| item(class, CompletionItemKind::ENUM_MEMBER, range, class))
            .collect(),
    )
}

// --- `{{ path }}` ---

/// `{{ pre` → dotted paths into the front matter (spec §Var).
fn moustache(
    index: &LineIndex,
    line: &str,
    line_start: usize,
    offset: usize,
    doc: &Document,
) -> Option<Vec<CompletionItem>> {
    let start = word_start(line, |c| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
    });
    let before = line[..start].trim_end();
    if !before.ends_with("{{") {
        return None;
    }
    let mut value = serde_json::to_value(&doc.front_matter.keys).ok()?;
    if let (serde_json::Value::Object(map), serde_json::Value::Object(extra)) =
        (&mut value, &doc.front_matter.extra)
    {
        for (k, v) in extra {
            map.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    let mut out = Vec::new();
    fn walk(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, String)>) {
        if let serde_json::Value::Object(map) = value {
            for (k, v) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                let kind = match v {
                    serde_json::Value::Object(_) => "mapping",
                    serde_json::Value::Array(_) => "list",
                    serde_json::Value::Null => "null",
                    _ => "value",
                };
                out.push((path.clone(), kind.to_string()));
                walk(&path, v, out);
            }
        }
    }
    walk("", &value, &mut out);
    let range = edit_range(index, line_start + start, offset);
    Some(
        out.into_iter()
            .map(|(path, kind)| {
                let mut it = item(&path, CompletionItemKind::VARIABLE, range, &path);
                it.detail = Some(kind);
                it
            })
            .collect(),
    )
}

// --- paths ---

/// `![alt](img/pa` and `{include}(chap/pa`: files next to the document.
fn paths(
    index: &LineIndex,
    line: &str,
    line_start: usize,
    offset: usize,
    extra: &Extra,
) -> Option<Vec<CompletionItem>> {
    let open = line.rfind('(')?;
    let partial = &line[open + 1..];
    if partial.contains(')') || partial.contains(' ') {
        return None;
    }
    let head = &line[..open];
    let image = head.ends_with(']') && head.rfind("![").is_some_and(|i| !head[i..].contains(')'));
    let include = head.ends_with("{include}");
    if !image && !include {
        return None;
    }
    let (subdir, segment) = match partial.rfind('/') {
        Some(slash) => (&partial[..=slash], &partial[slash + 1..]),
        None => ("", partial),
    };
    let dir = extra.dir.join(subdir);
    let range = edit_range(index, line_start + open + 1 + subdir.len(), offset);
    let _ = segment;
    let items = (extra.list_dir)(&dir)
        .into_iter()
        .filter(|name| {
            name.ends_with('/')
                || !include
                || name.ends_with(".md")
                || name.ends_with(".tmd")
                || name.ends_with(".tm")
        })
        .map(|name| {
            let kind = if name.ends_with('/') {
                CompletionItemKind::FOLDER
            } else {
                CompletionItemKind::FILE
            };
            item(&name, kind, range, &name)
        })
        .collect();
    Some(items)
}

// --- `:::`, `!!!`, `???` ---

fn container(
    index: &LineIndex,
    line: &str,
    line_start: usize,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let start = word_start(line, is_word_char);
    let before = line[..start].trim_start();
    let fence = before.trim_end();
    let directive = if fence.chars().all(|c| c == ':') && fence.len() >= 3 {
        Some("container")
    } else if matches!(fence, "!!!" | "???" | "???+") {
        Some("admonition")
    } else {
        None
    }?;
    let range = edit_range(index, line_start + start, offset);
    let mut items: Vec<CompletionItem> = ADMONITIONS
        .iter()
        .map(|a| {
            let mut it = item(a.name, CompletionItemKind::CLASS, range, a.name);
            it.detail = Some(match a.counter {
                Some(counter) => format!("{} (counter {counter})", a.label),
                None => a.label.to_string(),
            });
            it
        })
        .collect();
    if directive == "container" {
        for (name, detail) in [("figure", "Figure"), ("aside", "Aside")] {
            let mut it = item(name, CompletionItemKind::STRUCT, range, name);
            it.detail = Some(detail.to_string());
            items.insert(0, it);
        }
    }
    Some(items)
}

// --- fences ---

/// ```` ```yaml ta ```` → node words.
fn fence(
    index: &LineIndex,
    line: &str,
    line_start: usize,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let start = word_start(line, is_word_char);
    let before = &line[..start];
    let trimmed = before.trim_start();
    let fence_len = trimmed
        .chars()
        .take_while(|c| *c == '`' || *c == '~')
        .count();
    if fence_len < 3 {
        return None;
    }
    let info = &trimmed[fence_len..];
    let mut words = info.split_whitespace();
    let lang = words.next()?;
    if words.next().is_some() || !info.ends_with(' ') {
        return None;
    }
    let range = edit_range(index, line_start + start, offset);
    let items = NODE_WORDS
        .iter()
        .map(|w| {
            let mut it = item(w.word, CompletionItemKind::KEYWORD, range, w.word);
            it.detail = Some(format!("{lang} {} → {}", w.word, w.node));
            it
        })
        .collect();
    Some(items)
}

// --- front matter ---

/// Keys of the front-matter schema at the current nesting: top-level keys
/// on an unindented line, `press` keys under `press:`.
fn front_matter(
    text: &str,
    index: &LineIndex,
    line_start: usize,
    line: &str,
    offset: usize,
    press_schema: Option<&serde_json::Value>,
) -> Vec<CompletionItem> {
    let indent = line.len() - line.trim_start().len();
    let typed = &line[indent..];
    if typed.contains(':') {
        return Vec::new();
    }
    let Some(schema) = tmark::schema("frontmatter") else {
        return Vec::new();
    };
    // The parent key: the nearest line above with a smaller indent.
    let mut parent = None;
    if indent > 0 {
        for prev in text[..line_start].lines().rev() {
            let prev_indent = prev.len() - prev.trim_start().len();
            if prev.trim().is_empty() {
                continue;
            }
            if prev_indent < indent {
                parent = prev.trim().strip_suffix(':').map(str::to_string);
                break;
            }
        }
    }
    let node = match parent.as_deref() {
        None => schema.clone(),
        Some(key) => match schema["properties"]
            .get(key)
            .and_then(|p| deref(&schema, p))
        {
            Some(node) => node,
            None => return Vec::new(),
        },
    };
    let mut props: Vec<(String, serde_json::Value)> = node
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|p| p.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    // TeXSmith's own `press` keys, merged after TMark's.
    if parent.as_deref() == Some("press") {
        if let Some(external) = press_schema
            .and_then(|s| s.get("properties"))
            .and_then(|p| p.as_object())
        {
            for (k, v) in external {
                if !props.iter().any(|(existing, _)| existing == k) {
                    props.push((k.clone(), v.clone()));
                }
            }
        }
    }
    let range = edit_range(index, line_start + indent, offset);
    props
        .iter()
        .map(|(key, prop)| {
            let mut it = item(key, CompletionItemKind::FIELD, range, &format!("{key}: "));
            it.detail = prop
                .get("type")
                .map(|t| match t {
                    serde_json::Value::Array(ts) => ts
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(" | "),
                    other => other.as_str().unwrap_or_default().to_string(),
                })
                .filter(|s| !s.is_empty());
            it.documentation = prop
                .get("description")
                .and_then(|d| d.as_str())
                .and_then(|d| markdown(d.to_string()));
            it
        })
        .collect()
}

/// Follows a local `$ref` into `definitions`.
fn deref(schema: &serde_json::Value, node: &serde_json::Value) -> Option<serde_json::Value> {
    match node.get("$ref").and_then(|r| r.as_str()) {
        Some(reference) => {
            let name = reference.strip_prefix("#/definitions/")?;
            schema["definitions"].get(name).cloned()
        }
        None => Some(node.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmark::{FileId, MemoryLoader, ResolveOptions};

    fn setup(text: &str) -> (Document, Resolved, LineIndex) {
        let parsed = tmark::parse(text, FileId::default());
        let resolved = tmark::resolve(
            &parsed.document,
            &MemoryLoader::new(),
            &ResolveOptions::default(),
        );
        (parsed.document, resolved, LineIndex::new(text))
    }

    fn labels(items: &[CompletionItem]) -> Vec<&str> {
        items.iter().map(|i| i.label.as_str()).collect()
    }

    fn none(_: &Path) -> Vec<String> {
        Vec::new()
    }

    fn fake(_: &Path) -> Vec<String> {
        vec!["img/".into(), "plot.png".into(), "ch2.md".into()]
    }

    fn extra<'a>(list_dir: &'a dyn Fn(&Path) -> Vec<String>) -> Extra<'a> {
        Extra {
            press_schema: None,
            dir: Path::new("/docs"),
            list_dir,
        }
    }

    #[test]
    fn references_after_at() {
        let text = "# Intro {#sec:intro}\n\nA #(fw:boot) item. See @sec:in\n";
        let (doc, resolved, index) = setup(text);
        let at = text.len() as u32 - 1;
        let items = complete(text, &index, &doc, Some(&resolved), at, &extra(&none)).unwrap();
        assert!(
            labels(&items).contains(&"sec:intro"),
            "{:?}",
            labels(&items)
        );
        assert!(labels(&items).contains(&"fw:boot"), "{:?}", labels(&items));
        let edit = match items[0].text_edit.as_ref().unwrap() {
            CompletionTextEdit::Edit(e) => e,
            _ => unreachable!(),
        };
        assert_eq!(edit.range.start.character, 24);
        assert_eq!(edit.range.end.character, 30);
        // Bracketed, second item.
        let text2 = "# Intro {#sec:intro}\n\nSee @[sec:intro; s";
        let (doc, resolved, index) = setup(text2);
        let items = complete(
            text2,
            &index,
            &doc,
            Some(&resolved),
            text2.len() as u32,
            &extra(&none),
        )
        .unwrap();
        assert!(labels(&items).contains(&"sec:intro"));
        // Not after a closed list, nor in prose.
        let text3 = "See @[sec:intro] and s";
        let (doc, resolved, index) = setup(text3);
        assert!(complete(
            text3,
            &index,
            &doc,
            Some(&resolved),
            text3.len() as u32,
            &extra(&none)
        )
        .is_none());
    }

    #[test]
    fn roles_containers_fences() {
        let text = "Text {co";
        let (doc, resolved, index) = setup(text);
        let items = complete(
            text,
            &index,
            &doc,
            Some(&resolved),
            text.len() as u32,
            &extra(&none),
        )
        .unwrap();
        assert!(labels(&items).contains(&"code"));
        assert!(
            !labels(&items).contains(&"latex"),
            "deprecated roles are not offered"
        );
        let text = "Text {code l";
        let (doc, resolved, index) = setup(text);
        let items = complete(
            text,
            &index,
            &doc,
            Some(&resolved),
            text.len() as u32,
            &extra(&none),
        )
        .unwrap();
        assert!(labels(&items).contains(&"lang"), "{:?}", labels(&items));
        let text = "::: no";
        let (doc, resolved, index) = setup(text);
        let items = complete(
            text,
            &index,
            &doc,
            Some(&resolved),
            text.len() as u32,
            &extra(&none),
        )
        .unwrap();
        assert_eq!(labels(&items)[..2], ["aside", "figure"]);
        assert!(labels(&items).contains(&"note"));
        let text = "```yaml ta";
        let (doc, resolved, index) = setup(text);
        let items = complete(
            text,
            &index,
            &doc,
            Some(&resolved),
            text.len() as u32,
            &extra(&none),
        )
        .unwrap();
        assert!(labels(&items).contains(&"table"));
        let text = "{{ pa";
        let (doc, resolved, index) = setup(text);
        let items = complete(
            text,
            &index,
            &doc,
            Some(&resolved),
            text.len() as u32,
            &extra(&none),
        );
        assert!(
            items.is_some_and(|i| i.is_empty()),
            "moustache paths, none here"
        );
    }

    #[test]
    fn classes_paths_and_moustaches() {
        let text =
            "---\ntitle: T\npress:\n  template: article\n---\nA [x]{.draft .wide} and ![alt](pl";
        let (doc, resolved, index) = setup(text);
        let items = complete(
            text,
            &index,
            &doc,
            Some(&resolved),
            text.len() as u32,
            &extra(&fake),
        )
        .unwrap();
        assert_eq!(labels(&items), ["img/", "plot.png", "ch2.md"]);
        let text2 = "A [x]{.draft .wide} and [y]{.w";
        let (doc, resolved, index) = setup(text2);
        let items = complete(
            text2,
            &index,
            &doc,
            Some(&resolved),
            text2.len() as u32,
            &extra(&none),
        )
        .unwrap();
        assert_eq!(labels(&items), ["draft", "wide"]);
        let text3 = "{include}(ch";
        let (doc, resolved, index) = setup(text3);
        let items = complete(
            text3,
            &index,
            &doc,
            Some(&resolved),
            text3.len() as u32,
            &extra(&fake),
        )
        .unwrap();
        assert_eq!(
            labels(&items),
            ["img/", "ch2.md"],
            "only Markdown files for includes"
        );
        let text4 = "---\ntitle: T\npress:\n  template: article\n---\nSee {{ pr";
        let (doc, resolved, index) = setup(text4);
        let items = complete(
            text4,
            &index,
            &doc,
            Some(&resolved),
            text4.len() as u32,
            &extra(&none),
        )
        .unwrap();
        assert!(
            labels(&items).contains(&"press.template"),
            "{:?}",
            labels(&items)
        );
        assert!(labels(&items).contains(&"title"));
    }

    #[test]
    fn press_schema_merges_under_press() {
        let text = "---\npress:\n  te\n---\n";
        let (doc, resolved, index) = setup(text);
        let schema = serde_json::json!({"properties": {"template": {"type": "string", "description": "TeXSmith template"}, "declare": {}}});
        let extra = Extra {
            press_schema: Some(&schema),
            dir: Path::new("/docs"),
            list_dir: &none,
        };
        let at = text.find("  te\n").unwrap() + 4;
        let items = complete(text, &index, &doc, Some(&resolved), at as u32, &extra).unwrap();
        assert!(labels(&items).contains(&"template"), "{:?}", labels(&items));
        assert_eq!(
            labels(&items).iter().filter(|l| **l == "declare").count(),
            1
        );
    }

    #[test]
    fn front_matter_keys() {
        let text = "---\ntitle: X\nti\npress:\n  ba\n---\n# H\n";
        let (doc, resolved, index) = setup(text);
        let top = text.find("\nti\n").unwrap() + 3;
        let items = complete(
            text,
            &index,
            &doc,
            Some(&resolved),
            top as u32,
            &extra(&none),
        )
        .unwrap();
        assert!(labels(&items).contains(&"title"), "{:?}", labels(&items));
        assert!(labels(&items).contains(&"press"));
        let under = text.find("  ba\n").unwrap() + 4;
        let items = complete(
            text,
            &index,
            &doc,
            Some(&resolved),
            under as u32,
            &extra(&none),
        )
        .unwrap();
        assert!(
            labels(&items).contains(&"base_level"),
            "{:?}",
            labels(&items)
        );
        assert!(!labels(&items).contains(&"title"));
    }
}
