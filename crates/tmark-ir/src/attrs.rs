//! Attribute lists: `{#id .class key=value}`.
//!
//! Spec §Attributes (family 1) and §Roles ("Three attributes are universal").
//! Design: `design/03-ir.md` §Attributes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::span::{FileId, SubSpan};

/// The attribute list of a host element.
///
/// Order is normalised by the printer (`#id`, `.class`, keys in source
/// order). The universal attributes `#id`, `lang` and `media` have accessors
/// but stay in the list so unknown hosts round-trip.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Attrs {
    /// `#id`: an anchor (spec §Anchor).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Source range of the id token without its `#`, when the list was
    /// parsed from text (spec §Round-trip and source spans; design 03
    /// §Identity and spans). `None` when built by a pass or from JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_span: Option<SubSpan>,
    /// `.class` entries, in source order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    /// `key=value` entries, in source order. Duplicate keys are kept; `get`
    /// returns the first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kv: Vec<(String, String)>,
}

impl Attrs {
    pub fn new() -> Self {
        Self::default()
    }

    /// The `#id` attribute (spec §Anchor).
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// The `lang=` attribute (spec §Roles, universal attributes).
    pub fn lang(&self) -> Option<&str> {
        self.get("lang")
    }

    /// The `media=` attribute: `all`, `print` or `web` (spec §Roles).
    pub fn media(&self) -> Option<&str> {
        self.get("media")
    }

    /// The first value of `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.kv
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.classes.iter().any(|c| c == class)
    }

    pub fn is_empty(&self) -> bool {
        self.id.is_none() && self.classes.is_empty() && self.kv.is_empty()
    }

    /// Moves `id_span` from offsets relative to the list's text to
    /// offsets in `file`, `base` being where that text starts. A parser
    /// that does not know the base drops the sub-span instead.
    pub fn relocate(&mut self, file: FileId, base: u32) {
        if let Some(SubSpan(span)) = &mut self.id_span {
            span.file = file;
            span.start += base;
            span.end += base;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors() {
        let attrs = Attrs {
            id: Some("sec:intro".into()),
            id_span: None,
            classes: vec!["epigraph".into()],
            kv: vec![
                ("lang".into(), "fr".into()),
                ("media".into(), "print".into()),
                ("lang".into(), "en".into()),
            ],
        };
        assert_eq!(attrs.id(), Some("sec:intro"));
        assert_eq!(attrs.lang(), Some("fr"));
        assert_eq!(attrs.media(), Some("print"));
        assert_eq!(attrs.get("width"), None);
        assert!(attrs.has_class("epigraph"));
        assert!(!attrs.is_empty());
        assert!(Attrs::new().is_empty());
        assert_eq!(serde_json::to_string(&Attrs::new()).unwrap(), "{}");
    }
}
