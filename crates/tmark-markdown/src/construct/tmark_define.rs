//! TMark standalone definition (`#` defines), in the [text][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! tmark_define ::= '#' ('[' | '(' prefix ':' key ')' | '{' prefix ':' key '}')
//! prefix       ::= alpha *(alnum | '_' | '-')
//! key          ::= 1*(alnum | '_' | '.' | '-')
//! ```
//!
//! `#[` opens a bracket group (an index entry, content parsed as text);
//! `#(prefix:key)` is a counter item (argument); `#{prefix:key}` is the
//! deprecated spelling of the counter item. `\#` escapes (spec X5). The
//! construct never fires after a word byte, so `C#` and `#1` stay literal.
//!
//! ## Tokens
//!
//! * [`TmarkDefine`][Name::TmarkDefine]
//! * [`TmarkDefineMarker`][Name::TmarkDefineMarker]
//! * [`TmarkArgument`][Name::TmarkArgument] (for `(` and `{`)
//!
//! [text]: crate::construct::text

use crate::construct::tmark_brace::group_start;
use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;
use crate::util::tmark::{is_ident_byte, is_ident_start, is_word_byte};

/// Whether `bytes[index..]` is `prefix:key` followed by `close`.
fn counter_shape(bytes: &[u8], index: usize, close: u8) -> bool {
    let mut i = index;
    if !bytes.get(i).is_some_and(|b| is_ident_start(*b)) {
        return false;
    }
    while bytes.get(i).is_some_and(|b| is_ident_byte(*b)) {
        i += 1;
    }
    if bytes.get(i) != Some(&b':') {
        return false;
    }
    i += 1;
    let key_start = i;
    while bytes
        .get(i)
        .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
    {
        i += 1;
    }
    i > key_start && bytes.get(i) == Some(&close)
}

/// Start of a definition, at `#`.
///
/// ```markdown
/// > | a #[term] b
///       ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b'#') || !tokenizer.parse_state.options.constructs.tmark_define {
        return State::Nok;
    }
    if let Some(previous) = tokenizer.previous {
        if is_word_byte(previous) || previous == b'\\' {
            return State::Nok;
        }
    }
    let bytes = tokenizer.parse_state.bytes;
    let index = tokenizer.point.index;
    let ok = match bytes.get(index + 1) {
        Some(b'[') => bytes
            .get(index + 2)
            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b']'),
        Some(b'(') => counter_shape(bytes, index + 2, b')'),
        Some(b'{') => counter_shape(bytes, index + 2, b'}'),
        _ => false,
    };
    if !ok {
        return State::Nok;
    }
    tokenizer.enter(Name::TmarkDefine);
    tokenizer.enter(Name::TmarkDefineMarker);
    tokenizer.consume();
    State::Next(StateName::TmarkDefineAfterMarker)
}

/// After `#`, at `[`, `(` or `{`.
///
/// ```markdown
/// > | a #[term] b
///        ^
/// ```
pub fn after_marker(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::TmarkDefineMarker);
    match tokenizer.current {
        Some(b'[') => {
            tokenizer.exit(Name::TmarkDefine);
            group_start(tokenizer);
            State::Ok
        }
        _ => {
            tokenizer.attempt(State::Next(StateName::TmarkDefineAfterArgument), State::Nok);
            State::Retry(StateName::TmarkArgumentStart)
        }
    }
}

/// After the argument of a counter item.
///
/// ```markdown
/// > | #(fw:boot-loop) The firmware…
///                    ^
/// ```
pub fn after_argument(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::TmarkDefine);
    State::Ok
}
