//! The output writer: a string plus the continuation-line prefixes of the
//! enclosing containers (list indentation, `> ` of block quotes, the four
//! spaces of a definition or footnote body).
//!
//! Design `04-printer.md` §Implementation notes: "`Out` tracks column and
//! indentation for nested containers and list items; it is the only state."
//! Every line that carries text starts with the joined prefixes; an empty
//! line carries the prefixes with trailing whitespace removed, so the output
//! never has trailing spaces.

/// The printer's output buffer.
#[derive(Debug, Default)]
pub struct Out {
    buf: String,
    prefixes: Vec<String>,
    /// Text has been written on the current line (prefixes included).
    line_started: bool,
}

impl Out {
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes `text`. A `\n` ends the line; the next character that is not
    /// a newline first writes the prefixes.
    pub fn push(&mut self, text: &str) {
        for ch in text.chars() {
            self.push_char(ch);
        }
    }

    pub fn push_char(&mut self, ch: char) {
        if ch == '\n' {
            if !self.line_started {
                self.write_prefixes(true);
            }
            self.buf.push('\n');
            self.line_started = false;
        } else {
            if !self.line_started {
                self.write_prefixes(false);
                self.line_started = true;
            }
            self.buf.push(ch);
        }
    }

    fn write_prefixes(&mut self, trimmed: bool) {
        let joined: String = self.prefixes.concat();
        if trimmed {
            self.buf.push_str(joined.trim_end());
        } else {
            self.buf.push_str(&joined);
        }
    }

    /// Ends the current line if it carries text.
    pub fn ensure_newline(&mut self) {
        if self.line_started {
            self.push_char('\n');
        }
    }

    /// Ends the current line if needed, then writes one empty line.
    pub fn blank_line(&mut self) {
        self.ensure_newline();
        self.push_char('\n');
    }

    /// Adds a prefix for the lines that follow the current one.
    pub fn push_prefix(&mut self, prefix: &str) {
        self.prefixes.push(prefix.to_string());
    }

    pub fn pop_prefix(&mut self) {
        self.prefixes.pop();
    }

    /// Nothing has been written on the current line yet.
    pub fn at_line_start(&self) -> bool {
        !self.line_started
    }

    /// The last character written, prefixes included.
    pub fn last_char(&self) -> Option<char> {
        self.buf.chars().next_back()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// The output, ending with exactly one newline (empty when nothing was
    /// written).
    pub fn finish(mut self) -> String {
        let trimmed = self.buf.trim_end_matches('\n').len();
        self.buf.truncate(trimmed);
        if !self.buf.is_empty() {
            self.buf.push('\n');
        }
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_apply_from_the_next_line() {
        let mut out = Out::new();
        out.push("- ");
        out.push_prefix("  ");
        out.push("first\nsecond\n");
        out.blank_line();
        out.push("third\n");
        out.pop_prefix();
        out.push("- next\n");
        assert_eq!(out.finish(), "- first\n  second\n\n  third\n- next\n");
    }

    #[test]
    fn empty_lines_carry_trimmed_prefixes() {
        let mut out = Out::new();
        out.push_prefix("> ");
        out.push("a\n");
        out.blank_line();
        out.push("b\n");
        assert_eq!(out.finish(), "> a\n>\n> b\n");
    }

    #[test]
    fn finish_normalises_trailing_newlines() {
        let mut out = Out::new();
        out.push("a\n\n\n");
        assert_eq!(out.finish(), "a\n");
        assert_eq!(Out::new().finish(), "");
    }
}
