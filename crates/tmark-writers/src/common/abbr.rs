//! Acronym substitution in running text: the `*[HTML]: …` definitions and
//! `press.declare.acronyms` keys found as whole words in a `Str` render as
//! acronyms (`\tsacr`, `<abbr>`), as the legacy `abbr` extension did.
//! The parser does not emit `Abbr` nodes yet (open question (c) of
//! writers-and-passes.md §6); when it does, `split` finds nothing and this
//! module becomes a no-op.

use tmark_ir::Document;

/// A piece of a `Str`: plain text or an acronym key.
#[derive(Debug, PartialEq, Eq)]
pub enum Segment<'a> {
    Text(&'a str),
    Abbr(&'a str),
}

/// The acronym keys of a document, longest first so the longest match
/// wins, without duplicates.
pub fn keys(doc: &Document) -> Vec<String> {
    let mut keys: Vec<String> = doc.abbreviations.iter().map(|a| a.key.clone()).collect();
    if let serde_json::Value::Object(map) = &doc.front_matter.keys.press.declare.acronyms {
        keys.extend(map.keys().cloned());
    }
    keys.retain(|k| !k.trim().is_empty());
    keys.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    keys.dedup();
    keys
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric()
}

/// Splits `text` at whole-word occurrences of `keys` (case-sensitive).
pub fn split<'a>(text: &'a str, keys: &'a [String]) -> Vec<Segment<'a>> {
    if keys.is_empty() {
        return vec![Segment::Text(text)];
    }
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < text.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let before_ok = i == 0 || !text[..i].chars().next_back().is_some_and(is_word);
        let found = before_ok
            .then(|| {
                keys.iter().find(|k| {
                    text[i..].starts_with(k.as_str())
                        && !text[i + k.len()..].chars().next().is_some_and(is_word)
                })
            })
            .flatten();
        match found {
            Some(key) => {
                if start < i {
                    out.push(Segment::Text(&text[start..i]));
                }
                out.push(Segment::Abbr(&text[i..i + key.len()]));
                i += key.len();
                start = i;
            }
            None => i += text[i..].chars().next().map_or(1, char::len_utf8),
        }
    }
    if start < text.len() {
        out.push(Segment::Text(&text[start..]));
    }
    if out.is_empty() {
        out.push(Segment::Text(text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_whole_words() {
        let keys = vec!["HTML".to_string(), "GC-MS".to_string()];
        assert_eq!(
            split("The HTML spec and GC-MS, not HTMLX or xHTML.", &keys),
            vec![
                Segment::Text("The "),
                Segment::Abbr("HTML"),
                Segment::Text(" spec and "),
                Segment::Abbr("GC-MS"),
                Segment::Text(", not HTMLX or xHTML."),
            ]
        );
        assert_eq!(split("HTML", &keys), vec![Segment::Abbr("HTML")]);
        assert_eq!(split("plain", &keys), vec![Segment::Text("plain")]);
        assert_eq!(
            split("é HTML é", &keys),
            vec![
                Segment::Text("é "),
                Segment::Abbr("HTML"),
                Segment::Text(" é")
            ]
        );
    }
}
