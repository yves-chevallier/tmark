//! One file per rule; `RULES` is the catalogue (design 05 §Rules).

mod caption_id;
mod hardcoded_number;
mod heading_skip;
mod lead_promotion;
mod position_word;

use crate::Rule;

pub const RULES: &[&dyn Rule] = &[
    &hardcoded_number::HardcodedNumber,
    &position_word::PositionWord,
    &caption_id::CaptionIdOffConvention,
    &heading_skip::HeadingSkip,
    &lead_promotion::LeadPromotion,
];

/// Walk every `Str` of the document with its span.
pub(crate) fn strings(doc: &tmark_ir::Document, mut f: impl FnMut(&str, tmark_ir::Span)) {
    tmark_ir::walk(doc, &mut |node| {
        if let tmark_ir::NodeRef::Inline(tmark_ir::Inline::Str(s)) = node {
            f(&s.text, s.meta.span);
        }
    });
}

/// Span of a byte range inside a `Str`'s span.
pub(crate) fn sub_span(span: tmark_ir::Span, start: usize, end: usize) -> tmark_ir::Span {
    tmark_ir::Span::new(
        span.file,
        span.start + start as u32,
        span.start + end as u32,
    )
}
