//! Line-based edits of a YAML front matter, for the fix attached to
//! `deprecated-frontmatter-key` (design 05 §Fixes, Appendix "Deprecation
//! schedule"): a top-level `counters:` block moves under `press.declare`,
//! `press.admonition_style` becomes `press.callouts.style`.
//!
//! Deliberately not a YAML rewrite: the front matter is copied byte for
//! byte by the printer (spec §Round-trip), so the fix moves the lines of
//! the key's block and leaves every other line untouched. It understands
//! block mappings indented with spaces, which is what documents write;
//! a key with a flow value (`press: {template: book}`) or a mapping it
//! cannot descend into gives no fix rather than a wrong one.

/// Leading spaces of a line.
fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty() || line.trim_start().starts_with('#')
}

/// The line `key:` at exactly `indent` inside `range`.
fn find_key(lines: &[&str], range: std::ops::Range<usize>, at: usize, key: &str) -> Option<usize> {
    range.into_iter().find(|&i| {
        let line = lines[i];
        indent(line) == at
            && line[at..]
                .strip_prefix(key)
                .and_then(|rest| rest.strip_prefix(':'))
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    })
}

/// End (exclusive) of the block that starts at `start`: the following lines
/// indented deeper than it; trailing blank lines stay outside.
fn block_end(lines: &[&str], start: usize) -> usize {
    let at = indent(lines[start]);
    let mut end = start + 1;
    while end < lines.len() && (is_blank(lines[end]) || indent(lines[end]) > at) {
        end += 1;
    }
    while end > start + 1 && is_blank(lines[end - 1]) {
        end -= 1;
    }
    end
}

/// Indent of the children of the block at `start`, when it has any.
fn child_indent(lines: &[&str], start: usize, end: usize) -> Option<usize> {
    lines[start + 1..end]
        .iter()
        .find(|l| !is_blank(l))
        .map(|l| indent(l))
}

/// Located block of a dotted path: `(start, end, indent)`.
fn locate(lines: &[&str], path: &[&str]) -> Option<(usize, usize, usize)> {
    let mut range = 0..lines.len();
    let mut at = 0;
    let mut found = None;
    for (depth, key) in path.iter().enumerate() {
        let start = find_key(lines, range.clone(), at, key)?;
        let end = block_end(lines, start);
        found = Some((start, end, at));
        if depth + 1 < path.len() {
            at = child_indent(lines, start, end)?;
            range = start + 1..end;
        }
    }
    found
}

/// Moves the block of `from` (a dotted path) to `to` (a dotted path whose
/// last segment is the new key name), creating the intermediate mappings
/// at the end of their parent. `None` when the source is missing, the
/// target exists already (the parser drops the source then, so the source
/// is only removed), or the structure is not a block mapping.
pub fn move_key(text: &str, from: &str, to: &str) -> Option<String> {
    let from: Vec<&str> = from.split('.').collect();
    let to: Vec<&str> = to.split('.').collect();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    let (start, end, at) = locate(&borrowed, &from)?;
    // The moved block, dedented and renamed.
    let mut block: Vec<String> = lines[start..end]
        .iter()
        .map(|l| {
            if is_blank(l) {
                String::new()
            } else {
                l[at..].to_string()
            }
        })
        .collect();
    let (old_key, new_key) = (from[from.len() - 1], to[to.len() - 1]);
    if old_key != new_key {
        block[0] = format!("{new_key}{}", &block[0][old_key.len()..]);
    }
    lines.drain(start..end);
    // Descend to the target parent, creating what is missing.
    let mut parent: Option<(usize, usize)> = None; // (start, end) of the parent block
    let mut at = 0;
    for key in &to[..to.len() - 1] {
        let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
        let range = parent.map_or(0..borrowed.len(), |(s, e)| s + 1..e);
        let start = match find_key(&borrowed, range, at, key) {
            Some(start) => start,
            None => {
                let insert_at = parent.map_or(borrowed.len(), |(_, e)| e);
                lines.insert(insert_at, format!("{}{key}:", " ".repeat(at)));
                insert_at
            }
        };
        let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
        let end = block_end(&borrowed, start);
        if end == start + 1 && !borrowed[start].trim_end().ends_with(':') {
            return None; // an inline value: not a mapping we can extend
        }
        at = child_indent(&borrowed, start, end).unwrap_or(at + 2);
        parent = Some((start, end));
    }
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    let (insert_at, exists) = match parent {
        Some((s, e)) => (e, find_key(&borrowed, s + 1..e, at, new_key).is_some()),
        None => (
            borrowed.len(),
            find_key(&borrowed, 0..borrowed.len(), 0, new_key).is_some(),
        ),
    };
    if !exists {
        let pad = " ".repeat(at);
        for (i, line) in block.into_iter().enumerate() {
            let line = if line.is_empty() {
                line
            } else {
                format!("{pad}{line}")
            };
            lines.insert(insert_at + i, line);
        }
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_a_top_level_group_under_press() {
        let text = "title: T\ncounters:\n  fw:\n    name: Finding\n\nbibliography: refs.bib\npress:\n  template: book\n";
        let out = move_key(text, "counters", "press.declare.counters").unwrap();
        assert_eq!(
            out,
            "title: T\n\nbibliography: refs.bib\npress:\n  template: book\n  declare:\n    counters:\n      fw:\n        name: Finding\n"
        );
        let out = move_key(&out, "bibliography", "press.sources.bibliography").unwrap();
        assert_eq!(
            out,
            "title: T\n\npress:\n  template: book\n  declare:\n    counters:\n      fw:\n        name: Finding\n  sources:\n    bibliography: refs.bib\n"
        );
    }

    #[test]
    fn creates_press_when_absent_and_keeps_existing_groups() {
        let out = move_key(
            "glossary:\n  style: long\n",
            "glossary",
            "press.declare.glossary",
        )
        .unwrap();
        assert_eq!(
            out,
            "press:\n  declare:\n    glossary:\n      style: long\n"
        );
        let text =
            "acronyms: {a: b}\npress:\n    declare:\n        counters: {}\n    sources: {}\n";
        let out = move_key(text, "acronyms", "press.declare.acronyms").unwrap();
        assert_eq!(
            out,
            "press:\n    declare:\n        counters: {}\n        acronyms: {a: b}\n    sources: {}\n"
        );
    }

    #[test]
    fn renames_a_press_key_into_a_group() {
        let text = "press:\n  admonition_style: classic\n  fonts: x\n";
        let out = move_key(text, "press.admonition_style", "press.callouts.style").unwrap();
        assert_eq!(out, "press:\n  fonts: x\n  callouts:\n    style: classic\n");
    }

    #[test]
    fn gives_up_on_flow_mappings_and_removes_a_shadowed_source() {
        assert!(move_key(
            "press: {template: book}\ncounters: {}\n",
            "counters",
            "press.declare.counters"
        )
        .is_none());
        assert!(move_key("title: T\n", "counters", "press.declare.counters").is_none());
        let text = "counters: {a: 1}\npress:\n  declare:\n    counters: {b: 2}\n";
        let out = move_key(text, "counters", "press.declare.counters").unwrap();
        assert_eq!(out, "press:\n  declare:\n    counters: {b: 2}\n");
    }
}
