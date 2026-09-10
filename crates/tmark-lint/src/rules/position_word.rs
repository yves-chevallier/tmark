//! `position-word`: "above" and "below" in prose. Floats move in print, so a
//! position word is a reference in disguise (spec §Ref).

use tmark_ir::{Code, Diagnostic};

use super::{strings, sub_span};
use crate::{Context, Rule};

pub struct PositionWord;

impl Rule for PositionWord {
    fn code(&self) -> Code {
        Code::PositionWord
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        strings(ctx.doc, |text, span| {
            let lower = text.to_ascii_lowercase();
            for word in ["above", "below"] {
                let mut from = 0;
                while let Some(at) = lower[from..].find(word) {
                    let start = from + at;
                    let end = start + word.len();
                    from = end;
                    let before =
                        start == 0 || !lower[..start].ends_with(|c: char| c.is_alphanumeric());
                    let after = end == lower.len()
                        || !lower[end..].starts_with(|c: char| c.is_alphanumeric());
                    if before && after {
                        out.push(Diagnostic::new(
                            Code::PositionWord,
                            sub_span(span, start, end),
                            format!("`{}` assumes a position; in print floats move, so refer to the element with `@`", &text[start..end]),
                        ));
                    }
                }
            }
        });
    }
}
