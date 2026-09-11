//! `table-placement`: a `placement` that is not made of float placement
//! letters (spec §Table rung 5; TeXSmith `TableSettings._check_placement`,
//! `^[hHtbpT!]+$`).

use tmark_ir::{Code, Diagnostic};

use crate::{Context, Rule};

pub struct TablePlacement;

impl Rule for TablePlacement {
    fn code(&self) -> Code {
        Code::TablePlacement
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        tmark_ir::walk(ctx.doc, &mut |node| {
            let Some((settings, span)) = super::table_settings(node) else {
                return;
            };
            let Some(placement) = &settings.placement else {
                return;
            };
            if placement.is_empty() || !placement.chars().all(|c| "hHtbpT!".contains(c)) {
                out.push(Diagnostic::new(
                    Code::TablePlacement,
                    span,
                    format!("table placement `{placement}` is not made of the letters h, t, b, p, H, T and !"),
                ));
            }
        });
    }
}
