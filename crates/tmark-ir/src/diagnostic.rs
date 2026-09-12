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

impl Severity {
    /// The word the CLI prints and `tmark.toml` reads.
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Hint => "hint",
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }

    /// The inverse of [`Severity::as_str`].
    pub fn parse(s: &str) -> Option<Severity> {
        match s {
            "hint" => Some(Severity::Hint),
            "info" => Some(Severity::Info),
            "warning" => Some(Severity::Warning),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
}

/// Stable diagnostic identifiers. Design `05-diagnostics.md` §Who emits what.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Code {
    // --- Parse (tmark-syntax) ---
    /// Spec §Roles: an attribute list with no host element.
    AttrNoHost,
    /// Spec §Roles: a role head not followed by `[` or `(`.
    RoleDanglingHead,
    /// Design C4: a brace group followed by `[` or `(` whose name is not a
    /// role. Literal text; a hint against typos.
    RoleUnknown,
    /// Design C7: a `Kind:` line with no float to attach to.
    CaptionNoHost,
    /// Design C10: `{include}(…)` inside a paragraph; only the block form
    /// exists.
    IncludeInline,
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
    /// The tokenizer failed on the file (a bug in it, never in the input):
    /// the document is one paragraph of the text. AGENTS.md: "parsing
    /// never fails".
    ParseInternal,
    /// Appendix "PyMdownX compatibility profile": a spelling of the profile
    /// the parser recognises but does not implement yet (milestone 5), so
    /// it is literal text: critic markup, wiki links, fancy list markers.
    /// Loud rather than silent (P4).
    CompatUnsupported,
    /// Spec §Tabs: a `::: tab` outside `::: tabs`; wrapped in a `tabs` of
    /// one.
    ContainerOrphan,
    /// Spec §Table rung 5: the payload of a `yaml table` or
    /// `yaml table-config` fence is not YAML or not a mapping; the fence
    /// stays a code block.
    TableYaml,
    /// Spec §Table rung 5: a key the table schema does not know (top level,
    /// `table:`, a column, a cell, a separator, a named row). Python
    /// `extra="forbid"`.
    TableUnknownKey,
    /// Spec §Table rung 5: `columns` missing, not a list, fewer than two, a
    /// descriptor that is neither a name nor a mapping, a group without a
    /// name or without columns.
    TableColumns,
    /// Spec §Table: an `align` value outside `l|c|r|j` and their long forms.
    TableAlign,
    /// Spec §Table rung 5: a value of the wrong shape (a row that is neither
    /// a list nor a mapping, an empty row, a cell mapping without `value`,
    /// `rows`/`cols` below 1, `long` that is not a boolean or `auto`, a
    /// width that is not text).
    TableShape,
    /// Spec §Table rung 5: a row that does not cover the declared columns
    /// (extra cells, missing cells, a list longer than its group).
    TableRowWidth,
    /// Spec §Table rung 5: a span that collides with another, runs past the
    /// last column or row, or is not acknowledged with `~`.
    TableSpan,
    /// Spec §Table rung 5 (named-row mode): a key that names no top-level
    /// data column.
    TableColumnUnknown,
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
    /// Spec §Header: a reference to a heading's implicit id, which changes
    /// whenever the title is edited; write `{#id}`.
    RefImplicitId,
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
    /// Spec §Table rung 5: a `placement` that is not made of `hHtbpT!`.
    TablePlacement,
    /// Spec §Table: a width that is empty or a percentage outside (0, 100].
    TableWidth,
    /// Spec §Table: column percentages that add up to more than 100.
    TableWidthSum,
    /// Spec §Foreign directive: a dotted `::: a.b` directive, which the
    /// HTML and paged writers drop.
    DirectiveForeign,
    /// Spec §Emoji and icon shortcodes: a Material icon shortcode, which
    /// print drops.
    IconWebOnly,
    /// Spec §Feature registry: a spelling gated on a feature that is off
    /// (`^^x^^` without `inline.insert`), literal text.
    FeatureOff,
}

/// The pipeline stage that emits a code. Design `05-diagnostics.md` §Who
/// emits what.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Parse,
    Resolve,
    Lint,
}

impl Code {
    /// Every code, in catalogue order.
    pub const ALL: &'static [Code] = &[
        Code::AttrNoHost,
        Code::RoleDanglingHead,
        Code::RoleUnknown,
        Code::CaptionNoHost,
        Code::IncludeInline,
        Code::ContainerUnclosed,
        Code::ContainerUnknown,
        Code::FenceUnknownNodeWord,
        Code::FrontmatterYaml,
        Code::FrontmatterUnknownKey,
        Code::Deprecated,
        Code::ParseInternal,
        Code::CompatUnsupported,
        Code::ContainerOrphan,
        Code::TableYaml,
        Code::TableUnknownKey,
        Code::TableColumns,
        Code::TableAlign,
        Code::TableShape,
        Code::TableRowWidth,
        Code::TableSpan,
        Code::TableColumnUnknown,
        Code::RefUnresolved,
        Code::RefAmbiguous,
        Code::PrefixUnknown,
        Code::PrefixHostMismatch,
        Code::LabelDuplicate,
        Code::CitationShadowedByFootnote,
        Code::CrossrefInventoryMissing,
        Code::CrossrefInventoryStale,
        Code::IncludeMissing,
        Code::RefImplicitId,
        Code::HardcodedNumber,
        Code::PositionWord,
        Code::CaptionIdOffConvention,
        Code::StrictXConstruct,
        Code::DeprecatedFrontmatterKey,
        Code::LeadPromotion,
        Code::HeadingSkip,
        Code::TablePlacement,
        Code::TableWidth,
        Code::TableWidthSum,
        Code::DirectiveForeign,
        Code::IconWebOnly,
        Code::FeatureOff,
    ];

    /// The code with this kebab-case identifier.
    pub fn from_id(id: &str) -> Option<Code> {
        Code::ALL.iter().copied().find(|c| c.id() == id)
    }

    /// The stable kebab-case identifier printed by the CLI and shown by the
    /// LSP.
    pub fn id(self) -> &'static str {
        match self {
            Code::AttrNoHost => "attr-no-host",
            Code::RoleDanglingHead => "role-dangling-head",
            Code::RoleUnknown => "role-unknown",
            Code::CaptionNoHost => "caption-no-host",
            Code::IncludeInline => "include-inline",
            Code::ContainerUnclosed => "container-unclosed",
            Code::ContainerUnknown => "container-unknown",
            Code::FenceUnknownNodeWord => "fence-unknown-node-word",
            Code::FrontmatterYaml => "frontmatter-yaml",
            Code::FrontmatterUnknownKey => "frontmatter-unknown-key",
            Code::Deprecated => "deprecated",
            Code::ParseInternal => "parse-internal",
            Code::CompatUnsupported => "compat-unsupported",
            Code::ContainerOrphan => "container-orphan",
            Code::TableYaml => "table-yaml",
            Code::TableUnknownKey => "table-unknown-key",
            Code::TableColumns => "table-columns",
            Code::TableAlign => "table-align",
            Code::TableShape => "table-shape",
            Code::TableRowWidth => "table-row-width",
            Code::TableSpan => "table-span",
            Code::TableColumnUnknown => "table-column-unknown",
            Code::RefUnresolved => "ref-unresolved",
            Code::RefAmbiguous => "ref-ambiguous",
            Code::PrefixUnknown => "prefix-unknown",
            Code::PrefixHostMismatch => "prefix-host-mismatch",
            Code::LabelDuplicate => "label-duplicate",
            Code::CitationShadowedByFootnote => "citation-shadowed-by-footnote",
            Code::CrossrefInventoryMissing => "crossref-inventory-missing",
            Code::CrossrefInventoryStale => "crossref-inventory-stale",
            Code::IncludeMissing => "include-missing",
            Code::RefImplicitId => "ref-implicit-id",
            Code::HardcodedNumber => "hardcoded-number",
            Code::PositionWord => "position-word",
            Code::CaptionIdOffConvention => "caption-id-off-convention",
            Code::StrictXConstruct => "strict-x-construct",
            Code::DeprecatedFrontmatterKey => "deprecated-frontmatter-key",
            Code::LeadPromotion => "lead-promotion",
            Code::HeadingSkip => "heading-skip",
            Code::TablePlacement => "table-placement",
            Code::TableWidth => "table-width",
            Code::TableWidthSum => "table-width-sum",
            Code::DirectiveForeign => "directive-foreign",
            Code::IconWebOnly => "icon-web-only",
            Code::FeatureOff => "feature-off",
        }
    }

    /// Design `05-diagnostics.md` §Severities by default.
    pub fn default_severity(self) -> Severity {
        match self {
            Code::FrontmatterYaml
            | Code::FrontmatterUnknownKey
            | Code::StrictXConstruct
            | Code::ParseInternal
            | Code::TableYaml
            | Code::TableUnknownKey
            | Code::TableColumns
            | Code::TableAlign
            | Code::TableShape
            | Code::TableRowWidth
            | Code::TableSpan
            | Code::TableColumnUnknown
            | Code::TablePlacement
            | Code::TableWidth => Severity::Error,
            Code::AttrNoHost
            | Code::RoleDanglingHead
            | Code::CaptionNoHost
            | Code::IncludeInline
            | Code::ContainerUnclosed
            | Code::ContainerUnknown
            | Code::FenceUnknownNodeWord
            | Code::Deprecated
            | Code::CompatUnsupported
            | Code::RefUnresolved
            | Code::RefAmbiguous
            | Code::PrefixUnknown
            | Code::PrefixHostMismatch
            | Code::LabelDuplicate
            | Code::CitationShadowedByFootnote
            | Code::CrossrefInventoryMissing
            | Code::CrossrefInventoryStale
            | Code::IncludeMissing
            | Code::DeprecatedFrontmatterKey
            | Code::TableWidthSum => Severity::Warning,
            Code::LeadPromotion => Severity::Info,
            Code::RoleUnknown
            | Code::ContainerOrphan
            | Code::RefImplicitId
            | Code::HardcodedNumber
            | Code::PositionWord
            | Code::CaptionIdOffConvention
            | Code::HeadingSkip
            | Code::DirectiveForeign
            | Code::IconWebOnly
            | Code::FeatureOff => Severity::Hint,
        }
    }

    /// The stage that emits the code (the catalogue order of [`Code::ALL`]).
    pub fn stage(self) -> Stage {
        match self {
            Code::AttrNoHost
            | Code::RoleDanglingHead
            | Code::RoleUnknown
            | Code::CaptionNoHost
            | Code::IncludeInline
            | Code::ContainerUnclosed
            | Code::ContainerUnknown
            | Code::FenceUnknownNodeWord
            | Code::FrontmatterYaml
            | Code::FrontmatterUnknownKey
            | Code::Deprecated
            | Code::ParseInternal
            | Code::CompatUnsupported
            | Code::ContainerOrphan
            | Code::TableYaml
            | Code::TableUnknownKey
            | Code::TableColumns
            | Code::TableAlign
            | Code::TableShape
            | Code::TableRowWidth
            | Code::TableSpan
            | Code::TableColumnUnknown => Stage::Parse,
            Code::RefUnresolved
            | Code::RefAmbiguous
            | Code::PrefixUnknown
            | Code::PrefixHostMismatch
            | Code::LabelDuplicate
            | Code::CitationShadowedByFootnote
            | Code::CrossrefInventoryMissing
            | Code::CrossrefInventoryStale
            | Code::IncludeMissing
            | Code::RefImplicitId => Stage::Resolve,
            Code::HardcodedNumber
            | Code::PositionWord
            | Code::CaptionIdOffConvention
            | Code::StrictXConstruct
            | Code::DeprecatedFrontmatterKey
            | Code::LeadPromotion
            | Code::HeadingSkip
            | Code::TablePlacement
            | Code::TableWidth
            | Code::TableWidthSum
            | Code::DirectiveForeign
            | Code::IconWebOnly
            | Code::FeatureOff => Stage::Lint,
        }
    }

    /// One sentence on what the code reports, with the spec section it
    /// implements: the doc comment of the variant, for tools that list the
    /// catalogue (`tmark.codes()` in Python, editor settings).
    pub fn doc(self) -> &'static str {
        match self {
            Code::AttrNoHost => "Spec §Roles: an attribute list with no host element",
            Code::RoleDanglingHead => "Spec §Roles: a role head not followed by `[` or `(`",
            Code::RoleUnknown => {
                "Design C4: a brace group followed by `[` or `(` whose name is not a role"
            }
            Code::CaptionNoHost => "Design C7: a `Kind:` line with no float to attach to",
            Code::IncludeInline => {
                "Design C10: `{include}(…)` inside a paragraph; only the block form exists"
            }
            Code::ContainerUnclosed => {
                "Spec §Container directives: a `:::` fence without its closing line"
            }
            Code::ContainerUnknown => "Spec §Div: a `::: name` whose name is unknown",
            Code::FenceUnknownNodeWord => {
                "Spec §Data directives: an info string whose second word is not a node word"
            }
            Code::FrontmatterYaml => "Spec §Front matter: the YAML island does not parse",
            Code::FrontmatterUnknownKey => {
                "Spec §Front matter: an unknown key under a validated namespace"
            }
            Code::Deprecated => "Appendix \"Deprecation schedule\": a deprecated spelling",
            Code::ParseInternal => {
                "The tokenizer failed on the file (a bug in it): the document is one paragraph"
            }
            Code::RefUnresolved => "Spec §Ref: a key found in no registry",
            Code::RefAmbiguous => "Spec §Cite: a key present in two registries",
            Code::PrefixUnknown => "Spec §CounterItem: an undeclared counter prefix",
            Code::PrefixHostMismatch => "Spec §Anchor: a prefix that disagrees with its host",
            Code::LabelDuplicate => "Spec §Counters: the same label defined twice",
            Code::CitationShadowedByFootnote => {
                "Spec §Cite: a `[^key]` citation shadowed by a real footnote"
            }
            Code::CrossrefInventoryMissing => {
                "Spec §Cross-document references: an inventory that cannot be loaded"
            }
            Code::CrossrefInventoryStale => {
                "Design 06-registries: an inventory whose hash no longer matches"
            }
            Code::IncludeMissing => "Spec §Includes: an included file that cannot be loaded",
            Code::HardcodedNumber => "Spec §Ref: \"Figure 3\" typed in prose",
            Code::PositionWord => "Spec §Ref: \"above\" or \"below\" used as a reference",
            Code::CaptionIdOffConvention => {
                "Spec §Anchor: a caption id without the recommended prefix"
            }
            Code::StrictXConstruct => {
                "Spec §Conformance and deviations: an X-class construct under the strict profile"
            }
            Code::DeprecatedFrontmatterKey => {
                "Appendix \"Deprecation schedule\": a deprecated front-matter key"
            }
            Code::LeadPromotion => {
                "Spec §Para: a paragraph that is one strong span promoted to a lead-in"
            }
            Code::CompatUnsupported => {
                "Appendix \"PyMdownX compatibility profile\": a spelling milestone 5 implements, literal today"
            }
            Code::ContainerOrphan => "Spec §Tabs: a `::: tab` outside `::: tabs`, wrapped in a set of one",
            Code::RefImplicitId => {
                "Spec §Header: a reference to a heading's implicit id, which changes with the title"
            }
            Code::DirectiveForeign => {
                "Spec §Foreign directive: a dotted `:::` directive the HTML and paged writers drop"
            }
            Code::IconWebOnly => {
                "Spec §Emoji and icon shortcodes: a Material icon shortcode, dropped in print"
            }
            Code::FeatureOff => {
                "Spec §Feature registry: a spelling gated on a feature that is off, literal text"
            }
            Code::HeadingSkip => "Spec §Header: a heading level skipped",
            Code::TableYaml => "Spec §Table rung 5: a `yaml table` payload that is not a YAML mapping",
            Code::TableUnknownKey => "Spec §Table rung 5: an unknown key in a `yaml table` fence",
            Code::TableColumns => "Spec §Table rung 5: a `columns` list that is missing, empty or malformed",
            Code::TableAlign => "Spec §Table rung 5: an unknown column alignment",
            Code::TableShape => "Spec §Table rung 5: a row or cell of an unexpected shape",
            Code::TableRowWidth => "Spec §Table rung 5: a row with more or fewer cells than the columns",
            Code::TableSpan => "Spec §Table rung 5: a `rows`/`cols` span that overlaps or overruns",
            Code::TableColumnUnknown => "Spec §Table rung 5: a named row addressing an unknown column",
            Code::TablePlacement => "Spec §Table rung 5: a `placement` that is not made of `hHtbpT!`",
            Code::TableWidth => "Spec §Table: a width that is empty or a percentage outside (0, 100]",
            Code::TableWidthSum => "Spec §Table: column percentages adding up to more than 100",
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
    fn every_code_is_documented_and_staged() {
        let mut stage = Stage::Parse;
        for code in Code::ALL {
            assert!(!code.doc().is_empty(), "{}: no doc", code.id());
            assert!(
                code.stage() >= stage,
                "{}: out of catalogue order",
                code.id()
            );
            stage = code.stage();
        }
        assert_eq!(Code::from_id("heading-skip"), Some(Code::HeadingSkip));
        assert_eq!(Code::from_id("nope"), None);
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
