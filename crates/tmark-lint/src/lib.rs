//! The lint rule catalogue over the IR and the registries. Design:
//! `design/05-diagnostics.md`. Parse and resolve diagnostics are facts;
//! these are opinions the user can switch off or re-level.

#![forbid(unsafe_code)]

mod rules;

use std::collections::BTreeMap;

use tmark_ir::{Code, Diagnostic, Document, Severity};
use tmark_registry::Resolved;

/// What a rule sees.
pub struct Context<'a> {
    pub doc: &'a Document,
    pub resolved: &'a Resolved,
    /// The source text, for rules that look at spellings.
    pub text: &'a str,
}

/// One lint rule: pure and stateless.
pub trait Rule: Sync {
    fn code(&self) -> Code;
    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>);
}

/// Per-code severity overrides; `None` switches a rule off.
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub levels: BTreeMap<Code, Option<Severity>>,
}

impl Config {
    /// `"off" | "hint" | "info" | "warning" | "error"`, as in `tmark.toml`.
    pub fn set(&mut self, code: Code, level: &str) -> Result<(), String> {
        let level = match level {
            "off" => None,
            other => match Severity::parse(other) {
                Some(severity) => Some(severity),
                None => return Err(format!("unknown lint level `{other}`")),
            },
        };
        self.levels.insert(code, level);
        Ok(())
    }

    fn enabled(&self, code: Code) -> bool {
        !matches!(self.levels.get(&code), Some(None))
    }

    fn severity(&self, code: Code) -> Severity {
        match self.levels.get(&code) {
            Some(Some(s)) => *s,
            _ => code.default_severity(),
        }
    }
}

/// Every rule, in the order the catalogue lists them.
pub fn rules() -> &'static [&'static dyn Rule] {
    rules::RULES
}

/// Run the catalogue. Diagnostics come out in document order per rule,
/// with their configured severity.
pub fn lint(doc: &Document, resolved: &Resolved, text: &str, config: &Config) -> Vec<Diagnostic> {
    let ctx = Context {
        doc,
        resolved,
        text,
    };
    let mut out = Vec::new();
    for rule in rules() {
        if !config.enabled(rule.code()) {
            continue;
        }
        let mut found = Vec::new();
        rule.check(&ctx, &mut found);
        for mut d in found {
            d.severity = config.severity(d.code);
            out.push(d);
        }
    }
    out
}
