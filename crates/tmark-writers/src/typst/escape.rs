//! Typst markup escaping, from TeXSmith's `writers/typst/escaper.py`: the
//! characters that carry meaning in markup mode are backslash-escaped.

/// `_ESCAPE_CHARS`: `\ # $ * _ ` < > @ [ ]`.
const MARKUP: &[char] = &['\\', '#', '$', '*', '_', '`', '<', '>', '@', '[', ']'];

/// Escapes text for Typst markup mode.
pub fn markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    for c in text.chars() {
        if MARKUP.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Escapes text for a Typst string literal (`"…"`).
pub fn string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// A Typst label: `[^A-Za-z0-9_.:-]` becomes `-` (`typst/writer.py:932`,
/// shared with bibliography keys).
pub fn label(key: &str) -> String {
    key.trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// A raw span with a backtick fence longer than any run inside
/// (`_raw_inline`); a leading or trailing backtick is padded with a space.
pub fn raw_inline(text: &str) -> String {
    let fence = "`".repeat(longest_backtick_run(text) + 1);
    let pad = if text.starts_with('`') || text.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{text}{pad}{fence}")
}

/// A fence of at least three backticks, longer than any run in `body`.
pub fn fence_for(body: &str) -> String {
    "`".repeat((longest_backtick_run(body) + 1).max(3))
}

fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_markup_specials() {
        // `test_escaper_escapes_markup_specials` of TeXSmith.
        assert_eq!(
            markup("# * _ ` < > @ [ ] $ \\"),
            "\\# \\* \\_ \\` \\< \\> \\@ \\[ \\] \\$ \\\\"
        );
        assert_eq!(markup("plain text, 50%"), "plain text, 50%");
    }

    #[test]
    fn strings_labels_fences() {
        assert_eq!(string("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(label("fig:boot loop"), "fig:boot-loop");
        assert_eq!(label(" ein05 "), "ein05");
        assert_eq!(raw_inline("x"), "`x`");
        assert_eq!(raw_inline("a`b"), "``a`b``");
        assert_eq!(raw_inline("`a"), "`` `a ``");
        assert_eq!(fence_for("x"), "```");
        assert_eq!(fence_for("```x"), "````");
    }
}
