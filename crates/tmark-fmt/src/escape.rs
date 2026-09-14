//! Contextual, minimal escaping of literal text.
//!
//! Design `04-printer.md` §Implementation notes: "`\@` and `\#` only when
//! the following characters would otherwise form a reference or a
//! definition; `\{` only before a role or attribute head; standard
//! CommonMark escapes elsewhere. The fixed-point test catches every miss."
//! Spec §Lexical grammar gives the recognisers this module defends against
//! (the X4 guard for `@`, the `#[`/`#(` forms of X5, brace groups, the
//! caption line), CommonMark the rest (emphasis flanking, block starts).
//!
//! A `Str` holds decoded text: every backslash escape and entity of the
//! source is already resolved. The printer therefore re-escapes a character
//! only where it would be read as syntax in the position it is printed at.
//! Over-escaping is harmless to the round-trip (an escaped punctuation
//! character is that character); under-escaping breaks it, so ties go to
//! the escape.

use tmark_ir::sugar;

use crate::out::Out;

/// Where a run of text is printed.
#[derive(Copy, Clone, Debug, Default)]
pub struct Context {
    /// The run opens its block (first inline of a paragraph, list item,
    /// heading, cell): block-level recognisers apply to its first line.
    pub block_start: bool,
    /// Inside a pipe-table cell: `|` must be escaped (GFM).
    pub in_cell: bool,
    /// Inside a role's bracket group: unbalanced brackets would end it.
    pub in_group: bool,
    /// Right after an `IndexEntry`: a `[` would open one more group of
    /// `{index}[…][…]`.
    pub after_index: bool,
}

/// Writes `text` escaped for its position. `next` is the character that
/// follows the run: the first character of the next `Str`, a space, `\n`
/// for a break or the end of the block, `None` when another inline follows
/// (the conservative case).
pub fn text(out: &mut Out, text: &str, ctx: Context, next: Option<char>) {
    let chars: Vec<char> = text.chars().collect();
    let mut prev = out.last_char();
    // The first run of a block opens a line for the block recognisers
    // even after a marker (`- `, `> `, `# `): the column is not zero but
    // `# x` would still start a heading there.
    let mut line_start = out.at_line_start() || ctx.block_start;
    let mut buf = String::with_capacity(text.len() + 8);
    // Index of a character a line-start rule decided to escape later on the
    // line (the `.` of `1.`, the `:` of `Table:`).
    let mut pending: Option<usize> = None;
    let mut forced = autolink_escapes(&chars);
    forced.extend(quote_escapes(&chars));
    forced.extend(sugar_escapes(text));
    for (i, &c) in chars.iter().enumerate() {
        let next_c = chars.get(i + 1).copied().or(next);
        if c == '\n' {
            buf.push('\n');
            prev = Some('\n');
            line_start = true;
            pending = None;
            continue;
        }
        let mut esc = pending == Some(i) || forced.contains(&i);
        if i == 0 && c == '[' && ctx.after_index {
            esc = true;
        }
        if line_start {
            let (now, later) = line_start_escape(&chars, i, next, ctx.block_start && i == 0);
            esc |= now;
            pending = later;
        }
        esc |= general_escape(c, prev, next_c, ctx);
        if esc {
            buf.push('\\');
        }
        buf.push(c);
        prev = Some(c);
        line_start = false;
    }
    out.push(&buf);
}

/// Positions of the straight double quotes that would pair into a
/// `Quoted` when read back (`lower::sugar::quoted`): a `"` with a later
/// `"` on the same line and something between them. A quote that reached
/// the printer inside a `Str` is one the author escaped or one that never
/// paired; escaping the opening quote keeps it that way.
fn quote_escapes(chars: &[char]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            let close = chars[i + 1..]
                .iter()
                .position(|c| matches!(c, '"' | '\n'))
                .filter(|at| *at > 0 && chars[i + 1 + at] == '"');
            if let Some(at) = close {
                out.push(i);
                i += at + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Character positions of the inline sugar `tmark_ir::sugar` recognises
/// in a text run (spec §ProgressBar, Appendix "PyMdownX compatibility
/// profile"): the `[` of a progress bar spelling, the first punctuation
/// character of a smart symbol (`\(c)`, `\-->`, `\+/-`, `1\/2`, `c\/o`).
/// A spelling that reached the printer inside a `Str` is one the author
/// escaped, and the parser keeps a spelling literal when any character of
/// it is escaped in the source, so one backslash per spelling is enough.
fn sugar_escapes(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut index = 0;
    while i < text.len() {
        if text[i..].starts_with("[=") && sugar::progress_bar(&text[i..]).is_some() {
            out.push(index);
        }
        if let Some((len, _)) = sugar::smart_symbol(text, i) {
            let spelling = &text[i..i + len];
            if let Some(at) = spelling.chars().position(|c| c.is_ascii_punctuation()) {
                out.push(index + at);
            }
            i += len;
            index += spelling.chars().count();
            continue;
        }
        i += text[i..].chars().next().map_or(1, char::len_utf8);
        index += 1;
    }
    out
}

/// Positions to escape so that GFM autolink literals do not fire: the `:`
/// of `http://`, `https://`, `mailto:`, `xmpp:`, the `.` of `www.`, both at
/// a word start, and the `@` of an e-mail address (`user@host.tld`). The
/// parser decodes `\:`, `\.` and `\@` back to the literal text.
fn autolink_escapes(chars: &[char]) -> Vec<usize> {
    let mut out = Vec::new();
    let word_start = |i: usize| i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
    let starts_with = |i: usize, s: &str| {
        let pat: Vec<char> = s.chars().collect();
        chars.len() >= i + pat.len() && chars[i..i + pat.len()] == pat[..]
    };
    let mut i = 0;
    while i < chars.len() {
        if word_start(i) {
            for (scheme, colon) in [
                ("http://", 4),
                ("https://", 5),
                ("mailto:", 6),
                ("xmpp:", 4),
            ] {
                if starts_with(i, scheme) {
                    out.push(i + colon);
                }
            }
            if starts_with(i, "www.") && chars.get(i + 4).is_some_and(|c| c.is_alphanumeric()) {
                out.push(i + 3);
            }
        }
        if chars[i] == '@' && i > 0 && chars[i - 1].is_alphanumeric() {
            // `user@host.tld`: a dot after the `@`, inside the host.
            let host: String = chars[i + 1..]
                .iter()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '.' | '_'))
                .collect();
            if host.contains('.')
                && !host.ends_with('.')
                && host.starts_with(|c: char| c.is_alphanumeric())
            {
                out.push(i);
            }
        }
        i += 1;
    }
    out
}

fn is_ws(c: Option<char>) -> bool {
    c.is_some_and(char::is_whitespace)
}

fn is_alnum(c: Option<char>) -> bool {
    c.is_some_and(char::is_alphanumeric)
}

/// Characters that keep a bare `@` from being a reference when they precede
/// it (spec §Lexical grammar, the X4 look-behind `(?<![\w@/:.-])`).
fn guards_at(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '@' | '/' | ':' | '.' | '-'))
}

/// Rules that apply anywhere in a line.
fn general_escape(c: char, prev: Option<char>, next: Option<char>, ctx: Context) -> bool {
    match c {
        // A backslash before punctuation is an escape; before a line end a
        // hard break; before an unknown inline, who knows.
        '\\' => next.map_or(true, |n| n.is_ascii_punctuation() || n == '\n'),
        // Emphasis delimiters: harmless only between two spaces.
        '*' => !(is_ws(prev) && is_ws(next)),
        // `_` is intraword-safe between alphanumerics.
        '_' => !((is_alnum(prev) && is_alnum(next)) || (is_ws(prev) && is_ws(next))),
        // Code spans, sub/superscript sugar, strikeout, math: always.
        '`' | '~' | '^' | '$' => true,
        // `==mark==` and `++keys++` need a run of two.
        '=' | '+' => next == Some(c) || prev == Some(c),
        // Inline HTML and autolinks.
        '<' => next.is_some_and(|n| n.is_ascii_alphanumeric() || matches!(n, '/' | '!' | '?')),
        // Footnote reference, Pandoc citation, and any bracket inside a
        // role group.
        '[' => ctx.in_group || matches!(next, Some('^' | '@')),
        // A `]` followed by a destination, an attribute list, a reference
        // label or a definition colon closes a construct.
        ']' => ctx.in_group || matches!(next, Some('(' | '{' | '[' | ':')),
        // Image opener; `!!!` at a line start is handled by the line rules.
        '!' => next.is_none() || next == Some('['),
        // A brace group: role head, attribute list, moustache, or the
        // critic comment `{>>…<<}` (the other critic openers are escaped by
        // the `~`, `=` and `+` rules above).
        '{' => next
            .is_some_and(|n| n.is_alphanumeric() || matches!(n, '_' | '-' | '#' | '.' | '{' | '>')),
        // Spec §Two sigils: `@` refers, guarded by X4.
        '@' => (next.is_none() || next == Some('[') || is_alnum(next)) && !guards_at(prev),
        // Spec §Two sigils: `#[`, `#(`, and the deprecated `#{`.
        '#' => next.is_none() || matches!(next, Some('[' | '(' | '{')),
        // Entities.
        '&' => next.is_some_and(|n| n.is_ascii_alphanumeric() || n == '#'),
        '|' => ctx.in_cell,
        _ => false,
    }
}

/// Rules for the first character of a line. Returns whether to escape it
/// now and, optionally, the index of a later character to escape.
fn line_start_escape(
    chars: &[char],
    i: usize,
    next: Option<char>,
    block_start: bool,
) -> (bool, Option<usize>) {
    let c = chars[i];
    let at = |k: usize| {
        chars
            .get(i + k)
            .copied()
            .or(if i + k == chars.len() { next } else { None })
    };
    let line_end = chars[i..]
        .iter()
        .position(|&c| c == '\n')
        .map_or(chars.len(), |p| i + p);
    let line = &chars[i..line_end];
    // The line is made of `c` and spaces only, and nothing else follows it.
    let uniform = line.iter().all(|&x| x == c || x == ' ')
        && (line_end < chars.len() || matches!(next, None | Some('\n')));
    let now = match c {
        '#' => matches!(at(1), None | Some(' ' | '\t' | '#' | '\n')),
        '>' | '|' => true,
        '-' | '+' | '*' => {
            matches!(at(1), None | Some(' ' | '\t' | '\n')) || at(1) == Some(c) || uniform
        }
        '=' | '_' => uniform,
        ':' => matches!(at(1), Some(' ' | '\t' | ':')),
        '!' => at(1) == Some('!'),
        '?' => at(1) == Some('?'),
        '[' if block_start => {
            matches!(at(1), Some(' ' | 'x' | 'X' | '.'))
                && at(2) == Some(']')
                && matches!(at(3), Some(' ' | '\t'))
        }
        _ => false,
    };
    let mut later = None;
    if c.is_ascii_digit() {
        // `1.` or `1)` followed by a space: an ordered list marker.
        let digits = line.iter().take_while(|x| x.is_ascii_digit()).count();
        if digits <= 9
            && matches!(at(digits), Some('.' | ')'))
            && matches!(at(digits + 1), None | Some(' ' | '\t' | '\n'))
        {
            later = Some(i + digits);
        }
    }
    if block_start {
        // Spec §Caption: `Kind:` followed by whitespace opens a caption line.
        for kind in ["Table:", "Figure:", "Listing:"] {
            let k: Vec<char> = kind.chars().collect();
            if line.len() >= k.len()
                && line[..k.len()] == k[..]
                && matches!(at(k.len()), Some(' ' | '\t'))
            {
                later = Some(i + k.len() - 1);
            }
        }
    }
    (now, later)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esc(s: &str) -> String {
        let mut out = Out::new();
        text(
            &mut out,
            s,
            Context {
                block_start: true,
                ..Context::default()
            },
            Some('\n'),
        );
        out.finish().trim_end().to_string()
    }

    #[test]
    fn sigils_are_escaped_only_where_they_would_fire() {
        assert_eq!(esc("an @handle here"), "an \\@handle here");
        assert_eq!(esc("a #tag, #[x] and #(y)"), "a #tag, \\#[x] and \\#(y)");
        assert_eq!(esc("price 5 * 3 and a_b"), "price 5 * 3 and a_b");
        assert_eq!(
            esc("{sc}[x] and {#id} but {}"),
            "\\{sc}[x] and \\{#id} but {}"
        );
        // `[x] ` at a block start could be a task marker in a list item, and
        // `^` a superscript: both are over-escaped on purpose.
        assert_eq!(esc("[x] [^1] ![i]"), "\\[x] \\[\\^1] \\![i]");
    }

    #[test]
    fn block_starts_are_defused() {
        assert_eq!(esc("# not a heading"), "\\# not a heading");
        assert_eq!(esc("> quote"), "\\> quote");
        assert_eq!(esc("- item"), "\\- item");
        assert_eq!(esc("1. item"), "1\\. item");
        assert_eq!(esc("---"), "\\---");
        assert_eq!(esc("Table: not a caption"), "Table\\: not a caption");
        assert_eq!(esc("Tableau: fine"), "Tableau: fine");
        assert_eq!(esc(":   def"), "\\:   def");
        assert_eq!(esc("[ ] task"), "\\[ ] task");
    }

    #[test]
    fn autolink_literals_are_defused() {
        assert_eq!(
            esc("see http://x.y and www.x.y/z"),
            "see http\\://x.y and www\\.x.y/z"
        );
        assert_eq!(esc("mailto:me@x.y"), "mailto\\:me\\@x.y");
        assert_eq!(esc("mail me@example.com"), "mail me\\@example.com");
        assert_eq!(esc("a@b and ftp://x"), "a@b and ftp://x");
    }

    #[test]
    fn sugar_is_defused() {
        // Spec §ProgressBar and the appendix's smart symbols: a `Str`
        // spelled like one is literal text that must stay literal.
        assert_eq!(
            esc("A [=50% \"x\"] B [=50%]"),
            "A \\[=50% \\\"x\"] B \\[=50%]"
        );
        assert_eq!(esc("[=x%] and [x]"), "[=x%] and [x]");
        assert_eq!(
            esc("(c) (tm) (r) +/- =/= <--> --> <--"),
            "\\(c) \\(tm) \\(r) \\+/- \\=/= \\<--> \\--> \\<--"
        );
        assert_eq!(
            esc("1/2 cup, c/o Ada, 11/2, 1/7"),
            "1\\/2 cup, c\\/o Ada, 11/2, 1/7"
        );
        assert_eq!(esc("a ---> b"), "a ---> b");
    }

    #[test]
    fn commonmark_punctuation() {
        assert_eq!(esc("a `b` c"), "a \\`b\\` c");
        assert_eq!(esc("x*y* and _z_"), "x\\*y\\* and \\_z\\_");
        assert_eq!(esc("a < b <b> &amp; & c"), "a < b \\<b> \\&amp; & c");
        assert_eq!(esc("$5 and $6"), "\\$5 and \\$6");
        assert_eq!(esc("a\\b and c\\"), "a\\b and c\\\\");
    }
}
