//! `feature-off`: a spelling gated on a feature that is off, so it stayed
//! literal text: `^^x^^` without `inline.insert` (spec §Inline text,
//! §Feature registry).

use tmark_ir::{Code, Diagnostic};

use super::{strings, sub_span};
use crate::{Context, Rule};

pub struct FeatureOff;

/// `^^…^^` runs in `text`, as byte ranges: the markers with a non-empty,
/// non-blank content between them that holds no `^^`.
fn insert_runs(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find("^^") {
        let start = from + at;
        let inner_start = start + 2;
        let Some(len) = text[inner_start..].find("^^") else {
            break;
        };
        let inner = &text[inner_start..inner_start + len];
        if !inner.trim().is_empty()
            && !inner.starts_with(char::is_whitespace)
            && !inner.ends_with(char::is_whitespace)
        {
            let end = inner_start + len + 2;
            out.push((start, end));
            from = end;
        } else {
            from = inner_start;
        }
    }
    out
}

impl Rule for FeatureOff {
    fn code(&self) -> Code {
        Code::FeatureOff
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        if ctx.doc.front_matter.keys.press.feature("inline.insert") {
            return;
        }
        strings(ctx.doc, |text, span| {
            let exact = ctx
                .text
                .get(span.start as usize..span.end as usize)
                .is_some_and(|source| source == text);
            for (start, end) in insert_runs(text) {
                let at = if exact {
                    sub_span(span, start, end)
                } else {
                    span
                };
                out.push(Diagnostic::new(
                    Code::FeatureOff,
                    at,
                    format!(
                        "`{}` is literal text: `^^x^^` needs the feature `inline.insert` (or write `{{underline}}[x]`)",
                        &text[start..end]
                    ),
                ));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::insert_runs;

    #[test]
    fn runs() {
        assert_eq!(insert_runs("Now ^^inserted^^ text."), vec![(4, 16)]);
        assert_eq!(insert_runs("a ^^b^^ c ^^d^^"), vec![(2, 7), (10, 15)]);
        assert!(insert_runs("^^ ^^ and ^^^^").is_empty());
        assert!(insert_runs("no markers").is_empty());
    }
}
