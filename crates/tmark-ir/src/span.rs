//! Source identity: files, byte spans, node ids, and the line index.
//!
//! Design: `design/03-ir.md` §Identity and spans. Spec §Round-trip and source
//! spans: every node carries the span of the *whole* construct, including its
//! sugar; nodes never store line and column, [`LineIndex`] derives them.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Identifies one source file of a build. `0` is the main document; includes
/// get the following ids in the order the loader meets them.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(transparent)]
pub struct FileId(pub u32);

/// Dense, document-ordered node identifier. Passes that rebuild a document
/// keep the ids of nodes they did not create (design §Identity).
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(transparent)]
pub struct NodeId(pub u32);

/// A half-open byte range `start..end` in `file`.
///
/// Serialised as the array `[file, start, end]`.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(from = "(u32, u32, u32)", into = "(u32, u32, u32)")]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        Span { file, start, end }
    }

    /// Length in bytes.
    pub fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.end <= self.start
    }

    /// `true` when `offset` lies inside the span (`start <= offset < end`).
    pub fn contains(self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Smallest span covering both. Both must belong to the same file.
    pub fn join(self, other: Span) -> Span {
        debug_assert_eq!(self.file, other.file);
        Span {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl From<(u32, u32, u32)> for Span {
    fn from((file, start, end): (u32, u32, u32)) -> Self {
        Span::new(FileId(file), start, end)
    }
}

impl From<Span> for (u32, u32, u32) {
    fn from(span: Span) -> Self {
        (span.file.0, span.start, span.end)
    }
}

impl JsonSchema for Span {
    fn schema_name() -> String {
        "Span".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = <[u32; 3]>::json_schema(gen);
        if let schemars::schema::Schema::Object(obj) = &mut schema {
            obj.metadata().description = Some("Byte span as [file, start, end]".into());
        }
        schema
    }
}

/// The source range of one token inside a node: a reference key, an
/// attribute id, a counter key. Design `03-ir.md` §Identity and spans
/// ("sub-spans that tools need are stored as fields of the node"); spec
/// §Round-trip and source spans. Serialised like [`Span`]. Equality is
/// always `true`, as for `Meta`, so that derived node equality stays
/// structural; compare the inner `Span` for positions.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubSpan(pub Span);

impl SubSpan {
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        SubSpan(Span::new(file, start, end))
    }
}

impl PartialEq for SubSpan {
    fn eq(&self, _: &SubSpan) -> bool {
        true
    }
}

impl Eq for SubSpan {}

impl JsonSchema for SubSpan {
    fn schema_name() -> String {
        "SubSpan".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = <[u32; 3]>::json_schema(gen);
        if let schemars::schema::Schema::Object(obj) = &mut schema {
            obj.metadata().description =
                Some("Byte span of a token inside its node, as [file, start, end]".into());
        }
        schema
    }
}

/// Zero-based line and byte column.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LineCol {
    pub line: u32,
    /// Byte offset from the start of the line.
    pub col: u32,
}

/// Zero-based line and UTF-16 code-unit column, the coordinate system of the
/// Language Server Protocol (design `08-lsp.md` §Positions).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LineColUtf16 {
    pub line: u32,
    /// Offset in UTF-16 code units from the start of the line.
    pub col: u32,
}

/// A non-ASCII character of the text: where it sits and how wide it is.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct WideChar {
    line: u32,
    /// Byte column of the character in its line.
    col: u32,
    /// UTF-8 length, 2 to 4.
    len: u8,
}

impl WideChar {
    fn utf16_len(self) -> u32 {
        if self.len == 4 {
            2
        } else {
            1
        }
    }
}

/// Maps byte offsets to lines and columns and back. Built once per text; the
/// text itself is not kept.
///
/// Line endings are `\n`, `\r\n` and a lone `\r` (CommonMark §Characters and
/// lines). Offsets past the end of the text clamp to the end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    /// Byte offset of the first character of every line; `line_starts[0] == 0`.
    line_starts: Vec<u32>,
    /// Byte offset of the line terminator of every line (the end of the
    /// text for the last one); `line_ends[i] >= line_starts[i]`.
    line_ends: Vec<u32>,
    /// Non-ASCII characters, sorted by `(line, col)`.
    wide: Vec<WideChar>,
    /// Total length in bytes.
    len: u32,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        let mut line_ends = Vec::new();
        let mut wide = Vec::new();
        let mut line = 0u32;
        let mut line_start = 0u32;
        let mut prev_cr = false;
        for (offset, ch) in text.char_indices() {
            let offset = offset as u32;
            let after = offset + ch.len_utf8() as u32;
            match ch {
                '\n' => {
                    if !prev_cr {
                        line += 1;
                        line_ends.push(offset);
                    }
                    // `\r\n`: the line already started after the `\r`; fix it up.
                    if prev_cr {
                        *line_starts.last_mut().expect("at least one line") = after;
                    } else {
                        line_starts.push(after);
                    }
                    line_start = after;
                }
                '\r' => {
                    line += 1;
                    line_ends.push(offset);
                    line_starts.push(after);
                    line_start = after;
                }
                c if !c.is_ascii() => wide.push(WideChar {
                    line,
                    col: offset - line_start,
                    len: c.len_utf8() as u8,
                }),
                _ => {}
            }
            prev_cr = ch == '\r';
        }
        line_ends.push(text.len() as u32);
        LineIndex {
            line_starts,
            line_ends,
            wide,
            len: text.len() as u32,
        }
    }

    /// Number of lines, at least 1.
    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// Byte offset where `line` starts, or `None` past the last line.
    pub fn line_start(&self, line: u32) -> Option<u32> {
        self.line_starts.get(line as usize).copied()
    }

    /// Byte offset of the line terminator of `line` (the end of the text
    /// for the last line), or `None` past the last line.
    pub fn line_end(&self, line: u32) -> Option<u32> {
        self.line_ends.get(line as usize).copied()
    }

    /// Total length of the text in bytes.
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Line and byte column of a byte offset.
    pub fn line_col(&self, offset: u32) -> LineCol {
        let offset = offset.min(self.len);
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        LineCol {
            line: line as u32,
            col: offset - self.line_starts[line],
        }
    }

    /// Byte offset of a line and byte column. A column past the end of the
    /// line clamps to the line's end, before its terminator (LSP: "defaults
    /// back to the line length"); a line past the text clamps to its end.
    pub fn offset(&self, pos: LineCol) -> u32 {
        let (Some(start), Some(end)) = (self.line_start(pos.line), self.line_end(pos.line)) else {
            return self.len;
        };
        (start + pos.col).min(end)
    }

    fn wide_in_line(&self, line: u32) -> &[WideChar] {
        let lo = self.wide.partition_point(|w| w.line < line);
        let hi = self.wide.partition_point(|w| w.line <= line);
        &self.wide[lo..hi]
    }

    /// Converts a byte column to a UTF-16 column. A column inside a
    /// multibyte character snaps to that character's start.
    pub fn to_utf16(&self, pos: LineCol) -> LineColUtf16 {
        let mut shrink = 0;
        let mut col = pos.col;
        for w in self.wide_in_line(pos.line) {
            if w.col >= pos.col {
                break;
            }
            if pos.col < w.col + u32::from(w.len) {
                col = w.col;
                break;
            }
            shrink += u32::from(w.len) - w.utf16_len();
        }
        LineColUtf16 {
            line: pos.line,
            col: col.saturating_sub(shrink),
        }
    }

    /// Converts a UTF-16 column back to a byte column.
    pub fn from_utf16(&self, pos: LineColUtf16) -> LineCol {
        let mut bytes = 0u32;
        let mut units = 0u32;
        for w in self.wide_in_line(pos.line) {
            // ASCII run between the previous wide char and this one.
            let ascii = w.col - bytes;
            if units + ascii >= pos.col {
                break;
            }
            units += ascii;
            bytes = w.col;
            if units + w.utf16_len() > pos.col {
                // Column points inside a surrogate pair: snap to the character.
                return LineCol {
                    line: pos.line,
                    col: bytes,
                };
            }
            units += w.utf16_len();
            bytes += u32::from(w.len);
        }
        LineCol {
            line: pos.line,
            col: bytes + (pos.col - units),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lc(line: u32, col: u32) -> LineCol {
        LineCol { line, col }
    }

    #[test]
    fn ascii_lines() {
        let idx = LineIndex::new("ab\ncd\n\nef");
        assert_eq!(idx.line_count(), 4);
        assert_eq!(idx.line_col(0), lc(0, 0));
        assert_eq!(idx.line_col(2), lc(0, 2)); // the `\n` itself
        assert_eq!(idx.line_col(3), lc(1, 0));
        assert_eq!(idx.line_col(6), lc(2, 0));
        assert_eq!(idx.line_col(7), lc(3, 0));
        assert_eq!(idx.line_col(9), lc(3, 2));
        assert_eq!(idx.line_col(99), lc(3, 2));
        assert_eq!(idx.offset(lc(1, 1)), 4);
        assert_eq!(idx.offset(lc(1, 50)), 5, "clamps before the line break");
        assert_eq!(idx.offset(lc(9, 0)), 9);
    }

    #[test]
    fn crlf_and_lone_cr() {
        let idx = LineIndex::new("a\r\nb\rc\n");
        assert_eq!(idx.line_count(), 4);
        assert_eq!(idx.line_start(1), Some(3));
        assert_eq!(idx.line_start(2), Some(5));
        assert_eq!(idx.line_start(3), Some(7));
        assert_eq!(idx.line_col(4), lc(1, 1));
    }

    #[test]
    fn multibyte_and_emoji() {
        // "é" is 2 bytes / 1 unit, "€" 3 / 1, "😀" 4 / 2.
        let text = "aé€😀b\nxyz";
        let idx = LineIndex::new(text);
        let b = text.find('b').unwrap() as u32;
        assert_eq!(b, 1 + 2 + 3 + 4);
        assert_eq!(idx.line_col(b), lc(0, 10));
        assert_eq!(idx.to_utf16(lc(0, 10)), LineColUtf16 { line: 0, col: 5 });
        assert_eq!(idx.to_utf16(lc(0, 1)), LineColUtf16 { line: 0, col: 1 });
        assert_eq!(idx.to_utf16(lc(0, 3)), LineColUtf16 { line: 0, col: 2 });
        assert_eq!(idx.to_utf16(lc(0, 6)), LineColUtf16 { line: 0, col: 3 });
        for col in [0, 1, 3, 6, 10, 11] {
            let u = idx.to_utf16(lc(0, col));
            assert_eq!(
                idx.from_utf16(u),
                lc(0, col),
                "round trip of byte col {col}"
            );
        }
        // Inside the surrogate pair: snaps to the emoji start.
        assert_eq!(idx.from_utf16(LineColUtf16 { line: 0, col: 4 }), lc(0, 6));
        // Second line is ASCII and unaffected by the first.
        assert_eq!(idx.to_utf16(lc(1, 2)), LineColUtf16 { line: 1, col: 2 });
        assert_eq!(idx.from_utf16(LineColUtf16 { line: 1, col: 2 }), lc(1, 2));
        // Past the end of the line: keep the excess.
        assert_eq!(idx.from_utf16(LineColUtf16 { line: 0, col: 9 }), lc(0, 14));
    }

    #[test]
    fn byte_column_inside_a_multibyte_char_snaps() {
        let idx = LineIndex::new("😀b\néb");
        assert_eq!(idx.to_utf16(lc(0, 1)), LineColUtf16 { line: 0, col: 0 });
        assert_eq!(idx.to_utf16(lc(0, 3)), LineColUtf16 { line: 0, col: 0 });
        assert_eq!(idx.to_utf16(lc(0, 4)), LineColUtf16 { line: 0, col: 2 });
        assert_eq!(idx.to_utf16(lc(1, 1)), LineColUtf16 { line: 1, col: 0 });
        assert_eq!(idx.to_utf16(lc(1, 2)), LineColUtf16 { line: 1, col: 1 });
    }

    #[test]
    fn column_past_line_end_stays_on_the_line() {
        let idx = LineIndex::new("é\nb\r\nc");
        assert_eq!(idx.line_end(0), Some(2));
        assert_eq!(idx.line_end(1), Some(4));
        assert_eq!(idx.line_end(2), Some(7));
        assert_eq!(idx.offset(lc(0, 6)), 2, "before the `\\n`");
        assert_eq!(idx.offset(lc(1, 9)), 4, "before the `\\r\\n`");
        assert_eq!(idx.offset(lc(2, 9)), 7);
        assert_eq!(idx.offset(lc(7, 0)), 7);
    }

    #[test]
    fn sub_span_equality_is_structural() {
        let a = SubSpan::new(FileId(0), 1, 2);
        let b = SubSpan::new(FileId(0), 5, 9);
        assert_eq!(a, b);
        assert_ne!(a.0, b.0);
        assert_eq!(serde_json::to_string(&a).unwrap(), "[0,1,2]");
    }

    #[test]
    fn span_serialises_as_triple() {
        let span = Span::new(FileId(1), 3, 9);
        let json = serde_json::to_string(&span).unwrap();
        assert_eq!(json, "[1,3,9]");
        assert_eq!(serde_json::from_str::<Span>(&json).unwrap(), span);
        assert_eq!(span.len(), 6);
        assert!(span.contains(3) && !span.contains(9));
        assert_eq!(
            span.join(Span::new(FileId(1), 0, 4)),
            Span::new(FileId(1), 0, 9)
        );
    }
}
