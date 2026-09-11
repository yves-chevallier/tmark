//! `directive-foreign`: a dotted `::: a.b` directive for another processor
//! (mkdocstrings), kept verbatim by the printer and dropped by the HTML
//! and paged writers (spec §Foreign directive). `[TOC]` is the same node
//! and stays silent: its print equivalent is `press.toc`.

use tmark_ir::{Block, Code, Diagnostic, NodeRef};

use crate::{Context, Rule};

pub struct DirectiveForeign;

impl Rule for DirectiveForeign {
    fn code(&self) -> Code {
        Code::DirectiveForeign
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        tmark_ir::walk(ctx.doc, &mut |node: NodeRef| {
            let NodeRef::Block(Block::RawBlock(raw)) = node else {
                return;
            };
            if raw.format != "markdown" || !raw.text.starts_with(':') {
                return;
            }
            let head = raw.text.lines().next().unwrap_or_default().trim();
            out.push(Diagnostic::new(
                Code::DirectiveForeign,
                raw.meta.span,
                format!("`{head}` is a directive for another processor; the HTML and paged writers drop it"),
            ));
        });
    }
}
