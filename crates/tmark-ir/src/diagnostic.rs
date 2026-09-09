//! The one diagnostic type every stage emits.
//!
//! Design: `design/05-diagnostics.md`. Spec P4 "diagnostics are loud".

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::span::Span;

/// Design `05-diagnostics.md` §Severities by default.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Hint,
    Info,
    Warning,
    Error,
}

/// Stable diagnostic identifiers. Design `05-diagnostics.md` §Who emits what.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Code {
    // --- Parse (tmark-syntax) ---
    /// Spec §Roles: an attribute list with no host element.
    AttrNoHost,
    /// Spec §Roles: a role head not followed by `[` or `(`.
    RoleDanglingHead,
    /// Spec §Container directives: a `:::` fence without its closing line.
    ContainerUnclosed,
    /// Spec §Div: a `::: name` whose name is unknown.
    ContainerUnknown,
    /// Spec §Data directives: an info string whose second word is not a
    /// node word.
    FenceUnknownNodeWord,
    /// Spec §Front matter: the YAML island does not parse.
    FrontmatterYaml,
    /// Spec §Front matter: an unknown key under a validated namespace
    /// (`declare`, `sources`, `features`).
    FrontmatterUnknownKey,
    /// Appendix "Deprecation schedule": a deprecated spelling.
    Deprecated,
    // --- Resolve (tmark-registry) ---
    /// Spec §Ref: a key found in no registry.
    RefUnresolved,
    /// Spec §Cite: a key present in two registries.
    RefAmbiguous,
    /// Spec §CounterItem: an undeclared counter prefix.
    PrefixUnknown,
    /// Spec §Anchor: a prefix that disagrees with its host (`{#tbl:x}` on a
    /// figure).
    PrefixHostMismatch,
    /// Spec §Counters: the same label defined twice.
    LabelDuplicate,
    /// Spec §Cite: a `[^key]` citation shadowed by a real footnote.
    CitationShadowedByFootnote,
    /// Spec §Cross-document references: an inventory that cannot be loaded.
    CrossrefInventoryMissing,
    /// Design `06-registries.md`: an inventory whose hash no longer matches.
    CrossrefInventoryStale,
    /// Spec §Includes: an included file that cannot be loaded.
    IncludeMissing,
    // --- Lint (tmark-lint) ---
    /// Spec §Ref: "Figure 3" typed in prose.
    HardcodedNumber,
    /// Spec §Ref: "above" or "below" used as a reference.
    PositionWord,
    /// Spec §Anchor: a caption id without the recommended prefix.
    CaptionIdOffConvention,
    /// Spec §Conformance and deviations: an X-class construct under the
    /// strict profile.
    StrictXConstruct,
    /// Appendix "Deprecation schedule": a deprecated front-matter key.
    DeprecatedFrontmatterKey,
    /// Spec §Para: a leading strong span promoted to a lead-in.
    LeadPromotion,
    /// Spec §Header: a heading level skipped.
    HeadingSkip,
}

impl Code {
    /// The stable kebab-case identifier printed by the CLI and shown by the
    /// LSP.
    pub fn id(self) -> &'static str {
        match self {
            Code::AttrNoHost => "attr-no-host",
            Code::RoleDanglingHead => "role-dangling-head",
            Code::ContainerUnclosed => "container-unclosed",
            Code::ContainerUnknown => "container-unknown",
            Code::FenceUnknownNodeWord => "fence-unknown-node-word",
            Code::FrontmatterYaml => "frontmatter-yaml",
            Code::FrontmatterUnknownKey => "frontmatter-unknown-key",
            Code::Deprecated => "deprecated",
            Code::RefUnresolved => "ref-unresolved",
            Code::RefAmbiguous => "ref-ambiguous",
            Code::PrefixUnknown => "prefix-unknown",
            Code::PrefixHostMismatch => "prefix-host-mismatch",
            Code::LabelDuplicate => "label-duplicate",
            Code::CitationShadowedByFootnote => "citation-shadowed-by-footnote",
            Code::CrossrefInventoryMissing => "crossref-inventory-missing",
            Code::CrossrefInventoryStale => "crossref-inventory-stale",
            Code::IncludeMissing => "include-missing",
            Code::HardcodedNumber => "hardcoded-number",
            Code::PositionWord => "position-word",
            Code::CaptionIdOffConvention => "caption-id-off-convention",
            Code::StrictXConstruct => "strict-x-construct",
            Code::DeprecatedFrontmatterKey => "deprecated-frontmatter-key",
            Code::LeadPromotion => "lead-promotion",
            Code::HeadingSkip => "heading-skip",
        }
    }

    /// Design `05-diagnostics.md` §Severities by default.
    pub fn default_severity(self) -> Severity {
        match self {
            Code::FrontmatterYaml | Code::FrontmatterUnknownKey | Code::StrictXConstruct => {
                Severity::Error
            }
            Code::AttrNoHost
            | Code::RoleDanglingHead
            | Code::ContainerUnclosed
            | Code::ContainerUnknown
            | Code::FenceUnknownNodeWord
            | Code::Deprecated
            | Code::RefUnresolved
            | Code::RefAmbiguous
            | Code::PrefixUnknown
            | Code::PrefixHostMismatch
            | Code::LabelDuplicate
            | Code::CitationShadowedByFootnote
            | Code::CrossrefInventoryMissing
            | Code::CrossrefInventoryStale
            | Code::IncludeMissing
            | Code::DeprecatedFrontmatterKey => Severity::Warning,
            Code::LeadPromotion => Severity::Info,
            Code::HardcodedNumber
            | Code::PositionWord
            | Code::CaptionIdOffConvention
            | Code::HeadingSkip => Severity::Hint,
        }
    }
}

/// A text edit the LSP can apply: replace `span` with `replacement`.
/// Design `05-diagnostics.md` §Fixes. A `NodeEdit` variant arrives with
/// `tmark-fmt`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Fix {
    pub span: Span,
    pub replacement: String,
}

/// Design `05-diagnostics.md` §One type. The CLI prints
/// `file:line:col: severity code: message`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    pub code: Code,
    pub severity: Severity,
    pub span: Span,
    /// One sentence, no trailing period, names the construct.
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<Fix>,
    /// Other locations involved, with a label each (the other definition of
    /// a duplicate key).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<(Span, String)>,
}

impl Diagnostic {
    /// A diagnostic at the code's default severity, without fix or related
    /// locations.
    pub fn new(code: Code, span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: code.default_severity(),
            span,
            message: message.into(),
            fix: None,
            related: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::FileId;

    #[test]
    fn ids_match_serde_names() {
        let codes = [Code::AttrNoHost, Code::RefUnresolved, Code::HeadingSkip];
        for code in codes {
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(json, format!("\"{}\"", code.id()));
        }
    }

    #[test]
    fn constructor_uses_default_severity() {
        let d = Diagnostic::new(
            Code::RefUnresolved,
            Span::new(FileId(0), 4, 12),
            "unresolved reference `@fig:x`",
        );
        assert_eq!(d.severity, Severity::Warning);
        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["code"], "ref-unresolved");
        assert!(json.get("fix").is_none());
    }
}
