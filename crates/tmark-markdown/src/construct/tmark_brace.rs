//! TMark brace group, in the [text][] content type.
//!
//! One construct covers the three brace-delimited forms of the spec, which
//! the tokenizer does not need to tell apart:
//!
//! * a role head, `{name positional key=value}`, followed by one or more
//!   bracket groups (`[content]`) or by a parenthesised argument;
//! * an attribute list, `{#id .class key=value}`;
//! * a moustache, `{{ key }}`.
//!
//! ## Grammar
//!
//! ```bnf
//! tmark_brace ::= '{' *(char - '}' - eol | '"' *(char - '"' - eol) '"') '}'
//!               | '{{' *(char - eol) '}}'
//! ```
//!
//! The group closes on the same line or it is not a group at all: an
//! unclosed `{` is literal text (spec §Roles: "a brace group which is
//! neither stays literal").
//!
//! ## Tokens
//!
//! * [`TmarkBrace`][Name::TmarkBrace]
//! * [`TmarkBraceMarker`][Name::TmarkBraceMarker]
//! * [`TmarkBraceData`][Name::TmarkBraceData]
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::resolve::Name as ResolveName;
use crate::state::{Name as StateName, State};
use crate::tokenizer::{LabelKind, LabelStart, Tokenizer};
use crate::util::tmark::{line_end, looks_like_attributes};

/// Whether a brace opened at `index` closes on the same line.
fn closes_on_line(bytes: &[u8], index: usize, moustache: bool) -> bool {
    let end = line_end(bytes, index);
    let mut i = index + if moustache { 2 } else { 1 };
    let mut quoted = false;
    while i < end {
        match bytes[i] {
            b'"' if !moustache => quoted = !quoted,
            b'}' if !quoted => {
                if !moustache {
                    return true;
                }
                if bytes.get(i + 1) == Some(&b'}') {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// Consume a `[` as the start of a bracket group and register it as a
/// label start, so that the label machinery closes it at `]`.
///
/// Used after a role head, after `#`, and after a group (`][`).
pub fn group_start(tokenizer: &mut Tokenizer) {
    debug_assert_eq!(tokenizer.current, Some(b'['), "expected `[`");
    tokenizer.enter(Name::LabelLink);
    tokenizer.enter(Name::LabelMarker);
    tokenizer.consume();
    tokenizer.exit(Name::LabelMarker);
    tokenizer.exit(Name::LabelLink);
    tokenizer.tokenize_state.label_starts.push(LabelStart {
        kind: LabelKind::TmarkGroup,
        start: (tokenizer.events.len() - 4, tokenizer.events.len() - 1),
        inactive: false,
    });
    tokenizer.register_resolver_before(ResolveName::Label);
}

/// Start of a brace group.
///
/// ```markdown
/// > | a {aside}[b] c
///       ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b'{') || !tokenizer.parse_state.options.constructs.tmark_brace {
        return State::Nok;
    }
    let bytes = tokenizer.parse_state.bytes;
    let index = tokenizer.point.index;
    let moustache = bytes.get(index + 1) == Some(&b'{');
    if !closes_on_line(bytes, index, moustache) {
        return State::Nok;
    }
    // `size`: marker length; `size_b`: inside quotes; `size_c`: marker bytes
    // consumed; `seen`: data event open; `marker`: attribute list flag.
    tokenizer.tokenize_state.size = if moustache { 2 } else { 1 };
    tokenizer.tokenize_state.size_b = 0;
    tokenizer.tokenize_state.size_c = 1;
    tokenizer.tokenize_state.seen = false;
    tokenizer.tokenize_state.marker = u8::from(looks_like_attributes(bytes, index));
    tokenizer.enter(Name::TmarkBrace);
    tokenizer.enter(Name::TmarkBraceMarker);
    tokenizer.consume();
    State::Next(StateName::TmarkBraceOpenMore)
}

/// In the opening marker, after `{`.
///
/// ```markdown
/// > | a {{ key }} c
///        ^
/// ```
pub fn open_more(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.size_c < tokenizer.tokenize_state.size {
        tokenizer.tokenize_state.size_c += 1;
        tokenizer.consume();
        State::Next(StateName::TmarkBraceOpenMore)
    } else {
        tokenizer.exit(Name::TmarkBraceMarker);
        State::Retry(StateName::TmarkBraceInside)
    }
}

/// Inside the braces.
///
/// ```markdown
/// > | a {aside}[b] c
///        ^^^^^
/// ```
pub fn inside(tokenizer: &mut Tokenizer) -> State {
    let moustache = tokenizer.tokenize_state.size == 2;
    match tokenizer.current {
        None | Some(b'\n') => {
            // Guarded by `closes_on_line`.
            reset(tokenizer);
            State::Nok
        }
        Some(b'}')
            if tokenizer.tokenize_state.size_b == 0
                && (!moustache
                    || tokenizer.parse_state.bytes.get(tokenizer.point.index + 1)
                        == Some(&b'}')) =>
        {
            if tokenizer.tokenize_state.seen {
                tokenizer.exit(Name::TmarkBraceData);
            }
            tokenizer.enter(Name::TmarkBraceMarker);
            tokenizer.tokenize_state.size_c = 1;
            tokenizer.consume();
            State::Next(StateName::TmarkBraceCloseMore)
        }
        Some(byte) => {
            if !tokenizer.tokenize_state.seen {
                tokenizer.enter(Name::TmarkBraceData);
                tokenizer.tokenize_state.seen = true;
            }
            if byte == b'"' && !moustache {
                tokenizer.tokenize_state.size_b ^= 1;
            }
            tokenizer.consume();
            State::Next(StateName::TmarkBraceInside)
        }
    }
}

/// In the closing marker, after `}`.
///
/// ```markdown
/// > | a {{ key }} c
///               ^
/// ```
pub fn close_more(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.size_c < tokenizer.tokenize_state.size {
        tokenizer.tokenize_state.size_c += 1;
        tokenizer.consume();
        State::Next(StateName::TmarkBraceCloseMore)
    } else {
        tokenizer.exit(Name::TmarkBraceMarker);
        tokenizer.exit(Name::TmarkBrace);
        State::Retry(StateName::TmarkBraceAfter)
    }
}

/// After the group: a role head may take an argument or bracket groups.
///
/// ```markdown
/// > | a {aside}[b] c
///              ^
/// ```
pub fn after(tokenizer: &mut Tokenizer) -> State {
    let head = tokenizer.tokenize_state.size == 1 && tokenizer.tokenize_state.marker == 0;
    reset(tokenizer);
    if !head {
        return State::Ok;
    }
    match tokenizer.current {
        Some(b'(') => {
            tokenizer.attempt(
                State::Next(StateName::TmarkBraceAfterArgument),
                State::Next(StateName::TmarkBraceAfterArgument),
            );
            State::Retry(StateName::TmarkArgumentStart)
        }
        Some(b'[') => {
            group_start(tokenizer);
            State::Ok
        }
        _ => State::Ok,
    }
}

/// After an optional argument.
pub fn after_argument(_tokenizer: &mut Tokenizer) -> State {
    State::Ok
}

fn reset(tokenizer: &mut Tokenizer) {
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.size_b = 0;
    tokenizer.tokenize_state.size_c = 0;
    tokenizer.tokenize_state.seen = false;
    tokenizer.tokenize_state.marker = 0;
}
