//! Reference text from a `Resolved` (design 07 §Mapping rules,
//! "References"): the counter's `ref` template, `[?key]` for an unresolved
//! key, the textual templates of `WriterOptions.refs`.

use tmark_ir::NodeId;
use tmark_registry::{RefResolution, Resolved};

/// The resolution recorded for `key` on `node`, if any.
pub fn lookup<'a>(res: &'a Resolved, node: NodeId, key: &str) -> Option<&'a RefResolution> {
    res.refs.iter().find(|r| r.node == node && r.key == key)
}

/// What an unresolved key renders as, in every backend (spec §Ref).
pub fn unresolved(key: &str) -> String {
    format!("[?{key}]")
}

/// Renders `{name}` and `{number}` in a counter's `ref` template.
pub fn template(template: &str, name: &str, number: &str) -> String {
    template.replace("{name}", name).replace("{number}", number)
}

/// Renders a textual-reference template: `{text}`, `{number}`, `{page}`.
pub fn textual(template: &str, text: &str, number: &str, page: &str) -> String {
    template
        .replace("{text}", text)
        .replace("{number}", number)
        .replace("{page}", page)
}

/// The label word of a series as the reference wants it: a capitalised
/// prefix (`@Fig:x`) capitalises the word (spec §Ref, pandoc-crossref).
pub fn label_word(res: &Resolved, prefix: &str, key_as_written: &str) -> String {
    let name = res
        .counters
        .get(prefix)
        .and_then(|c| c.name.clone())
        .unwrap_or_default();
    if key_as_written
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
    {
        let mut chars = name.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => name,
        }
    } else {
        name
    }
}

/// The `ref` template of a series (`{name} {number}` by default).
pub fn reference_template(res: &Resolved, prefix: &str) -> String {
    res.counters
        .get(prefix)
        .map(|c| c.reference.clone())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "{name} {number}".to_string())
}

/// The formatted number of `key` in the series `prefix`, when TMark
/// numbers it.
pub fn number(res: &Resolved, prefix: &str, key: &str) -> Option<String> {
    res.counters.get(prefix).and_then(|c| c.label(key))
}

/// The minimal built-in citation style (design 07 §Mapping rules):
/// `Author Year`, the year alone for `-@key`, the key when the
/// bibliography has no record.
pub fn author_year(res: &Resolved, key: &str, suppress_author: bool) -> String {
    let Some(entry) = res.bibliography.get(key) else {
        return key.to_string();
    };
    let year = entry
        .fields
        .get("year")
        .or_else(|| entry.fields.get("date"))
        .map(|d| d.chars().take(4).collect::<String>());
    let author = entry.fields.get("author").map(|a| {
        let first = a.split(" and ").next().unwrap_or(a);
        match first.split_once(',') {
            Some((last, _)) => last.trim().to_string(),
            None => first.rsplit(' ').next().unwrap_or(first).to_string(),
        }
    });
    match (author, year, suppress_author) {
        (_, Some(year), true) => year,
        (Some(author), Some(year), false) => format!("{author} {year}"),
        (Some(author), None, false) => author,
        (None, Some(year), _) => year,
        _ => key.to_string(),
    }
}

/// The series numbers labels itself (declared counters); `false` for the
/// predeclared backend-numbered ones.
pub fn tmark_numbered(res: &Resolved, prefix: &str) -> bool {
    res.counters.get(prefix).is_some_and(|c| c.tmark_numbered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates() {
        assert_eq!(template("{name} {number}", "Figure", "3"), "Figure 3");
        assert_eq!(template("{number}", "Finding", "FW-01"), "FW-01");
        assert_eq!(
            textual("{text} (p. {page})", "the trace", "3", "12"),
            "the trace (p. 12)"
        );
        assert_eq!(unresolved("sec:nope"), "[?sec:nope]");
    }
}
