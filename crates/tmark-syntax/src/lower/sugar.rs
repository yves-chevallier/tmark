//! Inline sugar found inside text runs (spec Appendix "PyMdownX
//! compatibility profile"): emoji shortcodes `:smile:` expanded from the
//! `gemoji` table, the Material icon shortcodes `:material-cog:` (§Emoji
//! and icon shortcodes), and straight double quotes (§Quoted). The
//! progress bar and the smart symbols live in `tmark_ir::sugar`, shared
//! with the printer that escapes them; `inline.rs` turns the pieces into
//! nodes.

use tmark_ir::emoji;

pub use tmark_ir::sugar::{progress_bar, smart_symbol, Progress};

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
