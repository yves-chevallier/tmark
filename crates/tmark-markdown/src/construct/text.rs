//! The text content type.
//!
//! **Text** contains phrasing content such as
//! [attention][crate::construct::attention] (emphasis, gfm strikethrough, strong),
//! [raw (text)][crate::construct::raw_text] (code (text), math (text)), and actual text.
//!
//! The constructs found in text are:
//!
//! * [Attention][crate::construct::attention] (emphasis, gfm strikethrough, strong)
//! * [Autolink][crate::construct::autolink]
//! * [Character escape][crate::construct::character_escape]
//! * [Character reference][crate::construct::character_reference]
//! * [Raw (text)][crate::construct::raw_text] (code (text), math (text))
//! * [GFM: Label start (footnote)][crate::construct::gfm_label_start_footnote]
//! * [GFM: Task list item check][crate::construct::gfm_task_list_item_check]
//! * [Hard break (escape)][crate::construct::hard_break_escape]
//! * [HTML (text)][crate::construct::html_text]
//! * [Label start (image)][crate::construct::label_start_image]
//! * [Label start (link)][crate::construct::label_start_link]
//! * [Label end][crate::construct::label_end]
//! * [MDX: expression (text)][crate::construct::mdx_expression_text]
//! * [MDX: JSX (text)][crate::construct::mdx_jsx_text]
//!
//! > 👉 **Note**: for performance reasons, hard break (trailing) is formed by
//! > [whitespace][crate::construct::partial_whitespace].

use crate::construct::gfm_autolink_literal::resolve as resolve_gfm_autolink_literal;
use crate::construct::partial_whitespace::resolve_whitespace;
use crate::resolve::Name as ResolveName;
use crate::state::{Name as StateName, State};
use crate::subtokenize::Subresult;
use crate::tokenizer::Tokenizer;

/// Characters that can start something in text.
const MARKERS: [u8; 21] = [
    b'!',  // `label_start_image`
    b'#',  // `tmark_define`
    b'$',  // `raw_text` (math (text))
    b'&',  // `character_reference`
    b'*',  // `attention` (emphasis, strong)
    b'+',  // `attention` (tmark keystroke)
    b'<',  // `autolink`, `html_text`, `mdx_jsx_text`
    b'=',  // `attention` (tmark highlight)
    b'@',  // `tmark_reference`
    b'H',  // `gfm_autolink_literal` (`protocol` kind)
    b'W',  // `gfm_autolink_literal` (`www.` kind)
    b'[',  // `label_start_link`
    b'\\', // `character_escape`, `hard_break_escape`, `tmark_math_compat`
    b']',  // `label_end`, `gfm_label_start_footnote`
    b'^',  // `attention` (tmark superscript, insert)
    b'_',  // `attention` (emphasis, strong)
    b'`',  // `raw_text` (code (text))
    b'h',  // `gfm_autolink_literal` (`protocol` kind)
    b'w',  // `gfm_autolink_literal` (`www.` kind)
    b'{',  // `tmark_brace`, `mdx_expression_text`
    b'~',  // `attention` (gfm strikethrough, tmark subscript)
];

/// Start of text.
///
/// There is a slightly weird case where task list items have their check at
/// the start of the first paragraph.
/// So we start by checking for that.
///
/// ```markdown
/// > | abc
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.markers = &MARKERS;
    tokenizer.attempt(
        State::Next(StateName::TextBefore),
        State::Next(StateName::TextBefore),
    );
    State::Retry(StateName::GfmTaskListItemCheckStart)
}

/// Before text.
///
/// ```markdown
/// > | abc
///     ^
/// ```
pub fn before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => {
            tokenizer.register_resolver(ResolveName::Data);
            tokenizer.register_resolver(ResolveName::Text);
            State::Ok
        }
        Some(b'!') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::LabelStartImageStart)
        }
        // raw (text) (code (text), math (text))
        Some(b'$' | b'`') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::RawTextStart)
        }
        Some(b'&') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::CharacterReferenceStart)
        }
        // attention (emphasis, gfm strikethrough, strong, tmark sugar)
        Some(b'*' | b'+' | b'=' | b'^' | b'_' | b'~') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::AttentionStart)
        }
        // `autolink`, `html_text` (order does not matter), `mdx_jsx_text` (order matters).
        Some(b'<') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeHtml),
            );
            State::Retry(StateName::AutolinkStart)
        }
        Some(b'H' | b'h') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::GfmAutolinkLiteralProtocolStart)
        }
        Some(b'W' | b'w') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::GfmAutolinkLiteralWwwStart)
        }
        Some(b'[') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeFootnoteLabel),
            );
            State::Retry(StateName::TmarkReferencePandocStart)
        }
        Some(b'\\') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeCharacterEscape),
            );
            State::Retry(StateName::TmarkMathCompatStart)
        }
        Some(b'#') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::TmarkDefineStart)
        }
        Some(b'@') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::TmarkReferenceStart)
        }
        Some(b']') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeData),
            );
            State::Retry(StateName::LabelEndStart)
        }
        Some(b'{') => {
            tokenizer.attempt(
                State::Next(StateName::TextBefore),
                State::Next(StateName::TextBeforeMdxExpression),
            );
            State::Retry(StateName::TmarkBraceStart)
        }
        _ => State::Retry(StateName::TextBeforeData),
    }
}

/// Before html (text).
///
/// At `<`, which wasn’t an autolink.
///
/// ```markdown
/// > | a <b>
///       ^
/// ```
pub fn before_html(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::TextBefore),
        State::Next(StateName::TextBeforeMdxJsx),
    );
    State::Retry(StateName::HtmlTextStart)
}

/// Before mdx jsx (text).
///
/// At `<`, which wasn’t an autolink or html.
///
/// ```markdown
/// > | a <b>
///       ^
/// ```
pub fn before_mdx_jsx(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::TextBefore),
        State::Next(StateName::TextBeforeData),
    );
    State::Retry(StateName::MdxJsxTextStart)
}

/// Before character escape.
///
/// At `\`, which wasn’t TMark's `\(…\)`.
pub fn before_character_escape(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::TextBefore),
        State::Next(StateName::TextBeforeHardBreakEscape),
    );
    State::Retry(StateName::CharacterEscapeStart)
}

/// Before MDX expression (text).
///
/// At `{`, which wasn’t a TMark brace group.
pub fn before_mdx_expression(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::TextBefore),
        State::Next(StateName::TextBeforeData),
    );
    State::Retry(StateName::MdxExpressionTextStart)
}

/// Before hard break escape.
///
/// At `\`, which wasn’t a character escape.
///
/// ```markdown
/// > | a \␊
///       ^
/// ```
pub fn before_hard_break_escape(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::TextBefore),
        State::Next(StateName::TextBeforeData),
    );
    State::Retry(StateName::HardBreakEscapeStart)
}

/// Before GFM label start (footnote).
///
/// At `[`, which wasn’t a Pandoc-style citation.
pub fn before_footnote_label(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::TextBefore),
        State::Next(StateName::TextBeforeLabelStartLink),
    );
    State::Retry(StateName::GfmLabelStartFootnoteStart)
}

/// Before label start (link).
///
/// At `[`, which wasn’t a GFM label start (footnote).
///
/// ```markdown
/// > | [a](b)
///     ^
/// ```
pub fn before_label_start_link(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::TextBefore),
        State::Next(StateName::TextBeforeData),
    );
    State::Retry(StateName::LabelStartLinkStart)
}

/// Before data.
///
/// ```markdown
/// > | a
///     ^
/// ```
pub fn before_data(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::TextBefore), State::Nok);
    State::Retry(StateName::DataStart)
}

/// Resolve whitespace.
pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    resolve_whitespace(
        tokenizer,
        tokenizer.parse_state.options.constructs.hard_break_trailing,
        true,
    );

    if tokenizer
        .parse_state
        .options
        .constructs
        .gfm_autolink_literal
    {
        resolve_gfm_autolink_literal(tokenizer);
    }

    tokenizer.map.consume(&mut tokenizer.events);
    None
}
