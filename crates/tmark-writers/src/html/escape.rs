//! HTML escaping: text nodes and attribute values.

/// Escapes text content: `&`, `<`, `>`.
pub fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escapes a double-quoted attribute value: `text` plus `"`.
pub fn attr(s: &str) -> String {
    text(s).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes() {
        assert_eq!(text("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(attr("say \"hi\""), "say &quot;hi&quot;");
    }
}
