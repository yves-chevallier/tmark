//! Inline sugar found inside text runs (spec Appendix "PyMdownX
//! compatibility profile"): the progress bar `[=45% "label"]` and its
//! deprecated fraction form (§ProgressBar), emoji shortcodes `:smile:`
//! expanded from the `gemoji` table, and the Material icon shortcodes
//! `:material-cog:` (§Emoji and icon shortcodes). Pure scanners over the
//! decoded text; `inline.rs` turns the pieces into nodes.

use tmark_ir::emoji;

/// A recognised progress bar spelling at the start of a text.
#[derive(Debug, PartialEq)]
pub struct Progress {
    /// Bytes of the spelling, `[` to `]`.
    pub len: usize,
    /// Percentage, 0 to 100 (clamped).
    pub value: f64,
    pub label: Option<String>,
    /// Written as `[=a/b …]` (deprecated).
    pub fraction: bool,
}

fn number(s: &str) -> Option<(f64, usize)> {
    let digits = s
        .bytes()
        .take_while(|b| b.is_ascii_digit() || *b == b'.')
        .count();
    let text = &s[..digits];
    if text.is_empty()
        || !text.starts_with(|c: char| c.is_ascii_digit())
        || text.matches('.').count() > 1
        || text.ends_with('.')
    {
        return None;
    }
    text.parse().ok().map(|v| (v, digits))
}

/// The spec's recogniser at the start of `text`:
/// `\[=\s*(\d+(?:\.\d+)?)%(?:\s+"([^"]*)")?\s*\]`, plus the fraction
/// sugar `[=a/b "…"]`.
pub fn progress_bar(text: &str) -> Option<Progress> {
    let rest = text.strip_prefix("[=")?;
    let rest = rest.trim_start_matches([' ', '\t']);
    let (first, used) = number(rest)?;
    let rest = &rest[used..];
    let (value, fraction, rest) = if let Some(rest) = rest.strip_prefix('%') {
        (first, false, rest)
    } else {
        let rest = rest.strip_prefix('/')?;
        let (second, used) = number(rest)?;
        if second <= 0.0 {
            return None;
        }
        (first / second * 100.0, true, &rest[used..])
    };
    let mut label = None;
    let mut rest = rest;
    let blanks = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    if blanks > 0 && rest[blanks..].starts_with('"') {
        let inner = &rest[blanks + 1..];
        let end = inner.find(['"', '\n'])?;
        if !inner[end..].starts_with('"') {
            return None;
        }
        label = Some(inner[..end].to_string());
        rest = &inner[end + 1..];
    }
    let rest = rest.trim_start_matches([' ', '\t']);
    let rest = rest.strip_prefix(']')?;
    Some(Progress {
        len: text.len() - rest.len(),
        value: value.clamp(0.0, 100.0),
        label,
        fraction,
    })
}

/// A shortcode at `at` in `text` (a `:` there): its byte length and what
/// it is. A colon-delimited word touching a word character on either side
/// (`12:30:45`, `a:b:c`) is not one; a name the emoji table does not know
/// is not one either, so it stays literal with no diagnostic.
pub enum Shortcode {
    Emoji(&'static str),
    Icon,
}

pub fn shortcode(text: &str, at: usize) -> Option<(usize, Shortcode)> {
    let rest = &text[at..];
    if !rest.starts_with(':') || text[..at].ends_with(|c: char| c.is_alphanumeric()) {
        return None;
    }
    let body = &rest[1..];
    let len = body
        .bytes()
        .take_while(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'+')
        })
        .count();
    if len == 0 || !body[len..].starts_with(':') {
        return None;
    }
    if body[len + 1..].starts_with(|c: char| c.is_alphanumeric()) {
        return None;
    }
    let name = &body[..len];
    let kind = if emoji::is_icon(name) {
        Shortcode::Icon
    } else {
        Shortcode::Emoji(emoji::emoji(name)?)
    };
    Some((len + 2, kind))
}

/// The smart symbols of `pymdownx.smartsymbols` (spec Appendix "PyMdownX
/// compatibility profile": `(c)`, `(tm)`, `-->`, `1/2` → `Str`), with the
/// extension's own boundaries: an arrow is not part of a longer run of
/// dashes, a fraction not part of a longer number, `c/o` a whole word.
/// Returns the byte length of the spelling at `at` and the character it
/// stands for. Ordinal numbers (`1st`) are not here: they produce a
/// superscript, not a `Str` (design 02 §Text runs).
const SYMBOLS: &[(&str, &str)] = &[
    ("(tm)", "™"),
    ("(c)", "©"),
    ("(r)", "®"),
    ("+/-", "±"),
    ("=/=", "≠"),
    ("<-->", "↔"),
    ("-->", "→"),
    ("<--", "←"),
];

const FRACTIONS: &[(&str, &str)] = &[
    ("1/4", "¼"),
    ("1/2", "½"),
    ("3/4", "¾"),
    ("1/3", "⅓"),
    ("2/3", "⅔"),
    ("1/5", "⅕"),
    ("2/5", "⅖"),
    ("3/5", "⅗"),
    ("4/5", "⅘"),
    ("1/6", "⅙"),
    ("5/6", "⅚"),
    ("1/8", "⅛"),
    ("3/8", "⅜"),
    ("5/8", "⅝"),
    ("7/8", "⅞"),
];

pub fn smart_symbol(text: &str, at: usize) -> Option<(usize, &'static str)> {
    let rest = &text[at..];
    let before = text[..at].chars().next_back();
    for (spelling, symbol) in SYMBOLS {
        if !rest.starts_with(spelling) {
            continue;
        }
        // `\<-{2}\>|(?<!-)-{2}\>|\<-{2}(?!-)`: a longer dash run is not
        // an arrow.
        if spelling.starts_with('-') && before == Some('-') {
            continue;
        }
        if spelling.ends_with("--") && rest[spelling.len()..].starts_with('-') {
            continue;
        }
        return Some((spelling.len(), symbol));
    }
    // `\bc/o\b`.
    if rest.starts_with("c/o")
        && !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
        && !rest[3..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
    {
        return Some((3, "℅"));
    }
    // `(?<!\d)…(?!\d)`.
    if before.is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    FRACTIONS
        .iter()
        .find(|(spelling, _)| {
            rest.starts_with(spelling)
                && !rest[spelling.len()..].starts_with(|c: char| c.is_ascii_digit())
        })
        .map(|(spelling, symbol)| (spelling.len(), *symbol))
}

/// A straight double-quoted phrase at `at` in `text`: the byte range of
/// the phrase between the quotes. TeXSmith's `quotes.py` pattern
/// `(?<!\\)"([^"\n]+?)"`, which is what feeds `\enquote{…}` (spec §Quoted,
/// SmartyPants). Single quotes are not paired: an apostrophe is not a
/// quote and the legacy extension never touched them.
pub fn quoted(text: &str, at: usize) -> Option<(usize, usize)> {
    if !text[at..].starts_with('"') {
        return None;
    }
    let inner = &text[at + 1..];
    let end = inner.find('"')?;
    if end == 0 || inner[..end].contains('\n') {
        return None;
    }
    Some((at + 1, at + 1 + end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress() {
        let p = progress_bar("[=45% \"Review\"] rest").unwrap();
        assert_eq!((p.len, p.value, p.fraction), (15, 45.0, false));
        assert_eq!(p.label.as_deref(), Some("Review"));
        let p = progress_bar("[=9/20 \"Review\"]").unwrap();
        assert_eq!((p.len, p.value, p.fraction), (16, 45.0, true));
        let p = progress_bar("[=100%]").unwrap();
        assert_eq!((p.len, p.value), (7, 100.0));
        assert!(p.label.is_none());
        assert_eq!(progress_bar("[= 12.5% ]").unwrap().value, 12.5);
        assert_eq!(progress_bar("[=250%]").unwrap().value, 100.0, "clamped");
        assert!(
            progress_bar("[=45%\"x\"]").is_none(),
            "a blank before the label"
        );
        assert!(progress_bar("[=x%]").is_none());
        assert!(progress_bar("[=45% \"open]").is_none());
        assert!(progress_bar("[=1/0]").is_none());
        assert!(progress_bar("[45%]").is_none());
    }

    #[test]
    fn symbols() {
        assert_eq!(smart_symbol("(c) x", 0), Some((3, "©")));
        assert_eq!(smart_symbol("a (tm)", 2), Some((4, "™")));
        assert_eq!(smart_symbol("(r)", 0), Some((3, "®")));
        assert_eq!(smart_symbol("+/-", 0), Some((3, "±")));
        assert_eq!(smart_symbol("=/=", 0), Some((3, "≠")));
        assert_eq!(smart_symbol("a <--> b", 2), Some((4, "↔")));
        assert_eq!(smart_symbol("a --> b", 2), Some((3, "→")));
        assert_eq!(smart_symbol("a <-- b", 2), Some((3, "←")));
        assert_eq!(smart_symbol("a ---> b", 3), None, "a longer dash run");
        assert_eq!(smart_symbol("a <--- b", 2), None, "a longer dash run");
        assert_eq!(smart_symbol("c/o Ada", 0), Some((3, "℅")));
        assert_eq!(smart_symbol("abc/o", 2), None, "not a whole word");
        assert_eq!(smart_symbol("1/2 cup", 0), Some((3, "½")));
        assert_eq!(smart_symbol("11/2", 1), None, "part of a number");
        assert_eq!(smart_symbol("1/25", 0), None, "part of a number");
        assert_eq!(smart_symbol("1/7", 0), None, "no such fraction");
    }

    #[test]
    fn quotes() {
        assert_eq!(quoted("He said \"straight quotes\".", 8), Some((9, 24)));
        assert_eq!(quoted("a \"b\" and \"c\"", 2), Some((3, 4)));
        assert_eq!(quoted("a \"\" b", 2), None, "empty");
        assert_eq!(quoted("a \"b", 2), None, "unclosed");
    }

    #[test]
    fn shortcodes() {
        assert!(matches!(
            shortcode(":smile: x", 0),
            Some((7, Shortcode::Emoji("\u{1f604}")))
        ));
        assert!(matches!(
            shortcode("a :+1:", 2),
            Some((4, Shortcode::Emoji(_)))
        ));
        assert!(matches!(
            shortcode(":material-cog:", 0),
            Some((14, Shortcode::Icon))
        ));
        assert!(shortcode("12:30:45", 2).is_none());
        assert!(shortcode("a:b:c", 1).is_none());
        assert!(shortcode(":nope-not-an-emoji:", 0).is_none());
        assert!(shortcode(":Smile:", 0).is_none());
        assert!(shortcode("note: text:", 4).is_none());
    }
}
