//! TMark parenthesised argument, after a role head or after `#`.
//!
//! ## Grammar
//!
//! ```bnf
//! tmark_argument ::= '(' *(char - '(' - ')' - eol | tmark_argument) ')'
//! ```
//!
//! Parentheses nest when balanced, as in link destinations (spec §Roles).
//! The deprecated counter marker `#{prefix:key}` reuses the construct with
//! `{` and `}` as markers; nothing nests there.
//!
//! ## Tokens
//!
//! * [`TmarkArgument`][Name::TmarkArgument]
//! * [`TmarkArgumentMarker`][Name::TmarkArgumentMarker]
//! * [`TmarkArgumentData`][Name::TmarkArgumentData]

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;
use crate::util::tmark::line_end;

/// Whether an argument opened at `index` closes on the same line, with
/// balanced markers and at least one byte inside.
fn closes_on_line(bytes: &[u8], index: usize, open: u8, close: u8) -> bool {
    let end = line_end(bytes, index);
    let mut depth = 0usize;
    let mut i = index + 1;
    while i < end {
        let byte = bytes[i];
        if byte == open && open != close {
            depth += 1;
        } else if byte == close {
            if depth == 0 {
                return i > index + 1;
            }
            depth -= 1;
        }
        i += 1;
    }
    false
}

/// Start of an argument.
///
/// ```markdown
/// > | a {raw latex}(\clearpage) c
///                  ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    let open = match tokenizer.current {
        Some(b'(') => b'(',
        Some(b'{') => b'{',
        _ => return State::Nok,
    };
    let close = if open == b'(' { b')' } else { b'}' };
    if !closes_on_line(
        tokenizer.parse_state.bytes,
        tokenizer.point.index,
        open,
        close,
    ) {
        return State::Nok;
    }
    tokenizer.tokenize_state.marker_b = open;
    tokenizer.tokenize_state.size = 0;
    tokenizer.enter(Name::TmarkArgument);
    tokenizer.enter(Name::TmarkArgumentMarker);
    tokenizer.consume();
    tokenizer.exit(Name::TmarkArgumentMarker);
    tokenizer.enter(Name::TmarkArgumentData);
    State::Next(StateName::TmarkArgumentInside)
}

/// Inside an argument.
///
/// ```markdown
/// > | a {raw latex}(\clearpage) c
///                   ^^^^^^^^^^
/// ```
pub fn inside(tokenizer: &mut Tokenizer) -> State {
    let open = tokenizer.tokenize_state.marker_b;
    let close = if open == b'(' { b')' } else { b'}' };
    match tokenizer.current {
        None | Some(b'\n') => {
            // Guarded by `closes_on_line`.
            tokenizer.tokenize_state.marker_b = 0;
            tokenizer.tokenize_state.size = 0;
            State::Nok
        }
        Some(byte) if byte == close && tokenizer.tokenize_state.size == 0 => {
            tokenizer.exit(Name::TmarkArgumentData);
            tokenizer.enter(Name::TmarkArgumentMarker);
            tokenizer.consume();
            tokenizer.exit(Name::TmarkArgumentMarker);
            tokenizer.exit(Name::TmarkArgument);
            tokenizer.tokenize_state.marker_b = 0;
            State::Ok
        }
        Some(byte) => {
            if byte == open && open != close {
                tokenizer.tokenize_state.size += 1;
            } else if byte == close {
                tokenizer.tokenize_state.size -= 1;
            }
            tokenizer.consume();
            State::Next(StateName::TmarkArgumentInside)
        }
    }
}
