//! Inline sugar found inside text runs (spec Appendix "PyMdownX
//! compatibility profile"): emoji shortcodes `:smile:` expanded from the
//! `gemoji` table, the Material icon shortcodes `:material-cog:` (§Emoji
//! and icon shortcodes). The progress bar, the smart symbols and the
//! straight double quotes (§Quoted) live in `tmark_ir::sugar`, shared
//! with the printer that escapes them; `inline.rs` turns the pieces into
//! nodes.

use tmark_ir::emoji;

pub use tmark_ir::sugar::{progress_bar, quoted, smart_symbol, Progress};

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

#[cfg(test)]
mod tests {
    use super::*;

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
