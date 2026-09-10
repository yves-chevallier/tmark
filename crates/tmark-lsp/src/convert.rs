//! Coordinates and diagnostics in protocol terms.
//!
//! Design: `08-lsp.md` §Protocol details: the LSP speaks UTF-16 columns,
//! the IR speaks bytes; `LineIndex` converts once, both ways.

use std::path::PathBuf;

use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag, Location,
    NumberOrString, Position, Range, Uri,
};
use tmark::ir::{Code, FileId, LineCol, LineColUtf16, LineIndex, Severity, Span};

pub fn position(index: &LineIndex, offset: u32) -> Position {
    let at = index.to_utf16(index.line_col(offset));
    Position::new(at.line, at.col)
}

pub fn range(index: &LineIndex, span: Span) -> Range {
    Range::new(position(index, span.start), position(index, span.end))
}

pub fn offset(index: &LineIndex, position: Position) -> u32 {
    let at = index.from_utf16(LineColUtf16 {
        line: position.line,
        col: position.character,
    });
    index.offset(LineCol {
        line: at.line,
        col: at.col,
    })
}

/// The range covering the whole text.
pub fn full_range(index: &LineIndex) -> Range {
    Range::new(Position::new(0, 0), position(index, index.len()))
}

fn severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
        Severity::Hint => DiagnosticSeverity::HINT,
    }
}

/// A TMark diagnostic of `file` as the protocol shows it. Related
/// locations in other files are dropped until included files are
/// published (design `08-lsp.md`).
pub fn diagnostic(
    uri: &Uri,
    index: &LineIndex,
    file: FileId,
    d: &tmark::Diagnostic,
) -> Option<Diagnostic> {
    if d.span.file != file {
        return None;
    }
    let tags = match d.code {
        Code::Deprecated | Code::DeprecatedFrontmatterKey => vec![DiagnosticTag::DEPRECATED],
        _ => Vec::new(),
    };
    let related: Vec<_> = d
        .related
        .iter()
        .filter(|(span, _)| span.file == file)
        .map(|(span, message)| DiagnosticRelatedInformation {
            location: Location::new(uri.clone(), range(index, *span)),
            message: message.clone(),
        })
        .collect();
    Some(Diagnostic {
        range: range(index, d.span),
        severity: Some(severity(d.severity)),
        code: Some(NumberOrString::String(d.code.id().to_string())),
        code_description: None,
        source: Some("tmark".into()),
        message: d.message.clone(),
        related_information: (!related.is_empty()).then_some(related),
        tags: (!tags.is_empty()).then_some(tags),
        data: None,
    })
}

/// The file-system path of a `file:` URI; `None` for other schemes.
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    if uri.scheme().map(|s| s.as_str()) != Some("file") {
        return None;
    }
    let decoded = percent_decode(uri.path().as_str());
    // `file:///C:/x` carries a leading slash the platform path must not.
    let path = decoded
        .strip_prefix('/')
        .filter(|rest| rest.as_bytes().get(1) == Some(&b':'))
        .unwrap_or(&decoded);
    Some(PathBuf::from(path))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &s[i + 1..i + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn utf16_positions() {
        let text = "a😀b\né";
        let index = LineIndex::new(text);
        let span = Span::new(FileId(0), 5, 6); // the `b`
        let r = range(&index, span);
        assert_eq!((r.start.line, r.start.character), (0, 3));
        assert_eq!((r.end.line, r.end.character), (0, 4));
        assert_eq!(offset(&index, Position::new(0, 3)), 5);
        assert_eq!(offset(&index, Position::new(1, 1)), text.len() as u32);
        assert_eq!(full_range(&index).end, Position::new(1, 1));
    }

    #[test]
    fn file_uri_paths() {
        let uri = Uri::from_str("file:///home/me/a%20b/doc.md").unwrap();
        assert_eq!(
            uri_to_path(&uri),
            Some(PathBuf::from("/home/me/a b/doc.md"))
        );
        let uri = Uri::from_str("file:///C:/Users/me/doc.md").unwrap();
        assert_eq!(uri_to_path(&uri), Some(PathBuf::from("C:/Users/me/doc.md")));
        let uri = Uri::from_str("untitled:Untitled-1").unwrap();
        assert_eq!(uri_to_path(&uri), None);
    }
}
