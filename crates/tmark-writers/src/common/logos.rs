//! TeX logos (spec §TeX logos): the words of `registry::TEX_LOGOS`,
//! written as plain words in prose, are set as logos by the writers under
//! the feature `typography.tex-logos` (on by default). Whole words only,
//! case-sensitive; the split runs on `Str` nodes alone, so code, math, raw
//! passthroughs, link destinations and attribute values never see it.

use tmark_ir::{registry, Document};

/// A piece of a `Str`: plain text or a logo word.
#[derive(Debug, PartialEq, Eq)]
pub enum Segment<'a> {
    Text(&'a str),
    Logo(&'a str),
}

/// Whether the document turns the feature on (front matter `features`,
/// else the registry default).
pub fn enabled(doc: &Document) -> bool {
    doc.front_matter.keys.press.feature("typography.tex-logos")
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric()
}

/// Splits `text` at whole-word logo names (longest first, so `LaTeX2e`
/// wins over `LaTeX`).
pub fn split(text: &str) -> Vec<Segment<'_>> {
    let mut names: Vec<&str> = registry::TEX_LOGOS.to_vec();
    names.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < text.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let before_ok = i == 0 || !text[..i].chars().next_back().is_some_and(is_word);
        let found = before_ok
            .then(|| {
                names.iter().find(|n| {
                    text[i..].starts_with(*n)
                        && !text[i + n.len()..].chars().next().is_some_and(is_word)
                })
            })
            .flatten();
        match found {
            Some(name) => {
                if start < i {
                    out.push(Segment::Text(&text[start..i]));
                }
                out.push(Segment::Logo(&text[i..i + name.len()]));
                i += name.len();
                start = i;
            }
            None => i += text[i..].chars().next().map_or(1, char::len_utf8),
        }
    }
    if start < text.len() {
        out.push(Segment::Text(&text[start..]));
    }
    if out.is_empty() {
        out.push(Segment::Text(text));
    }
    out
}

/// The HTML of a logo: `<span class="tex-logo">` with the raised `a` and
/// the lowered `e` of the TeX and LaTeX families.
pub fn html(name: &str) -> String {
    let inner = name
        .replace("La", "L<sup>a</sup>")
        .replace("TeX", "T<sub>e</sub>X")
        .replace("Xe", "X<sub>e</sub>");
    format!("<span class=\"tex-logo\">{inner}</span>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_whole_words() {
        assert_eq!(
            split("Use LaTeX or XeLaTeX, not latex or LaTeXy; LaTeX2e too."),
            vec![
                Segment::Text("Use "),
                Segment::Logo("LaTeX"),
                Segment::Text(" or "),
                Segment::Logo("XeLaTeX"),
                Segment::Text(", not latex or LaTeXy; "),
                Segment::Logo("LaTeX2e"),
                Segment::Text(" too."),
            ]
        );
        assert_eq!(split("TeX"), vec![Segment::Logo("TeX")]);
        assert_eq!(split("plain"), vec![Segment::Text("plain")]);
        assert_eq!(
            html("XeLaTeX"),
            "<span class=\"tex-logo\">X<sub>e</sub>L<sup>a</sup>T<sub>e</sub>X</span>"
        );
    }
}
