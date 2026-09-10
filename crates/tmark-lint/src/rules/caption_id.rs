//! `caption-id-off-convention`: a caption id whose prefix is not the
//! conventional one for its kind (spec §Anchor: `tbl:`, `fig:`, `lst:` are
//! recommended; a mismatch with the host is linted).

use tmark_ir::{Block, Code, Diagnostic};

use crate::{Context, Rule};

pub struct CaptionIdOffConvention;

impl Rule for CaptionIdOffConvention {
    fn code(&self) -> Code {
        Code::CaptionIdOffConvention
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        tmark_ir::walk(ctx.doc, &mut |node| {
            let tmark_ir::NodeRef::Block(block) = node else {
                return;
            };
            let Block::Caption(caption) = block else {
                return;
            };
            let Some(id) = caption.attrs.id() else { return };
            let expected = caption.kind.prefix();
            let prefix = id.split_once(':').map(|(p, _)| p);
            let user_series = prefix.is_some_and(|p| {
                ctx.resolved
                    .counters
                    .get(p)
                    .is_some_and(|c| c.tmark_numbered)
            });
            if prefix != Some(expected) && !user_series {
                out.push(Diagnostic::new(
                    Code::CaptionIdOffConvention,
                    caption.meta.span,
                    format!("caption id `{id}` does not follow the `{expected}:` convention"),
                ));
            }
        });
    }
}
