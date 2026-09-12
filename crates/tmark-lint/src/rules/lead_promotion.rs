//! `lead-promotion`: a paragraph that is one strong span, promoted to a
//! lead-in by the `paragraph.lead` feature (spec §Para). Information, not
//! a fault: the formatter rewrites it to `{lead}[…]`.

use tmark_ir::{Block, Code, Diagnostic};

use crate::{Context, Rule};

pub struct LeadPromotion;

impl Rule for LeadPromotion {
    fn code(&self) -> Code {
        Code::LeadPromotion
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        tmark_ir::walk(ctx.doc, &mut |node| {
            let tmark_ir::NodeRef::Block(block) = node else {
                return;
            };
            let Block::Para(para) = block else { return };
            if para.lead.is_none() || para.meta.span.file != ctx.doc.file {
                return;
            }
            let start = para.meta.span.start as usize;
            let promoted = ctx.text.get(start..).is_some_and(|s| s.starts_with("**"));
            if promoted {
                out.push(Diagnostic::new(
                    Code::LeadPromotion,
                    para.meta.span,
                    "leading strong span promoted to a lead-in; `tmark fmt` writes `{lead}[…]`",
                ));
            }
        });
    }
}
