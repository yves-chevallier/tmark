//! HTML escaping: text nodes and attribute values.

/// Escapes text content: `&`, `<`, `>` and `"` (as the CommonMark
/// reference implementation does, so the suite's expectations compare).
pub fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escapes a double-quoted attribute value.
pub fn attr(s: &str) -> String {
    text(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes() {
        assert_eq!(text("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(text("say \"hi\""), "say &quot;hi&quot;");
        assert_eq!(attr("say \"hi\""), "say &quot;hi&quot;");
    }
}
