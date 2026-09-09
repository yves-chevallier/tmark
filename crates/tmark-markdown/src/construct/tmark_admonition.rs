//! TMark admonition in the PyMdownX spelling, in the [flow][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! tmark_admonition ::= ('!!!' | '???' | '???+') 1*space info eol body
//! info             ::= 1*(char - eol)
//! ```
//!
//! The body is the indented block that follows (see
//! [`partial_tmark_body`][crate::construct::partial_tmark_body]). The info
//! is kept raw (`type class… "Title"`); `tmark-syntax` parses it. The body
//! is not tokenised here: the lowering re-parses it as a document, with the
//! chunk positions giving the mapping back to the source.
//!
//! ## Tokens
//!
//! * [`TmarkAdmonition`][Name::TmarkAdmonition]
//! * [`TmarkAdmonitionMarker`][Name::TmarkAdmonitionMarker]
//! * [`TmarkAdmonitionInfo`][Name::TmarkAdmonitionInfo]
//! * [`TmarkAdmonitionChunk`][Name::TmarkAdmonitionChunk]
//!
//! [flow]: crate::construct::flow

use crate::construct::partial_space_or_tab::space_or_tab;
use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start, at `!` or `?`.
///
/// ```markdown
/// > | !!! note "Title"
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if !tokenizer.parse_state.options.constructs.tmark_admonition {
        return State::Nok;
    }
    match tokenizer.current {
        Some(b'!' | b'?') => {
            tokenizer.tokenize_state.marker = tokenizer.current.unwrap();
            tokenizer.tokenize_state.size = 0;
            tokenizer.tokenize_state.seen = false;
            tokenizer.enter(Name::TmarkAdmonition);
            tokenizer.enter(Name::TmarkAdmonitionMarker);
            State::Retry(StateName::TmarkAdmonitionSequence)
        }
        _ => State::Nok,
    }
}

/// In the marker.
///
/// ```markdown
/// > | !!! note "Title"
///     ^^^
/// ```
pub fn sequence(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        State::Next(StateName::TmarkAdmonitionSequence)
    } else if tokenizer.tokenize_state.size != 3 {
        reset(tokenizer);
        State::Nok
    } else if tokenizer.tokenize_state.marker == b'?'
        && tokenizer.current == Some(b'+')
        && !tokenizer.tokenize_state.seen
    {
        tokenizer.tokenize_state.seen = true;
        tokenizer.consume();
        State::Next(StateName::TmarkAdmonitionSequence)
    } else {
        tokenizer.exit(Name::TmarkAdmonitionMarker);
        State::Retry(StateName::TmarkAdmonitionInfoBefore)
    }
}

/// After the marker, at the whitespace before the info.
///
/// ```markdown
/// > | !!! note "Title"
///        ^
/// ```
pub fn info_before(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.tokenize_state.size_b = 0;
        tokenizer.attempt(State::Next(StateName::TmarkAdmonitionInfo), State::Nok);
        State::Retry(space_or_tab(tokenizer))
    } else {
        reset(tokenizer);
        State::Nok
    }
}

/// In the info.
///
/// ```markdown
/// > | !!! note "Title"
///         ^^^^^^^^^^^^
/// ```
pub fn info(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            if tokenizer.tokenize_state.size_b == 0 {
                reset(tokenizer);
                return State::Nok;
            }
            tokenizer.exit(Name::TmarkAdmonitionInfo);
            tokenizer.tokenize_state.token_1 = Name::TmarkAdmonitionChunk;
            tokenizer.attempt(State::Next(StateName::TmarkAdmonitionAfter), State::Nok);
            State::Retry(StateName::TmarkBodyAtBreak)
        }
        _ => {
            if tokenizer.tokenize_state.size_b == 0 {
                tokenizer.enter(Name::TmarkAdmonitionInfo);
                tokenizer.tokenize_state.size_b = 1;
            }
            tokenizer.consume();
            State::Next(StateName::TmarkAdmonitionInfo)
        }
    }
}

/// After the body.
pub fn after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::TmarkAdmonition);
    reset(tokenizer);
    // Feel free to interrupt.
    tokenizer.interrupt = false;
    State::Ok
}

fn reset(tokenizer: &mut Tokenizer) {
    tokenizer.tokenize_state.marker = 0;
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.size_b = 0;
    tokenizer.tokenize_state.seen = false;
    tokenizer.tokenize_state.token_1 = Name::Data;
}
