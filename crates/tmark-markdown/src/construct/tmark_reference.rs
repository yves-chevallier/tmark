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
use crate::util::normalize_identifier::normalize_identifier;
use crate::util::tmark::{line_end, word_char_before};

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
    if word_char_before(tokenizer.parse_state.bytes, tokenizer.point.index)
        || matches!(tokenizer.previous, Some(b'@' | b'/' | b':' | b'.' | b'-'))
    {
        return State::Nok;
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

/// Start of a Pandoc-style citation, at `[` followed by `@` or `-@`
/// (`[see @ein05, p. 33; -@AI2027]`), accepted for import (spec §Cite).
/// The whole bracketed text is the data; the lowering strips the `@`s.
///
/// ```markdown
/// > | See [@ein05, p. 33].
///         ^
/// ```
pub fn pandoc_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b'[') || !tokenizer.parse_state.options.constructs.tmark_reference
    {
        return State::Nok;
    }
    let bytes = tokenizer.parse_state.bytes;
    let index = tokenizer.point.index;
    let end = line_end(bytes, index);
    let rest = &bytes[index..end];
    // `[@` or `[-@`, anywhere in the items, and a closing `]` on the line.
    let has_at = rest
        .windows(2)
        .any(|w| w == b"[@" || w == b" @" || w == b"-@" || w == b";@");
    let Some(close) = rest.iter().position(|b| *b == b']') else {
        return State::Nok;
    };
    if !has_at
        || close < 3
        || !(rest.starts_with(b"[@")
            || rest.starts_with(b"[-@")
            || rest[1..close].contains(&b'@') && rest[1..close].iter().all(|b| *b != b'['))
    {
        return State::Nok;
    }
    // Only when the first item is a citation: `[see @key]` or `[@key]`.
    let first_item_end = rest[1..close]
        .iter()
        .position(|b| *b == b';')
        .map_or(close, |p| p + 1);
    if !rest[1..first_item_end].contains(&b'@') {
        return State::Nok;
    }
    tokenizer.tokenize_state.size = close + 1;
    tokenizer.enter(Name::TmarkReference);
    tokenizer.enter(Name::TmarkReferenceData);
    State::Retry(StateName::TmarkReferenceInside)
}

/// A citation key of the deprecated forms: a bare-reference key
/// (`[A-Za-z][\w:.-]*[A-Za-z0-9]`, `/` allowed after `doi:`) or a bare
/// DOI (`10.<digits>/…`), which the lowering prefixes with `doi:`.
fn is_citation_key(key: &[u8]) -> bool {
    if key.len() < 2 {
        return false;
    }
    let doi = key.starts_with(b"doi:") || (key.starts_with(b"10.") && key.contains(&b'/'));
    let first_ok = key[0].is_ascii_alphabetic() || doi;
    let last = key[key.len() - 1];
    first_ok
        && !matches!(last, b'.' | b',' | b';' | b':' | b'!' | b'?' | b'-' | b'/')
        && key.iter().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(b, b'_' | b':' | b'.' | b'-')
                || (doi && matches!(b, b'/' | b'(' | b')' | b'<' | b'>'))
        })
}

/// Start of a deprecated `[^key]` citation: a footnote call whose label
/// is a citation key with no `[^key]:` definition in the document (spec
/// §Cite, Appendix "Deprecation schedule"; decision X7). A real footnote
/// (defined label) or a numeric label is left to the GFM construct.
///
/// ```markdown
/// > | See [^ein05].
///         ^
/// ```
pub fn footnote_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b'[') || !tokenizer.parse_state.options.constructs.tmark_reference
    {
        return State::Nok;
    }
    let bytes = tokenizer.parse_state.bytes;
    let index = tokenizer.point.index;
    if bytes.get(index + 1) != Some(&b'^') {
        return State::Nok;
    }
    let end = line_end(bytes, index);
    let rest = &bytes[index..end];
    let Some(close) = rest.iter().position(|b| *b == b']') else {
        return State::Nok;
    };
    let key = &rest[2..close];
    if !is_citation_key(key) {
        return State::Nok;
    }
    let id = normalize_identifier(core::str::from_utf8(key).unwrap_or(""));
    if tokenizer.parse_state.gfm_footnote_definitions.contains(&id) {
        return State::Nok;
    }
    tokenizer.tokenize_state.size = close + 1;
    tokenizer.enter(Name::TmarkReference);
    tokenizer.enter(Name::TmarkReferenceData);
    State::Retry(StateName::TmarkReferenceInside)
}

/// Start of a deprecated `^[k1,k2]` citation group: `^[` followed by
/// comma-separated citation keys and `]` on the same line (decision X7).
/// `^[` never opens a caret superscript (`attention` refuses it), so a
/// group that is not a key list stays literal text.
///
/// ```markdown
/// > | tested,^[ein05,AI2027] and
///            ^
/// ```
pub fn caret_start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current != Some(b'^') || !tokenizer.parse_state.options.constructs.tmark_reference
    {
        return State::Nok;
    }
    let bytes = tokenizer.parse_state.bytes;
    let index = tokenizer.point.index;
    if bytes.get(index + 1) != Some(&b'[') {
        return State::Nok;
    }
    let end = line_end(bytes, index);
    let rest = &bytes[index..end];
    let Some(close) = rest.iter().position(|b| *b == b']') else {
        return State::Nok;
    };
    let inner = &rest[2..close];
    if inner.is_empty() || !inner.split(|b| *b == b',').all(is_citation_key) {
        return State::Nok;
    }
    tokenizer.tokenize_state.size = close + 1;
    tokenizer.enter(Name::TmarkReference);
    tokenizer.enter(Name::TmarkReferenceData);
    State::Retry(StateName::TmarkReferenceInside)
}
