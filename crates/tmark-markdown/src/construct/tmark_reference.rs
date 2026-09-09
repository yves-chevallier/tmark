//! TMark reference or citation (`@` refers), in the [text][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! tmark_reference ::= '@' (key | '[' 1*(char - ']' - eol) ']')
//! key             ::= alpha *(alnum | '_' | ':' | '.' | '-') alnum
//!                   | ('doi:' | 'http://' | 'https://') 1*(char - space - '[' - ']' - '(' - ')')
//! ```
//!
//! The X4 guard: `@` never fires after a word byte or after one of
//! `@ / : . -`, so e-mail addresses and URLs stay literal (spec §Lexical
//! grammar). A `doi:` key or a URL may hold a `/`
//! (`design/12-spec-challenges.md`, C3); trailing sentence punctuation
//! stays out of every key.
//!
//! ## Tokens
//!
//! * [`TmarkReference`][Name::TmarkReference]
//! * [`TmarkReferenceMarker`][Name::TmarkReferenceMarker]
//! * [`TmarkReferenceData`][Name::TmarkReferenceData]
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;
use crate::util::tmark::{is_word_byte, line_end};

/// Length of the data after `@` at `index + 1`, or 0 when there is none.
fn data_len(bytes: &[u8], index: usize) -> usize {
    let start = index + 1;
    let end = line_end(bytes, index);
    let rest = &bytes[start..end];
    if rest.first() == Some(&b'[') {
        return match rest.iter().position(|b| *b == b']') {
            Some(close) if close > 1 => close + 1,
            _ => 0,
        };
    }
    if !rest.first().is_some_and(u8::is_ascii_alphabetic) {
        return 0;
    }
    let url =
        rest.starts_with(b"doi:") || rest.starts_with(b"http://") || rest.starts_with(b"https://");
    let mut len = 0;
    while len < rest.len() {
        let byte = rest[len];
        let ok = if url {
            !byte.is_ascii_whitespace() && !matches!(byte, b'[' | b']' | b'(' | b')' | b'<' | b'>')
        } else {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'.' | b'-')
        };
        if !ok {
            break;
        }
        len += 1;
    }
    // The key ends on an alphanumeric (plain keys) or on a non-punctuation
    // byte (URL-shaped keys): sentence punctuation stays out.
    while len > 0 {
        let last = rest[len - 1];
        let keep = if url {
            !matches!(last, b'.' | b',' | b';' | b':' | b'!' | b'?')
        } else {
            last.is_ascii_alphanumeric()
        };
        if keep {
            break;
        }
        len -= 1;
    }
    if len < 2 {
        0
    } else {
        len
    }
}

/// Start of a reference.
///
/// ```markdown
/// > | See @sec:intro.
///         ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b'@') || !tokenizer.parse_state.options.constructs.tmark_reference
    {
        return State::Nok;
    }
    if let Some(previous) = tokenizer.previous {
        if is_word_byte(previous) || matches!(previous, b'@' | b'/' | b':' | b'.' | b'-') {
            return State::Nok;
        }
    }
    let len = data_len(tokenizer.parse_state.bytes, tokenizer.point.index);
    if len == 0 {
        return State::Nok;
    }
    tokenizer.tokenize_state.size = len;
    tokenizer.enter(Name::TmarkReference);
    tokenizer.enter(Name::TmarkReferenceMarker);
    tokenizer.consume();
    tokenizer.exit(Name::TmarkReferenceMarker);
    tokenizer.enter(Name::TmarkReferenceData);
    State::Next(StateName::TmarkReferenceInside)
}

/// In the key or the bracketed items.
///
/// ```markdown
/// > | See @sec:intro.
///          ^^^^^^^^^
/// ```
pub fn inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.tokenize_state.size > 0 {
        tokenizer.tokenize_state.size -= 1;
        tokenizer.consume();
        State::Next(StateName::TmarkReferenceInside)
    } else {
        tokenizer.exit(Name::TmarkReferenceData);
        tokenizer.exit(Name::TmarkReference);
        State::Ok
    }
}
