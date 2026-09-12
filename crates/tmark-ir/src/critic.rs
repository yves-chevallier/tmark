//! Critic markup (spec Appendix "PyMdownX compatibility profile", challenge
//! C49): the shapes a `Span{.critic}` takes and the one recogniser every
//! consumer shares.
//!
//! `{++x++}`, `{--x--}`, `{~~old~>new~~}` and `{>>note<<}` are *annotations*:
//! a reviewer's insertion, deletion, substitution or note, which the paged
//! backends typeset through the `ts-critic` contract (`\tsins`, `\tsdel`,
//! `\tssubst`, `\tscomment`). The IR keeps the node the appendix names —
//! `Underline`, `Strikeout`, the pair of the two, `Comment` — inside a
//! `Span` whose single class `critic` says the mark is an annotation and not
//! an author's own underline, strikeout or note. A consumer that ignores the
//! class still renders the inner node, which is the legacy rendering.
//!
//! `{==x==}` is *not* here: critic's highlight is `pymdownx.mark`'s
//! highlight, the contract defines no macro for it, and it lowers to a plain
//! `Highlight` like `==x==`.

use crate::node::{Inline, SpanNode};

/// The class that marks a `Span` as critic markup.
pub const CLASS: &str = "critic";

/// What a `Span{.critic}` annotates.
#[derive(Debug)]
pub enum Critic<'a> {
    /// `{++x++}`: `Underline` inside the span. `\tsins{…}`.
    Insert(&'a [Inline]),
    /// `{--x--}`: `Strikeout` inside the span. `\tsdel{…}`.
    Delete(&'a [Inline]),
    /// `{~~old~>new~~}`: the pair, in source order. `\tssubst{old}{new}`.
    Substitute {
        old: &'a [Inline],
        new: &'a [Inline],
    },
    /// `{>>note<<}`: a `Comment`. `\tscomment{…}`.
    Comment(&'a str),
}

/// The annotation a span carries, when it is critic markup: exactly the
/// class `critic`, no id, no other attribute, and one of the four shapes.
/// Anything else is an ordinary span.
pub fn critic(span: &SpanNode) -> Option<Critic<'_>> {
    let attrs = &span.attrs;
    if attrs.id.is_some() || !attrs.kv.is_empty() {
        return None;
    }
    match attrs.classes.as_slice() {
        [only] if only == CLASS => {}
        _ => return None,
    }
    match span.content.as_slice() {
        [Inline::Underline(n)] => Some(Critic::Insert(&n.content)),
        [Inline::Strikeout(n)] => Some(Critic::Delete(&n.content)),
        [Inline::Strikeout(old), Inline::Underline(new)] => Some(Critic::Substitute {
            old: &old.content,
            new: &new.content,
        }),
        [Inline::Comment(n)] => Some(Critic::Comment(&n.text)),
        _ => None,
    }
}

/// The five critic spellings, as the lowering reads them out of a literal
/// brace group (`value` is the text between the braces, so `++x++` for
/// `{++x++}`). The ranges are byte ranges into `value`.
#[derive(Debug, PartialEq)]
pub enum Spelling {
    Insert(usize, usize),
    Delete(usize, usize),
    Substitute {
        old: (usize, usize),
        new: (usize, usize),
    },
    /// `{==x==}`: a plain `Highlight`, not an annotation.
    Highlight(usize, usize),
    Comment(usize, usize),
}

/// The critic spelling a brace group's text is, if any. PyMdownX's
/// `pymdownx.critic` regex, restricted to one line by the brace group the
/// tokenizer hands over: the substitution needs its `~>`, the first one
/// splitting the two halves (the extension's non-greedy `.*?`).
pub fn spelling(value: &str) -> Option<Spelling> {
    let inner = |open: &str, close: &str| -> Option<(usize, usize)> {
        (value.len() >= open.len() + close.len()
            && value.starts_with(open)
            && value.ends_with(close))
        .then(|| (open.len(), value.len() - close.len()))
    };
    if let Some((start, end)) = inner("++", "++") {
        return Some(Spelling::Insert(start, end));
    }
    if let Some((start, end)) = inner("==", "==") {
        return Some(Spelling::Highlight(start, end));
    }
    if let Some((start, end)) = inner(">>", "<<") {
        return Some(Spelling::Comment(start, end));
    }
    if let Some((start, end)) = inner("~~", "~~") {
        let at = value[start..end].find("~>")?;
        return Some(Spelling::Substitute {
            old: (start, start + at),
            new: (start + at + 2, end),
        });
    }
    if let Some((start, end)) = inner("--", "--") {
        return Some(Spelling::Delete(start, end));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellings() {
        assert_eq!(spelling("++a++"), Some(Spelling::Insert(2, 3)));
        assert_eq!(spelling("--a--"), Some(Spelling::Delete(2, 3)));
        assert_eq!(spelling("==a=="), Some(Spelling::Highlight(2, 3)));
        assert_eq!(spelling(">>a<<"), Some(Spelling::Comment(2, 3)));
        assert_eq!(
            spelling("~~a~>bc~~"),
            Some(Spelling::Substitute {
                old: (2, 3),
                new: (5, 7)
            })
        );
        assert_eq!(spelling("++++"), Some(Spelling::Insert(2, 2)), "empty");
        assert_eq!(spelling("~~a~~"), None, "a substitution needs its `~>`");
        assert_eq!(spelling("--"), None, "the delimiters may not overlap");
        assert_eq!(spelling("k=v"), None);
        assert_eq!(spelling("--a++"), None);
    }
}
