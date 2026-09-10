//! `hardcoded-number`: "Figure 3" typed in prose instead of a reference
//! (spec §Tooling roadmap, `tmark lint`).

use tmark_ir::{Code, Diagnostic};

use super::{strings, sub_span};
use crate::{Context, Rule};

pub struct HardcodedNumber;

const WORDS: &[&str] = &[
    "Figure", "Table", "Listing", "Section", "Chapter", "Equation", "Appendix",
];

impl Rule for HardcodedNumber {
    fn code(&self) -> Code {
        Code::HardcodedNumber
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        strings(ctx.doc, |text, span| {
            for word in WORDS {
                let mut from = 0;
                while let Some(at) = text[from..].find(word) {
                    let start = from + at;
                    let end = start + word.len();
                    from = end;
                    let before_ok =
                        start == 0 || !text[..start].ends_with(|c: char| c.is_alphanumeric());
                    let rest = &text[end..];
                    let mut digits = rest.strip_prefix(' ').unwrap_or("");
                    let n = digits.chars().take_while(|c| c.is_ascii_digit()).count();
                    if !before_ok || n == 0 {
                        continue;
                    }
                    digits = &digits[n..];
                    // `Table 3.` and `Figure 3,` are prose; `Table 3` in "Table 3 shows".
                    let after_ok =
                        digits.is_empty() || !digits.starts_with(|c: char| c.is_alphanumeric());
                    if after_ok {
                        out.push(Diagnostic::new(
                            Code::HardcodedNumber,
                            sub_span(span, start, end + 1 + n),
                            format!("`{word} {}` is typed by hand; refer to the element with `@` instead", &rest[1..1 + n]),
                        ));
                    }
                }
            }
        });
    }
}
