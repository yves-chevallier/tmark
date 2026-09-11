//! One file per rule; `RULES` is the catalogue (design 05 §Rules).

mod caption_id;
mod directive_foreign;
mod feature_off;
mod hardcoded_number;
mod heading_skip;
mod icon_web_only;
mod lead_promotion;
mod position_word;
mod table_placement;
mod table_width;
mod table_width_sum;

use crate::Rule;

pub const RULES: &[&dyn Rule] = &[
    &hardcoded_number::HardcodedNumber,
    &position_word::PositionWord,
    &caption_id::CaptionIdOffConvention,
    &heading_skip::HeadingSkip,
    &lead_promotion::LeadPromotion,
    &table_placement::TablePlacement,
    &table_width::TableWidth,
    &table_width_sum::TableWidthSum,
    &directive_foreign::DirectiveForeign,
    &icon_web_only::IconWebOnly,
    &feature_off::FeatureOff,
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

/// The `table:` settings of a table or table-config node the parser holds
/// faithfully (a node with a kept `source` is a best effort: skipped).
pub(crate) fn table_settings(
    node: tmark_ir::NodeRef<'_>,
) -> Option<(&tmark_ir::TableSettings, tmark_ir::Span)> {
    match node {
        tmark_ir::NodeRef::Block(tmark_ir::Block::Table(t)) if t.source.is_none() => {
            Some((&t.model.settings, t.meta.span))
        }
        tmark_ir::NodeRef::Block(tmark_ir::Block::TableConfig(c)) if c.source.is_none() => {
            Some((&c.settings, c.meta.span))
        }
        _ => None,
    }
}

/// The number of a `NN%` or `NN.N%` width (TeXSmith `PERCENT_RE`).
pub(crate) fn percent(width: &str) -> Option<f64> {
    let number = width.strip_suffix('%')?;
    let valid = !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit() || c == '.')
        && number.matches('.').count() <= 1
        && number.starts_with(|c: char| c.is_ascii_digit());
    valid.then(|| number.parse().ok()).flatten()
}
