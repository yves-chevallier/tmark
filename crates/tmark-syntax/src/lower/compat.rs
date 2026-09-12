//! `compat-unsupported`: PyMdownX spellings the parser recognises but does
//! not implement yet (spec Appendix "PyMdownX compatibility profile",
//! milestone 5), reported instead of silently left as text (P4): wiki
//! links and fancy list markers. Each scan is deliberately narrow: a miss
//! is literal text, as before; a false positive would be a wrong warning
//! on prose. Content tabs, progress bars, emoji and icon shortcodes,
//! `^^x^^` and `[TOC]` were implemented by the C31–C42 wave, critic markup
//! by C49 (`tmark_ir::critic`, `lower/inline.rs`), and each left this
//! file.

use tmark_ir::{Code, Inline, Span};

use super::Lowerer;

/// A recognised but unimplemented spelling inside a text run: `(start,
/// end, what)` as byte offsets into `text`.
fn scan_text(text: &str) -> Vec<(usize, usize, &'static str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &text[i..];
        // Wiki link: `[[Page]]`.
        if rest.starts_with("[[") {
            if let Some(at) = rest.find("]]") {
                if at > 2 && !rest[2..at].contains('[') {
                    out.push((i, i + at + 2, "wiki link `[[…]]`"));
                    i += at + 2;
                    continue;
                }
            }
        }
        i += rest.chars().next().map_or(1, char::len_utf8);
    }
    out
}

/// A paragraph-initial spelling: what it is, when the paragraph's text
/// starts one.
fn scan_paragraph_start(text: &str) -> Option<&'static str> {
    // Fancy list markers: `a.`, `iv.`, `#.`, `1)`, `a)`. Upper-case letters
    // are left alone (`I. M. Pei was…` is prose more often than a list).
    let marker_end = text.find([' ', '\t'])?;
    let marker = &text[..marker_end];
    let one_lower = |body: &str| body.len() == 1 && body.as_bytes()[0].is_ascii_lowercase();
    let fancy = if let Some(body) = marker.strip_suffix(')') {
        body == "#"
            || (!body.is_empty() && body.bytes().all(|b| b.is_ascii_digit()))
            || one_lower(body)
    } else if let Some(body) = marker.strip_suffix('.') {
        body == "#"
            || one_lower(body)
            || (!body.is_empty()
                && body
                    .bytes()
                    .all(|b| matches!(b, b'i' | b'v' | b'x' | b'l' | b'c')))
    } else {
        false
    };
    fancy.then_some("fancy list marker (`a.`, `i.`, `#.`, `1)`)")
}

impl Lowerer {
    /// Report the unimplemented spellings inside one text node (wiki
    /// links). `value` is the decoded text, `source` its source slice: a
    /// spelling whose first character is escaped in the source (`\[[x]]`)
    /// is the author's literal text and is not reported.
    pub fn compat_scan_text(&mut self, value: &str, source: &str, span: Span) {
        // Offsets into `value` are offsets into `source` only when nothing
        // was decoded; otherwise the whole node is reported.
        let exact = value == source && (span.end - span.start) as usize == value.len();
        for (start, end, what) in scan_text(value) {
            let first = &value[start..start + 1];
            if source.contains(&format!("\\{first}")) {
                continue;
            }
            let at = if exact {
                Span::new(
                    span.file,
                    span.start + start as u32,
                    span.start + end as u32,
                )
            } else {
                span
            };
            self.compat_unsupported(at, what);
        }
    }

    /// Report a paragraph that starts an unimplemented block spelling
    /// (fancy list markers); `source` is the paragraph's source, whose
    /// leading backslash means literal text.
    pub fn compat_scan_paragraph(&mut self, content: &[Inline], source: &str, span: Span) {
        let Some(Inline::Str(first)) = content.first() else {
            return;
        };
        if source.starts_with('\\') {
            return;
        }
        if let Some(what) = scan_paragraph_start(&first.text) {
            self.compat_unsupported(span, what);
        }
    }

    pub fn compat_unsupported(&mut self, span: Span, what: &str) {
        self.diag(
            Code::CompatUnsupported,
            span,
            format!("{what} is not implemented yet; it stays literal text"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_scans() {
        let found: Vec<&str> = scan_text("a [=45% \"x\"] [[Page]] :smile: 10:30: end:")
            .into_iter()
            .map(|(_, _, w)| w)
            .collect();
        assert_eq!(found, ["wiki link `[[…]]`"]);
        assert!(scan_text("plain [x] {k=v} 1:2 note: text").is_empty());
    }

    #[test]
    fn paragraph_scans() {
        assert!(
            scan_paragraph_start("=== \"Tab\"").is_none(),
            "tabs are implemented"
        );
        assert!(
            scan_paragraph_start("[TOC]").is_none(),
            "[TOC] is implemented"
        );
        assert!(scan_paragraph_start("a. first").is_some());
        assert!(scan_paragraph_start("iv. fourth").is_some());
        assert!(scan_paragraph_start("#. any").is_some());
        assert!(scan_paragraph_start("1) one").is_some());
        assert!(scan_paragraph_start("I. M. Pei").is_none());
        assert!(scan_paragraph_start("i.e. this").is_none());
        assert!(scan_paragraph_start("Note. this").is_none());
        assert!(scan_paragraph_start("plain").is_none());
    }
}
