//! TMark container directive, in the [flow][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! tmark_container ::= fence_open *(eol content_line) [eol fence_close]
//! fence_open      ::= 3*marker *space name *(char - eol)
//! fence_close     ::= 3*marker *space
//! marker          ::= ':' | '/'
//! ```
//!
//! `:::` is the canonical fence (spec §Container directives); `///` is the
//! deprecated PyMdownX block fence. Nesting is by fence length: a closing
//! fence needs at least as many markers as the opening one. The content
//! lines are kept raw, like fenced code: the lowering re-parses them as a
//! document, with the chunk positions giving the mapping back to the source.
//! A container left open closes at the end of the document (the lowering
//! reports it).
//!
//! ## Tokens
//!
//! * [`TmarkContainer`][Name::TmarkContainer]
//! * [`TmarkContainerFence`][Name::TmarkContainerFence]
//! * [`TmarkContainerFenceSequence`][Name::TmarkContainerFenceSequence]
//! * [`TmarkContainerFenceInfo`][Name::TmarkContainerFenceInfo]
//! * [`TmarkContainerChunk`][Name::TmarkContainerChunk]
//!
//! [flow]: crate::construct::flow

use crate::construct::partial_space_or_tab::{space_or_tab, space_or_tab_min_max};
use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;
use crate::util::{
    constant::TAB_SIZE,
    slice::{Position, Slice},
    tmark::is_ident_start,
};

const FENCE_MIN: usize = 3;

/// Start.
///
/// ```markdown
/// > | ::: figure {cols=2}
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if !tokenizer.parse_state.options.constructs.tmark_container {
        return State::Nok;
    }
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(
            State::Next(StateName::TmarkContainerBeforeSequenceOpen),
            State::Nok,
        );
        return State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1));
    }
    if matches!(tokenizer.current, Some(b':' | b'/')) {
        return State::Retry(StateName::TmarkContainerBeforeSequenceOpen);
    }
    State::Nok
}

/// In opening fence, after prefix, at sequence.
pub fn before_sequence_open(tokenizer: &mut Tokenizer) -> State {
    let mut prefix = 0;
    if let Some(event) = tokenizer.events.last() {
        if event.name == Name::SpaceOrTab {
            prefix = Slice::from_position(
                tokenizer.parse_state.bytes,
                &Position::from_exit_event(&tokenizer.events, tokenizer.events.len() - 1),
            )
            .len();
        }
    }
    match tokenizer.current {
        Some(b':' | b'/') => {
            tokenizer.tokenize_state.marker = tokenizer.current.unwrap();
            tokenizer.tokenize_state.size_c = prefix;
            tokenizer.tokenize_state.size = 0;
            tokenizer.enter(Name::TmarkContainer);
            tokenizer.enter(Name::TmarkContainerFence);
            tokenizer.enter(Name::TmarkContainerFenceSequence);
            State::Retry(StateName::TmarkContainerSequenceOpen)
        }
        _ => State::Nok,
    }
}

/// In opening fence sequence.
///
/// ```markdown
/// > | ::: figure
///     ^^^
/// ```
pub fn sequence_open(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        State::Next(StateName::TmarkContainerSequenceOpen)
    } else if tokenizer.tokenize_state.size < FENCE_MIN {
        reset(tokenizer);
        State::Nok
    } else {
        tokenizer.exit(Name::TmarkContainerFenceSequence);
        if matches!(tokenizer.current, Some(b'\t' | b' ')) {
            tokenizer.attempt(State::Next(StateName::TmarkContainerInfoBefore), State::Nok);
            State::Retry(space_or_tab(tokenizer))
        } else {
            State::Retry(StateName::TmarkContainerInfoBefore)
        }
    }
}

/// In opening fence, at the name.
///
/// ```markdown
/// > | ::: figure
///         ^
/// ```
pub fn info_before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        Some(byte) if is_ident_start(byte) => {
            tokenizer.enter(Name::TmarkContainerFenceInfo);
            State::Retry(StateName::TmarkContainerInfo)
        }
        _ => {
            // An opening fence needs a name; a bare `:::` here is text.
            reset(tokenizer);
            State::Nok
        }
    }
}

/// In the info.
pub fn info(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::TmarkContainerFenceInfo);
            tokenizer.exit(Name::TmarkContainerFence);
            // Do not form containers.
            tokenizer.concrete = true;
            tokenizer.check(
                State::Next(StateName::TmarkContainerAtNonLazyBreak),
                State::Next(StateName::TmarkContainerAfter),
            );
            State::Retry(StateName::NonLazyContinuationStart)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::TmarkContainerInfo)
        }
    }
}

/// At an eol/eof in content, before a non-lazy closing fence or content.
pub fn at_non_lazy_break(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(
        State::Next(StateName::TmarkContainerAfter),
        State::Next(StateName::TmarkContainerContentBefore),
    );
    tokenizer.enter(Name::LineEnding);
    tokenizer.consume();
    tokenizer.exit(Name::LineEnding);
    State::Next(StateName::TmarkContainerCloseStart)
}

/// Before closing fence, at optional whitespace.
pub fn close_start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::TmarkContainerFence);
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(
            State::Next(StateName::TmarkContainerBeforeSequenceClose),
            State::Nok,
        );
        State::Retry(space_or_tab_min_max(tokenizer, 0, TAB_SIZE - 1))
    } else {
        State::Retry(StateName::TmarkContainerBeforeSequenceClose)
    }
}

/// In closing fence, after optional whitespace, at sequence.
pub fn before_sequence_close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.enter(Name::TmarkContainerFenceSequence);
        State::Retry(StateName::TmarkContainerSequenceClose)
    } else {
        State::Nok
    }
}

/// In closing fence sequence.
///
/// ```markdown
///   | ::: figure
///   | …
/// > | :::
///     ^^^
/// ```
pub fn sequence_close(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(tokenizer.tokenize_state.marker) {
        tokenizer.tokenize_state.size_b += 1;
        tokenizer.consume();
        State::Next(StateName::TmarkContainerSequenceClose)
    } else if tokenizer.tokenize_state.size_b >= tokenizer.tokenize_state.size {
        tokenizer.tokenize_state.size_b = 0;
        tokenizer.exit(Name::TmarkContainerFenceSequence);
        if matches!(tokenizer.current, Some(b'\t' | b' ')) {
            tokenizer.attempt(
                State::Next(StateName::TmarkContainerAfterSequenceClose),
                State::Nok,
            );
            State::Retry(space_or_tab(tokenizer))
        } else {
            State::Retry(StateName::TmarkContainerAfterSequenceClose)
        }
    } else {
        tokenizer.tokenize_state.size_b = 0;
        State::Nok
    }
}

/// After closing fence sequence, after optional whitespace.
pub fn after_sequence_close(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::TmarkContainerFence);
            State::Ok
        }
        _ => State::Nok,
    }
}

/// Before content, at eol.
pub fn content_before(tokenizer: &mut Tokenizer) -> State {
    tokenizer.enter(Name::LineEnding);
    tokenizer.consume();
    tokenizer.exit(Name::LineEnding);
    State::Next(StateName::TmarkContainerContentStart)
}

/// Before content, after optional prefix.
pub fn content_start(tokenizer: &mut Tokenizer) -> State {
    if matches!(tokenizer.current, Some(b'\t' | b' ')) {
        tokenizer.attempt(
            State::Next(StateName::TmarkContainerBeforeContentChunk),
            State::Nok,
        );
        State::Retry(space_or_tab_min_max(
            tokenizer,
            0,
            tokenizer.tokenize_state.size_c,
        ))
    } else {
        State::Retry(StateName::TmarkContainerBeforeContentChunk)
    }
}

/// Before content chunk.
pub fn before_content_chunk(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.check(
                State::Next(StateName::TmarkContainerAtNonLazyBreak),
                State::Next(StateName::TmarkContainerAfter),
            );
            State::Retry(StateName::NonLazyContinuationStart)
        }
        _ => {
            tokenizer.enter(Name::TmarkContainerChunk);
            State::Retry(StateName::TmarkContainerContentChunk)
        }
    }
}

/// In content chunk.
pub fn content_chunk(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None | Some(b'\n') => {
            tokenizer.exit(Name::TmarkContainerChunk);
            State::Retry(StateName::TmarkContainerBeforeContentChunk)
        }
        _ => {
            tokenizer.consume();
            State::Next(StateName::TmarkContainerContentChunk)
        }
    }
}

/// After the container.
pub fn after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::TmarkContainer);
    reset(tokenizer);
    // Feel free to interrupt.
    tokenizer.interrupt = false;
    // No longer concrete.
    tokenizer.concrete = false;
    State::Ok
}

fn reset(tokenizer: &mut Tokenizer) {
    tokenizer.tokenize_state.marker = 0;
    tokenizer.tokenize_state.size = 0;
    tokenizer.tokenize_state.size_b = 0;
    tokenizer.tokenize_state.size_c = 0;
}
