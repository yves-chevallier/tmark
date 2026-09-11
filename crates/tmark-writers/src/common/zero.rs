//! The zero-width collapse (spec §Attributes: "Zero-width nodes … take no
//! space in the text: whitespace on both sides collapses to a single space,
//! and disappears before punctuation"). A post-pass over an inline
//! sequence shared by every writer (design 07 §Mapping rules; the rule of
//! writers-and-passes.md §2 "Zero-width").

use tmark_ir::{Inline, Str};

/// Punctuation a zero-width node must not separate from the word before it.
const CLOSING_PUNCTUATION: &[char] = &['.', ',', ';', ':', '!', '?', ')'];

/// Returns `inlines` with the whitespace around each inline that `is_zero`
/// judges zero-width adjusted: the trailing whitespace of the preceding
/// `Str` goes when the following text starts with whitespace, a break or
/// closing punctuation; a break following a break is dropped.
pub fn collapse(inlines: &[Inline], is_zero: &dyn Fn(&Inline) -> bool) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
    let mut i = 0;
    while i < inlines.len() {
        let inline = &inlines[i];
        if !is_zero(inline) {
            out.push(inline.clone());
            i += 1;
            continue;
        }
        // The next non-zero-width sibling decides.
        let mut j = i + 1;
        while j < inlines.len() && is_zero(&inlines[j]) {
            j += 1;
        }
        let next = inlines.get(j);
        let next_starts_blank = match next {
            Some(Inline::Str(s)) => s
                .text
                .chars()
                .next()
                .is_some_and(|c| c.is_whitespace() || CLOSING_PUNCTUATION.contains(&c)),
            Some(Inline::Space(_) | Inline::SoftBreak(_) | Inline::LineBreak(_)) => true,
            _ => false,
        };
        let prev_is_break = matches!(out.last(), Some(Inline::Space(_) | Inline::SoftBreak(_)));
        if next_starts_blank {
            match out.last_mut() {
                Some(Inline::Str(s)) => {
                    let trimmed = s.text.trim_end().len();
                    s.text.truncate(trimmed);
                }
                Some(Inline::Space(_) | Inline::SoftBreak(_))
                    if matches!(next, Some(Inline::Space(_) | Inline::SoftBreak(_))) =>
                {
                    out.pop();
                }
                _ => {}
            }
        } else if prev_is_break {
            // `a\n{index}[x]b`: nothing to trim on the right, keep the break.
        }
        out.extend(inlines[i..j].iter().cloned());
        i = j;
    }
    // A `Str` emptied by the trim is dropped.
    out.retain(|n| !matches!(n, Inline::Str(Str { text, .. }) if text.is_empty()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmark_ir::{Comment, Meta, SoftBreak, Str};

    fn s(text: &str) -> Inline {
        Inline::Str(Str {
            meta: Meta::default(),
            text: text.into(),
        })
    }

    fn c() -> Inline {
        Inline::Comment(Comment {
            meta: Meta::default(),
            text: "x".into(),
        })
    }

    fn zero(i: &Inline) -> bool {
        matches!(i, Inline::Comment(_))
    }

    fn texts(v: &[Inline]) -> Vec<String> {
        v.iter()
            .map(|i| match i {
                Inline::Str(s) => s.text.clone(),
                Inline::Comment(_) => "<c>".into(),
                Inline::SoftBreak(_) => "<nl>".into(),
                _ => "?".into(),
            })
            .collect()
    }

    #[test]
    fn collapses_to_one_space() {
        let v = collapse(&[s("a "), c(), s(" b")], &zero);
        assert_eq!(texts(&v), ["a", "<c>", " b"]);
    }

    #[test]
    fn disappears_before_punctuation() {
        let v = collapse(&[s("chien "), c(), s(".")], &zero);
        assert_eq!(texts(&v), ["chien", "<c>", "."]);
    }

    #[test]
    fn keeps_a_space_when_glued() {
        let v = collapse(&[s("a "), c(), s("b")], &zero);
        assert_eq!(texts(&v), ["a ", "<c>", "b"]);
        let v = collapse(&[s("a"), c(), s(" b")], &zero);
        assert_eq!(texts(&v), ["a", "<c>", " b"]);
    }

    #[test]
    fn several_in_a_row() {
        let v = collapse(&[s("a "), c(), s(" "), c(), s(" b")], &zero);
        assert_eq!(texts(&v), ["a", "<c>", "<c>", " b"]);
    }

    #[test]
    fn breaks() {
        let nl = || {
            Inline::SoftBreak(SoftBreak {
                meta: Meta::default(),
            })
        };
        let v = collapse(&[s("a"), nl(), c(), nl(), s("b")], &zero);
        assert_eq!(texts(&v), ["a", "<c>", "<nl>", "b"]);
        let v = collapse(&[s("a"), nl(), c(), s("b")], &zero);
        assert_eq!(texts(&v), ["a", "<nl>", "<c>", "b"]);
    }
}
