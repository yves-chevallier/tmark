//! Attribute lists: `{#id .class key=value}`, id first, then classes, then
//! keys in source order; a value is quoted when it holds whitespace, `}`,
//! a quote, or is empty (spec §Attributes; design 04 "What normalises").

use tmark_ir::Attrs;

use crate::out::Out;

/// The value of `key=value`, quoted when it has to be.
pub fn value(v: &str) -> String {
    if v.is_empty() || v.contains(|c: char| c.is_whitespace() || matches!(c, '}' | '"' | '=')) {
        format!("\"{}\"", v.replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

/// The items of an attribute list, without the braces; empty for no attrs.
pub fn items(attrs: &Attrs) -> String {
    let mut parts = Vec::new();
    if let Some(id) = &attrs.id {
        parts.push(format!("#{id}"));
    }
    for class in &attrs.classes {
        parts.push(format!(".{class}"));
    }
    for (k, v) in &attrs.kv {
        parts.push(format!("{k}={}", value(v)));
    }
    parts.join(" ")
}

/// Writes ` {…}` when the list is not empty (`prefix` is what precedes it,
/// normally one space).
pub fn write(out: &mut Out, attrs: &Attrs, prefix: &str) {
    if attrs.is_empty() {
        return;
    }
    out.push(prefix);
    out.push("{");
    out.push(&items(attrs));
    out.push("}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_and_quoting() {
        let mut attrs = Attrs::new();
        attrs.kv.push(("lang".into(), "en".into()));
        attrs.classes.push("draft".into());
        attrs.id = Some("sec:a".into());
        attrs.kv.push(("title".into(), "LaTeX toolchain".into()));
        assert_eq!(
            items(&attrs),
            "#sec:a .draft lang=en title=\"LaTeX toolchain\""
        );
        assert_eq!(value(""), "\"\"");
        assert_eq!(value("60%"), "60%");
    }
}
