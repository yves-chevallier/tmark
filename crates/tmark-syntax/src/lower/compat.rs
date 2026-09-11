//! `compat-unsupported`: PyMdownX spellings the parser recognises but does
//! not implement yet (spec Appendix "PyMdownX compatibility profile",
//! milestone 5), reported instead of silently left as text (P4). Each scan
//! is deliberately narrow: a miss is literal text, as before; a false
//! positive would be a wrong warning on prose.

use tmark_ir::{Code, Inline, Span};

use super::Lowerer;

/// Critic markup, which the tokenizer hands over as a brace group:
/// `{++ins++}`, `{--del--}`, `{~~a~>b~~}`, `{==hl==}`, `{>>comment<<}`
/// (`value` is the text between the braces).
fn is_critic(value: &str) -> bool {
    [
        ("++", "++"),
        ("--", "--"),
        ("~~", "~~"),
        ("==", "=="),
        (">>", "<<"),
    ]
    .iter()
    .any(|(open, close)| value.len() >= 4 && value.starts_with(open) && value.ends_with(close))
}

/// A recognised but unimplemented spelling inside a text run: `(start,
/// end, what)` as byte offsets into `text`.
fn scan_text(text: &str) -> Vec<(usize, usize, &'static str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &text[i..];
        // Progress bar: `[=45% "label"]`.
        if rest.starts_with("[=") && rest[2..].starts_with(|c: char| c.is_ascii_digit()) {
            if let Some(at) = rest.find(']') {
                if rest[2..at].contains('%') {
                    out.push((i, i + at + 1, "progress bar `[=n% \"label\"]`"));
                    i += at + 1;
                    continue;
                }
            }
        }
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
        // Emoji or icon shortcode: `:smile:`, `:material-home:`, not
        // preceded or followed by a word character (`a:b:`, `10:30:`).
        if rest.starts_with(':')
            && (i == 0 || !text[..i].ends_with(|c: char| c.is_alphanumeric()))
            && rest[1..].starts_with(|c: char| c.is_ascii_lowercase())
        {
            let len = rest[1..]
                .bytes()
                .take_while(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'+')
                })
                .count();
            if rest[1 + len..].starts_with(':')
                && !rest[2 + len..].starts_with(|c: char| c.is_alphanumeric())
            {
                let what = if rest[1..].starts_with("material-")
                    || rest[1..].starts_with("fontawesome-")
                    || rest[1..].starts_with("octicons-")
                    || rest[1..].starts_with("simple-")
                {
                    "icon shortcode `:material-…:`"
                } else {
                    "emoji shortcode `:name:`"
                };
                out.push((i, i + 2 + len, what));
                i += 2 + len;
                continue;
            }
        }
        i += rest.chars().next().map_or(1, char::len_utf8);
    }
    out
}

/// A paragraph-initial spelling: what it is, when the paragraph's text
/// starts one.
fn scan_paragraph_start(text: &str) -> Option<&'static str> {
    if text.starts_with("=== \"") || text.starts_with("===! \"") || text.starts_with("===+ \"") {
        return Some("content tabs `=== \"Title\"`");
    }
    if text.trim_end() == "[TOC]" {
        return Some("`[TOC]` (the table of contents is `press.toc` in print)");
    }
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
    /// Report the unimplemented spellings inside one text node (progress
    /// bars, wiki links, shortcodes). `value` is the decoded text, `source`
    /// its source slice: a spelling whose first character is escaped in the
    /// source (`\[TOC]`, `\:smile:`) is the author's literal text and is not
    /// reported.
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

    /// Report a literal brace group that is critic markup.
    pub fn compat_scan_brace(&mut self, value: &str, span: Span) {
        if is_critic(value) {
            self.compat_unsupported(span, "critic markup");
        }
    }

    /// Report a paragraph that starts an unimplemented block spelling
    /// (content tabs, `[TOC]`, fancy list markers); `source` is the
    /// paragraph's source, whose leading backslash means literal text.
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
        let found: Vec<&str> = scan_text(
            "a [=45% \"x\"] [[Page]] :smile: :material-home: 10:30: a:b: http://x.y:80/ end:",
        )
        .into_iter()
        .map(|(_, _, w)| w)
        .collect();
        assert_eq!(
            found,
            [
                "progress bar `[=n% \"label\"]`",
                "wiki link `[[…]]`",
                "emoji shortcode `:name:`",
                "icon shortcode `:material-…:`",
            ]
        );
        assert!(scan_text("plain [x] {k=v} 1:2 note: text").is_empty());
        assert!(is_critic("--del--") && is_critic(">>c<<") && is_critic("~~a~>b~~"));
        assert!(!is_critic("--") && !is_critic("k=v") && !is_critic("--a++"));
    }

    #[test]
    fn paragraph_scans() {
        assert!(scan_paragraph_start("=== \"Tab\"").is_some());
        assert!(scan_paragraph_start("[TOC]").is_some());
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
