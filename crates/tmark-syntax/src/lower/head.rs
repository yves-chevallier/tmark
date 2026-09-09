//! Small parsers for the heads the tokenizer keeps raw: attribute lists,
//! role heads, reference items, fence info strings, container and
//! admonition info. Spec §Lexical grammar.

use tmark_ir::{Attrs, RefItem};
use tmark_markdown::tmark::{is_ident_byte, is_ident_start};

/// Splits `s` into whitespace-separated tokens, keeping `"…"` values whole.
fn tokens(s: &str) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        let mut quoted = false;
        while i < bytes.len() && (quoted || !bytes[i].is_ascii_whitespace()) {
            if bytes[i] == b'"' {
                quoted = !quoted;
            }
            i += 1;
        }
        out.push(&s[start..i]);
    }
    out
}

fn is_ident(s: &str) -> bool {
    let bytes = s.as_bytes();
    !bytes.is_empty() && is_ident_start(bytes[0]) && bytes[1..].iter().all(|b| is_ident_byte(*b))
}

fn is_key(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(is_ident_byte)
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

/// `key=value` with a quoted or bare value.
fn key_value(token: &str) -> Option<(String, String)> {
    let (key, value) = token.split_once('=')?;
    if !is_key(key) || value.is_empty() {
        return None;
    }
    Some((key.to_string(), unquote(value)))
}

/// Parses the inside of an attribute list (`#id .class key=value`).
/// `None` when a token is not an attribute: the group is not an attribute
/// list (spec: "there are no bare-word attributes").
pub fn parse_attrs(s: &str) -> Option<Attrs> {
    let mut attrs = Attrs::new();
    let list = tokens(s);
    if list.is_empty() {
        return None;
    }
    for token in list {
        if let Some(id) = token.strip_prefix('#') {
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|b| is_ident_byte(b) || matches!(b, b':' | b'.'))
            {
                return None;
            }
            attrs.id = Some(id.to_string());
        } else if let Some(class) = token.strip_prefix('.') {
            if !is_key(class) {
                return None;
            }
            attrs.classes.push(class.to_string());
        } else if let Some((key, value)) = key_value(token) {
            attrs.kv.push((key, value));
        } else {
            return None;
        }
    }
    Some(attrs)
}

/// A parsed role head: `{name positional key=value}`.
#[derive(Debug, PartialEq)]
pub struct RoleHead {
    pub name: String,
    /// Deprecated `name:registry` suffix (spec Appendix "Deprecation schedule").
    pub registry_suffix: Option<String>,
    pub positional: Option<String>,
    pub kv: Vec<(String, String)>,
}

/// Parses the inside of a role head. `None` when it is not one.
pub fn parse_role_head(s: &str) -> Option<RoleHead> {
    let list = tokens(s);
    let first = *list.first()?;
    let (name, registry_suffix) = match first.split_once(':') {
        Some((name, suffix)) if is_ident(name) && is_key(suffix) => {
            (name.to_string(), Some(suffix.to_string()))
        }
        None if is_ident(first) => (first.to_string(), None),
        _ => return None,
    };
    let mut positional = None;
    let mut kv = Vec::new();
    for token in &list[1..] {
        if let Some(pair) = key_value(token) {
            kv.push(pair);
        } else if positional.is_none() && kv.is_empty() && !token.contains('=') {
            positional = Some(token.to_string());
        } else {
            return None;
        }
    }
    Some(RoleHead {
        name,
        registry_suffix,
        positional,
        kv,
    })
}

fn is_ref_key(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':' | b'.' | b'-' | b'/'))
}

/// Parses the items of a bracketed reference (`see ein05, pp. 33-35; -AI2027`):
/// Pandoc's item grammar, one item per `;`, each with an optional prefix, an
/// optional `-`, the key and an optional suffix after the first `,`.
pub fn parse_ref_items(inner: &str) -> Vec<RefItem> {
    inner
        .split(';')
        .map(|item| {
            let item = item.trim();
            let (head, suffix) = match item.split_once(',') {
                Some((head, suffix)) => (head.trim(), Some(suffix.trim().to_string())),
                None => (item, None),
            };
            let mut words: Vec<&str> = head.split_whitespace().collect();
            let mut suppress_author = false;
            let key = match words.pop() {
                Some(word) => {
                    let candidate = word.strip_prefix('-').unwrap_or(word);
                    if candidate != word {
                        suppress_author = true;
                    }
                    if is_ref_key(candidate) {
                        candidate.to_string()
                    } else {
                        // Not a key: the whole item is the key, as typed.
                        words.clear();
                        suppress_author = false;
                        item.to_string()
                    }
                }
                None => item.to_string(),
            };
            RefItem {
                prefix: if words.is_empty() {
                    None
                } else {
                    Some(words.join(" "))
                },
                suppress_author,
                key,
                suffix: suffix.filter(|s| !s.is_empty()),
            }
        })
        .collect()
}

/// A parsed fence info string (family 4).
#[derive(Debug, PartialEq)]
pub struct FenceInfo {
    pub lang: String,
    pub node: Option<String>,
    pub options: Vec<(String, String)>,
    /// Text of the info string that did not parse as options, kept for the
    /// listing.
    pub rest: Option<String>,
}

/// Parses `lang [node] [key=value…]`.
pub fn parse_fence_info(lang: Option<&str>, meta: Option<&str>) -> Option<FenceInfo> {
    let lang = lang?.trim();
    if lang.is_empty() {
        return None;
    }
    let mut info = FenceInfo {
        lang: lang.to_string(),
        node: None,
        options: Vec::new(),
        rest: None,
    };
    let Some(meta) = meta else {
        return Some(info);
    };
    let list = tokens(meta);
    let mut index = 0;
    if let Some(first) = list.first() {
        if tmark_ir::registry::node_word(first).is_some() {
            info.node = Some((*first).to_string());
            index = 1;
        }
    }
    let mut rest = Vec::new();
    for token in &list[index..] {
        match key_value(token) {
            Some(pair) => info.options.push(pair),
            None => rest.push(*token),
        }
    }
    if !rest.is_empty() {
        info.rest = Some(rest.join(" "));
    }
    Some(info)
}

/// Splits a container info string into its name and the raw attribute list.
pub fn parse_container_info(info: &str) -> (String, Option<Attrs>, bool) {
    let info = info.trim();
    let (name, rest) = match info.find(|c: char| c.is_ascii_whitespace()) {
        Some(at) => (&info[..at], info[at..].trim()),
        None => (info, ""),
    };
    let mut valid = true;
    let attrs = if rest.is_empty() {
        None
    } else if rest.starts_with('{') && rest.ends_with('}') {
        let parsed = parse_attrs(&rest[1..rest.len() - 1]);
        valid = parsed.is_some();
        parsed
    } else {
        valid = false;
        None
    };
    (name.to_string(), attrs, valid)
}

/// Splits `type class… "Title"` of a PyMdownX admonition.
pub fn parse_admonition_info(info: &str) -> (String, Vec<String>, Option<String>) {
    let info = info.trim();
    let (head, title) = match info.find('"') {
        Some(at) => (
            info[..at].trim(),
            Some(info[at..].trim().trim_matches('"').to_string()),
        ),
        None => (info, None),
    };
    let mut words = head.split_whitespace().map(str::to_string);
    let kind = words.next().unwrap_or_default();
    (kind, words.collect(), title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attrs() {
        let attrs = parse_attrs("#sec:intro .draft lang=en title=\"a b\"").unwrap();
        assert_eq!(attrs.id.as_deref(), Some("sec:intro"));
        assert_eq!(attrs.classes, vec!["draft"]);
        assert_eq!(attrs.get("lang"), Some("en"));
        assert_eq!(attrs.get("title"), Some("a b"));
        assert!(parse_attrs("collapsed").is_none(), "no bare words");
        assert!(parse_attrs("").is_none());
        assert!(parse_attrs("aside side=left").is_none());
    }

    #[test]
    fn role_heads() {
        let head = parse_role_head("aside side=left").unwrap();
        assert_eq!(head.name, "aside");
        assert_eq!(head.kv, vec![("side".to_string(), "left".to_string())]);
        let head = parse_role_head("raw latex").unwrap();
        assert_eq!(head.positional.as_deref(), Some("latex"));
        let head = parse_role_head("index:physics").unwrap();
        assert_eq!(head.registry_suffix.as_deref(), Some("physics"));
        assert!(parse_role_head("#id").is_none());
        assert!(parse_role_head("a b c").is_none(), "one positional at most");
    }

    #[test]
    fn ref_items() {
        let items = parse_ref_items("see ein05, pp. 33-35; -AI2027, ch. 1; fig:a");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].prefix.as_deref(), Some("see"));
        assert_eq!(items[0].key, "ein05");
        assert_eq!(items[0].suffix.as_deref(), Some("pp. 33-35"));
        assert!(items[1].suppress_author);
        assert_eq!(items[1].key, "AI2027");
        assert_eq!(items[2].key, "fig:a");
        assert!(items[2].prefix.is_none() && items[2].suffix.is_none());
    }

    #[test]
    fn fence_info() {
        let info = parse_fence_info(Some("python"), Some("image include=\"plot.py\"")).unwrap();
        assert_eq!(info.node.as_deref(), Some("image"));
        assert_eq!(
            info.options,
            vec![("include".to_string(), "plot.py".to_string())]
        );
        let info = parse_fence_info(Some("yaml"), Some("table")).unwrap();
        assert_eq!(info.node.as_deref(), Some("table"));
        let info = parse_fence_info(Some("js"), Some("title=\"a.js\" linenums=\"1\"")).unwrap();
        assert!(info.node.is_none());
        assert_eq!(info.options.len(), 2);
        assert!(parse_fence_info(None, None).is_none());
    }

    #[test]
    fn container_and_admonition_info() {
        let (name, attrs, valid) = parse_container_info("warning {title=\"LaTeX toolchain\"}");
        assert_eq!(name, "warning");
        assert_eq!(attrs.unwrap().get("title"), Some("LaTeX toolchain"));
        assert!(valid);
        let (kind, classes, title) = parse_admonition_info("note inline end \"Folded\"");
        assert_eq!(kind, "note");
        assert_eq!(classes, vec!["inline", "end"]);
        assert_eq!(title.as_deref(), Some("Folded"));
    }
}
