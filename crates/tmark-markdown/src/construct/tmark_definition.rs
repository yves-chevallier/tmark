//! TMark definition list item, in the [flow][] content type.
//!
//! ## Grammar
//!
//! ```bnf
//! tmark_definition ::= ':' 1*3space 1*(char - eol) eol body
//! ```
//!
//! PHP-Markdown-Extra `def_list` (spec §DefinitionList): the preceding
//! paragraph is the term, the marker line and the indented lines that follow
//! are the definition. The construct may interrupt a paragraph, which is how
//! the term gets its definition. Pairing term and definitions is done in the
//! lowering.
//!
//! ## Tokens
//!
//! * [`TmarkDefinition`][Name::TmarkDefinition]
//! * [`TmarkDefinitionMarker`][Name::TmarkDefinitionMarker]
//! * [`TmarkDefinitionChunk`][Name::TmarkDefinitionChunk]
//!
//! [flow]: crate::construct::flow

use crate::construct::partial_space_or_tab::space_or_tab_min_max;
use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;

/// Start, at `:`.
///
/// ```markdown
/// > | :   Definition of the term.
///     ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b':') || !tokenizer.parse_state.options.constructs.tmark_definition
    {
        return State::Nok;
    }
    let bytes = tokenizer.parse_state.bytes;
    let mut i = tokenizer.point.index + 1;
    let mut spaces = 0;
    while matches!(bytes.get(i), Some(b' ')) && spaces < 3 {
        spaces += 1;
        i += 1;
    }
    let content = matches!(bytes.get(i), Some(b) if !b.is_ascii_whitespace());
    if spaces == 0 || !content {
        return State::Nok;
    }
    tokenizer.enter(Name::TmarkDefinition);
    tokenizer.enter(Name::TmarkDefinitionMarker);
    tokenizer.consume();
    tokenizer.exit(Name::TmarkDefinitionMarker);
    State::Next(StateName::TmarkDefinitionBeforeContent)
}

/// After `:`, at the spaces.
pub fn before_content(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::TmarkDefinitionContent), State::Nok);
    State::Retry(space_or_tab_min_max(tokenizer, 1, 3))
}

/// At the first byte of the definition.
///
/// ```markdown
/// > | :   Definition of the term.
///         ^
/// ```
pub fn content(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.token_1 = Name::TmarkDefinitionChunk;
    tokenizer.attempt(State::Next(StateName::TmarkDefinitionAfter), State::Nok);
    State::Retry(StateName::TmarkBodyAtBreak)
}

/// After the body.
pub fn after(tokenizer: &mut Tokenizer) -> State {
    tokenizer.exit(Name::TmarkDefinition);
    tokenizer.tokenize_state.token_1 = Name::Data;
    // Feel free to interrupt.
    tokenizer.interrupt = false;
    State::Ok
}
