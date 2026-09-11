//! LaTeX math for Typst through `mitex` (writers-and-passes.md §4, a port
//! of `typst/writer.py:267-297`): `#mi(`…`)` inline, `#mitex(`…`)`
//! display; `\label{k}` is stripped and re-attached as ` <k>`; a math
//! node that is only `\eqref{k}`/`\ref{k}` becomes `#ref(<k>)`.

use super::escape;

/// `@preview/mitex:0.2.6`.
pub const MITEX_PACKAGE: &str = "@preview/mitex:0.2.6";

/// What a math node renders as.
pub struct Rendered {
    pub text: String,
    /// A label was emitted: the template must number equations.
    pub labelled: bool,
    /// `mitex` was used.
    pub mitex: bool,
}

/// `\label{k}` occurrences: the labels and the text without them.
fn strip_labels(tex: &str) -> (Vec<String>, String) {
    let mut labels = Vec::new();
    let mut out = String::with_capacity(tex.len());
    let mut rest = tex;
    while let Some(pos) = rest.find("\\label") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + "\\label".len()..];
        let trimmed = after.trim_start();
        if let Some(inner) = trimmed.strip_prefix('{') {
            if let Some(end) = inner.find('}') {
                labels.push(inner[..end].to_string());
                rest = &inner[end + 1..];
                continue;
            }
        }
        out.push_str("\\label");
        rest = after;
    }
    out.push_str(rest);
    (labels, out.trim().to_string())
}

/// `^\s*\\(?:eqref|ref)\s*\{([^}]*)\}\s*$`.
fn pure_ref(tex: &str) -> Option<&str> {
    let t = tex.trim();
    let rest = t
        .strip_prefix("\\eqref")
        .or_else(|| t.strip_prefix("\\ref"))?
        .trim_start();
    let inner = rest.strip_prefix('{')?;
    let end = inner.find('}')?;
    inner[end + 1..].trim().is_empty().then_some(&inner[..end])
}

/// Renders `tex`; `id` is the `MathBlock` anchor, which wins over an
/// embedded `\label`.
pub fn render(tex: &str, display: bool, id: Option<&str>) -> Rendered {
    if let Some(key) = pure_ref(tex) {
        let label = escape::label(key);
        if !label.is_empty() {
            return Rendered {
                text: format!("#ref(<{label}>)"),
                labelled: true,
                mitex: false,
            };
        }
    }
    let (labels, body) = strip_labels(tex);
    let fence = "`".repeat(longest_run(&body) + 1);
    let label = id
        .map(str::to_string)
        .or_else(|| labels.first().cloned())
        .map(|l| escape::label(&l))
        .filter(|l| !l.is_empty());
    if display {
        let mut text = format!("#mitex({fence}{body}{fence})");
        if let Some(label) = &label {
            text.push_str(&format!(" <{label}>"));
        }
        Rendered {
            text,
            labelled: label.is_some(),
            mitex: true,
        }
    } else {
        Rendered {
            text: format!("#mi({fence}{body}{fence})"),
            labelled: false,
            mitex: true,
        }
    }
}

fn longest_run(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_and_display() {
        let r = render("a^2", false, None);
        assert_eq!(r.text, "#mi(`a^2`)");
        assert!(r.mitex && !r.labelled);
        let r = render("e = mc^2", true, Some("eq:einstein"));
        assert_eq!(r.text, "#mitex(`e = mc^2`) <eq:einstein>");
        assert!(r.labelled);
    }

    #[test]
    fn labels_and_refs() {
        let r = render("x = 1 \\label{eq:one}", true, None);
        assert_eq!(r.text, "#mitex(`x = 1`) <eq:one>");
        let r = render("\\eqref{eq:max2}", false, None);
        assert_eq!(r.text, "#ref(<eq:max2>)");
        assert!(!r.mitex);
        let r = render("a`b", false, None);
        assert_eq!(r.text, "#mi(``a`b``)");
    }
}
