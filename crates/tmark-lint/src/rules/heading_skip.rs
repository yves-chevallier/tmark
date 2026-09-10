//! `heading-skip`: a heading level deeper than the previous one by more
//! than one step (spec §Header: headings are relative and re-aligned, so a
//! skipped level is almost always a typo).

use tmark_ir::{Block, Code, Diagnostic};

use crate::{Context, Rule};

pub struct HeadingSkip;

impl Rule for HeadingSkip {
    fn code(&self) -> Code {
        Code::HeadingSkip
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        let mut previous: Option<u8> = None;
        for block in &ctx.doc.blocks {
            let Block::Header(h) = block else { continue };
            if let Some(p) = previous {
                if h.level > p + 1 {
                    out.push(Diagnostic::new(
                        Code::HeadingSkip,
                        h.meta.span,
                        format!("heading level {} follows level {p}", h.level),
                    ));
                }
            }
            previous = Some(h.level);
        }
    }
}
