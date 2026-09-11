//! Media restriction (spec §Roles, `media=`; design 07 §Mapping rules: a
//! node with `media=print` is skipped by the HTML writer, `media=web` by
//! LaTeX and Typst, with the whitespace collapse of a zero-width node).

use tmark_ir::{Attrs, Block, Inline};

use crate::Media;

/// The node's `media=` excludes `media`.
pub fn skipped(attrs: &Attrs, media: Media) -> bool {
    matches!(
        (attrs.media(), media),
        (Some("print"), Media::Web) | (Some("web"), Media::Print)
    )
}

/// The attribute list of a block, when it has one.
pub fn block_attrs(block: &Block) -> Option<&Attrs> {
    match block {
        Block::Header(n) => Some(&n.attrs),
        Block::CodeBlock(n) => Some(&n.options),
        Block::BlockQuote(n) => Some(&n.attrs),
        Block::Table(n) => Some(&n.attrs),
        Block::Caption(n) => Some(&n.attrs),
        Block::Figure(n) => Some(&n.attrs),
        Block::Admonition(n) => Some(&n.attrs),
        Block::Div(n) => Some(&n.attrs),
        Block::MathBlock(n) => Some(&n.attrs),
        Block::Para(p) => match p.content.as_slice() {
            [Inline::Image(image)] => Some(&image.attrs),
            _ => None,
        },
        _ => None,
    }
}

/// The attribute list of an inline, when it has one.
pub fn inline_attrs(inline: &Inline) -> Option<&Attrs> {
    match inline {
        Inline::Image(n) => Some(&n.attrs),
        Inline::Span(n) => Some(&n.attrs),
        _ => None,
    }
}

pub fn block_skipped(block: &Block, media: Media) -> bool {
    block_attrs(block).is_some_and(|a| skipped(a, media))
}

pub fn inline_skipped(inline: &Inline, media: Media) -> bool {
    inline_attrs(inline).is_some_and(|a| skipped(a, media))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_rules() {
        let mut attrs = Attrs::new();
        assert!(!skipped(&attrs, Media::Print));
        attrs.kv.push(("media".into(), "print".into()));
        assert!(!skipped(&attrs, Media::Print));
        assert!(skipped(&attrs, Media::Web));
        attrs.kv[0].1 = "web".into();
        assert!(skipped(&attrs, Media::Print));
        attrs.kv[0].1 = "all".into();
        assert!(!skipped(&attrs, Media::Print));
    }
}
