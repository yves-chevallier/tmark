//! Byte-level predicates shared by the TMark constructs.
//!
//! They are public so that `tmark-syntax` applies the same rules when it
//! lowers the tree (one definition of "what an attribute list looks like").

/// Whether a byte belongs to a word for the X4 guard (`@` never fires inside
/// a word). Non-ASCII bytes count as word bytes.
pub fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

/// Whether a byte can start an identifier (`[A-Za-z]`).
pub fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic()
}

/// Whether a byte can continue an identifier (`[A-Za-z0-9_-]`).
pub fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

/// Whether `bytes[index]` is a `{` that starts an attribute list: after
/// optional blanks comes `#x`, `.x`, or `key=` (spec §Attributes).
pub fn looks_like_attributes(bytes: &[u8], index: usize) -> bool {
    if bytes.get(index) != Some(&b'{') {
        return false;
    }
    let mut i = index + 1;
    while matches!(bytes.get(i), Some(b' ' | b'\t')) {
        i += 1;
    }
    match bytes.get(i) {
        Some(b'#' | b'.') => {
            matches!(bytes.get(i + 1), Some(b) if !b.is_ascii_whitespace() && *b != b'}')
        }
        Some(b) if is_ident_byte(*b) => {
            let mut j = i;
            while matches!(bytes.get(j), Some(b) if is_ident_byte(*b)) {
                j += 1;
            }
            bytes.get(j) == Some(&b'=')
        }
        _ => false,
    }
}

/// Index of the end of the current line: the `\n`, or the end of input.
pub fn line_end(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }
    index
}
