//! `icon-web-only`: a Material icon shortcode (`:material-cog:`), which
//! lowers to `Span{.icon media=web}` and reaches no print backend (spec
//! §Emoji and icon shortcodes).

use tmark_ir::{Code, Diagnostic, Inline, NodeRef};

use crate::{Context, Rule};

pub struct IconWebOnly;

impl Rule for IconWebOnly {
    fn code(&self) -> Code {
        Code::IconWebOnly
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        tmark_ir::walk(ctx.doc, &mut |node: NodeRef| {
            let NodeRef::Inline(Inline::Span(span)) = node else {
                return;
            };
            if !span.attrs.has_class("icon") || span.attrs.media() != Some("web") {
                return;
            }
            let shortcode = tmark_ir::plain_text(&span.content);
            out.push(Diagnostic::new(
                Code::IconWebOnly,
                span.meta.span,
                format!("`{shortcode}` is a web-only icon; print drops it (use an emoji or an image for a symbol that must reach print)"),
            ));
        });
    }
}
