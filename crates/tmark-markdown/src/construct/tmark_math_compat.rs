//! LaTeX-habit inline math, `\(…\)`, in the [text][] content type.
//!
//! Emits the same tokens as [raw (text)][crate::construct::raw_text] for
//! `$…$`, so the tree holds an ordinary inline math node and the printer
//! canonicalises the delimiters (spec §Math (inline)).
//!
//! ## Tokens
//!
//! * [`MathText`][Name::MathText]
//! * [`MathTextSequence`][Name::MathTextSequence]
//! * [`MathTextData`][Name::MathTextData]
//!
//! [text]: crate::construct::text

use crate::event::Name;
use crate::state::{Name as StateName, State};
use crate::tokenizer::Tokenizer;
use crate::util::tmark::line_end;

/// Start, at `\`.
///
/// ```markdown
/// > | a \(x\) b
///       ^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b'\\')
        || !tokenizer.parse_state.options.constructs.tmark_math_compat
    {
        return State::Nok;
    }
    let bytes = tokenizer.parse_state.bytes;
    let index = tokenizer.point.index;
    if bytes.get(index + 1) != Some(&b'(') {
        return State::Nok;
    }
    let end = line_end(bytes, index);
    let mut i = index + 2;
    let mut found = false;
    while i + 1 < end {
        if bytes[i] == b'\\' && bytes[i + 1] == b')' {
            found = i > index + 2;
            break;
        }
        i += 1;
    }
    if !found {
        return State::Nok;
    }
    tokenizer.enter(Name::MathText);
    tokenizer.enter(Name::MathTextSequence);
    tokenizer.consume();
    State::Next(StateName::TmarkMathCompatOpen)
}

/// After `\`, at `(`.
pub fn open(tokenizer: &mut Tokenizer) -> State {
    tokenizer.consume();
    tokenizer.exit(Name::MathTextSequence);
    tokenizer.enter(Name::MathTextData);
    State::Next(StateName::TmarkMathCompatInside)
}

/// Inside the math.
///
/// ```markdown
/// > | a \(x\) b
///         ^
/// ```
pub fn inside(tokenizer: &mut Tokenizer) -> State {
    let bytes = tokenizer.parse_state.bytes;
    let index = tokenizer.point.index;
    if tokenizer.current == Some(b'\\') && bytes.get(index + 1) == Some(&b')') {
        tokenizer.exit(Name::MathTextData);
        tokenizer.enter(Name::MathTextSequence);
        tokenizer.consume();
        State::Next(StateName::TmarkMathCompatClose)
    } else {
        tokenizer.consume();
        State::Next(StateName::TmarkMathCompatInside)
    }
}

/// After `\`, at `)`.
pub fn close(tokenizer: &mut Tokenizer) -> State {
    tokenizer.consume();
    tokenizer.exit(Name::MathTextSequence);
    tokenizer.exit(Name::MathText);
    State::Ok
}
