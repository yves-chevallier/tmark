//! The closed registries: roles, node words, predeclared counter prefixes,
//! admonition types, features and deprecated spellings.
//!
//! Design: `design/03-ir.md` §Closed registries. Each table is a `const`
//! slice of a small struct; grammars, completion lists, lint messages and
//! documentation are generated from them and no other crate hard-codes a
//! role name (AGENTS.md, SSOT).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------

/// How a role takes its payload. Spec §Roles: "brackets hold content,
/// parentheses hold a verbatim argument; each role accepts one form or the
/// other, never both".
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ArgStyle {
    /// One bracket group of Markdown content: `{aside}[…]`.
    Content,
    /// One or more bracket groups: `{index}[a][b]`.
    ContentMany,
    /// One parenthesised verbatim argument: `{raw latex}(…)`.
    Argument,
}

/// One entry of the role registry. Spec §Roles, §Node catalogue.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Role {
    /// The bare identifier after `{`.
    pub name: &'static str,
    pub arg: ArgStyle,
    /// The key the positional argument stands for: `{code py}` is
    /// `{code lang=py}`. `None` when the role takes no positional argument.
    pub principal: Option<&'static str>,
    /// Accepted `key=value` keys, the principal one included.
    pub keys: &'static [&'static str],
    /// The IR node the role produces (a variant name of `Inline` or `Block`).
    pub node: &'static str,
    /// The canonical role that replaces a deprecated one (Appendix
    /// "Deprecation schedule"); `None` for a canonical role.
    pub replaced_by: Option<&'static str>,
}

const fn mk_role(
    name: &'static str,
    arg: ArgStyle,
    principal: Option<&'static str>,
    keys: &'static [&'static str],
    node: &'static str,
) -> Role {
    Role {
        name,
        arg,
        principal,
        keys,
        node,
        replaced_by: None,
    }
}

const fn deprecated_role(
    name: &'static str,
    arg: ArgStyle,
    node: &'static str,
    replaced_by: &'static str,
) -> Role {
    Role {
        name,
        arg,
        principal: None,
        keys: &[],
        node,
        replaced_by: Some(replaced_by),
    }
}

/// Spec §Roles, §Inline text, §Notes, §Anchors, references, citations, §Raw
/// passthrough, §Includes.
pub const ROLES: &[Role] = &[
    mk_role("lead", ArgStyle::Content, None, &[], "Para"),
    mk_role("sc", ArgStyle::Content, None, &[], "SmallCaps"),
    mk_role("del", ArgStyle::Content, None, &[], "Strikeout"),
    mk_role("underline", ArgStyle::Content, None, &[], "Underline"),
    mk_role("mark", ArgStyle::Content, None, &[], "Highlight"),
    mk_role("sub", ArgStyle::Content, None, &[], "Subscript"),
    mk_role("sup", ArgStyle::Content, None, &[], "Superscript"),
    mk_role("keys", ArgStyle::Content, None, &[], "Keystroke"),
    mk_role("code", ArgStyle::Content, Some("lang"), &["lang"], "Code"),
    mk_role("aside", ArgStyle::Content, Some("side"), &["side"], "Aside"),
    mk_role(
        "index",
        ArgStyle::ContentMany,
        None,
        &["main", "registry"],
        "IndexEntry",
    ),
    mk_role("counter", ArgStyle::Argument, None, &[], "CounterItem"),
    mk_role(
        "raw",
        ArgStyle::Argument,
        Some("backend"),
        &["backend"],
        "RawInline",
    ),
    mk_role("include", ArgStyle::Argument, None, &["base"], "Include"),
    // Deprecated spellings, still parsed (Appendix "Deprecation schedule").
    deprecated_role("margin", ArgStyle::Content, "Aside", "aside"),
    deprecated_role("latex", ArgStyle::Content, "RawInline", "raw"),
    deprecated_role("typst", ArgStyle::Content, "RawInline", "raw"),
    deprecated_role("html", ArgStyle::Content, "RawInline", "raw"),
];

/// Looks a role up by its exact name.
pub fn role(name: &str) -> Option<&'static Role> {
    ROLES.iter().find(|r| r.name == name)
}

// ---------------------------------------------------------------------------
// Node words
// ---------------------------------------------------------------------------

/// The second word of a data directive's info string. Spec §Data directives.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeWord {
    pub word: &'static str,
    /// The block node produced.
    pub node: &'static str,
}

/// Spec §Data directives: "Node words: `code` (default), `table`,
/// `table-config`, `image`, `raw`."
pub const NODE_WORDS: &[NodeWord] = &[
    NodeWord {
        word: "code",
        node: "CodeBlock",
    },
    NodeWord {
        word: "table",
        node: "Table",
    },
    NodeWord {
        word: "table-config",
        node: "TableConfig",
    },
    NodeWord {
        word: "image",
        node: "Image",
    },
    NodeWord {
        word: "raw",
        node: "RawBlock",
    },
];

/// Languages whose bare fence does not produce `code`. Spec §Data
/// directives: "a bare `mermaid` fence produces an image".
pub const LANG_DEFAULT_NODE_WORDS: &[(&str, &str)] = &[("mermaid", "image")];

pub fn node_word(word: &str) -> Option<&'static NodeWord> {
    NODE_WORDS.iter().find(|n| n.word == word)
}

/// The node word a fence with only a language produces.
pub fn default_node_word(lang: &str) -> &'static NodeWord {
    let word = LANG_DEFAULT_NODE_WORDS
        .iter()
        .find(|(l, _)| *l == lang)
        .map_or("code", |(_, w)| *w);
    node_word(word).expect("default node words are registered")
}

// ---------------------------------------------------------------------------
// Counter prefixes
// ---------------------------------------------------------------------------

/// Numbering scope of a counter. Spec §Counters (`scope`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Document,
    Chapter,
    Section,
}

/// A predeclared entry of the counter registry. Spec §Counters, Table
/// "Predeclared counter prefixes".
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Prefix {
    pub name: &'static str,
    /// The label word ("Figure"); `None` for `gls` and `doi`, which number
    /// nothing.
    pub label: Option<&'static str>,
    /// `None` for `gls` and `doi`.
    pub scope: Option<Scope>,
    /// The `ref` template with `{name}` and `{number}` fields.
    pub reference: Option<&'static str>,
}

const fn mk_prefix(name: &'static str, label: &'static str, scope: Scope) -> Prefix {
    Prefix {
        name,
        label: Some(label),
        scope: Some(scope),
        reference: Some("{name} {number}"),
    }
}

/// Spec §Counters.
pub const PREFIXES: &[Prefix] = &[
    mk_prefix("part", "Part", Scope::Document),
    mk_prefix("chap", "Chapter", Scope::Document),
    mk_prefix("sec", "Section", Scope::Document),
    mk_prefix("app", "Appendix", Scope::Document),
    mk_prefix("fig", "Figure", Scope::Chapter),
    mk_prefix("tbl", "Table", Scope::Chapter),
    mk_prefix("lst", "Listing", Scope::Chapter),
    mk_prefix("eq", "Equation", Scope::Chapter),
    mk_prefix("thm", "Theorem", Scope::Chapter),
    mk_prefix("note", "Note", Scope::Document),
    Prefix {
        name: "gls",
        label: None,
        scope: None,
        reference: None,
    },
    Prefix {
        name: "doi",
        label: None,
        scope: None,
        reference: None,
    },
];

/// Looks a predeclared prefix up, case-insensitively (spec §Counters:
/// "matched case-insensitively").
pub fn prefix(name: &str) -> Option<&'static Prefix> {
    PREFIXES.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

// ---------------------------------------------------------------------------
// Admonitions
// ---------------------------------------------------------------------------

/// A built-in admonition type. Spec §Admonition (callout).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Admonition {
    /// The type word after `:::` or `!!!`.
    pub name: &'static str,
    /// The default title word.
    pub label: &'static str,
    /// The counter prefix of a theorem-type admonition.
    pub counter: Option<&'static str>,
}

const fn mk_admonition(name: &'static str, label: &'static str) -> Admonition {
    Admonition {
        name,
        label,
        counter: None,
    }
}

const fn mk_theorem(name: &'static str, label: &'static str) -> Admonition {
    Admonition {
        name,
        label,
        counter: Some("thm"),
    }
}

/// Spec §Admonition: the built-in types and the predeclared theorem types
/// (`proof` has no counter).
pub const ADMONITIONS: &[Admonition] = &[
    mk_admonition("note", "Note"),
    mk_admonition("tip", "Tip"),
    mk_admonition("warning", "Warning"),
    mk_admonition("important", "Important"),
    mk_admonition("danger", "Danger"),
    mk_admonition("info", "Info"),
    mk_admonition("hint", "Hint"),
    mk_admonition("seealso", "See also"),
    mk_admonition("question", "Question"),
    mk_admonition("abstract", "Abstract"),
    mk_theorem("theorem", "Theorem"),
    mk_theorem("lemma", "Lemma"),
    mk_theorem("corollary", "Corollary"),
    mk_theorem("proposition", "Proposition"),
    mk_theorem("definition", "Definition"),
    mk_admonition("proof", "Proof"),
];

pub fn admonition(name: &str) -> Option<&'static Admonition> {
    ADMONITIONS.iter().find(|a| a.name == name)
}

// ---------------------------------------------------------------------------
// Features
// ---------------------------------------------------------------------------

/// A switchable behaviour. Spec §Feature registry and extensibility.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Feature {
    /// Dotted name, as written under `features:`.
    pub name: &'static str,
    pub default: bool,
    /// The spec section that defines the effect.
    pub section: &'static str,
    pub effect: &'static str,
}

const fn mk_feature(
    name: &'static str,
    default: bool,
    section: &'static str,
    effect: &'static str,
) -> Feature {
    Feature {
        name,
        default,
        section,
        effect,
    }
}

/// Spec Table "The feature registry".
pub const FEATURES: &[Feature] = &[
    mk_feature(
        "paragraph.lead",
        true,
        "Para",
        "promote a leading short strong span to {lead}[…]",
    ),
    mk_feature(
        "table.decimal-align",
        true,
        "Table",
        "align numeric right-aligned columns on the decimal point",
    ),
    mk_feature(
        "tasklist.partial",
        false,
        "BulletList, OrderedList",
        "`- [.]` partial task items",
    ),
    mk_feature(
        "figures.exec",
        false,
        "Image, Figure",
        "execute `python image` fences",
    ),
    mk_feature(
        "glossary.wikipedia",
        false,
        "Glossary and acronyms",
        "fetch glossary summaries from Wikipedia links",
    ),
    mk_feature("inline.insert", false, "Inline text", "`^^x^^` as <ins>"),
    mk_feature(
        "compat.pymdownx",
        true,
        "PyMdownX compatibility profile",
        "accept the PyMdownX sugar; off under strict",
    ),
];

pub fn feature(name: &str) -> Option<&'static Feature> {
    FEATURES.iter().find(|f| f.name == name)
}

// ---------------------------------------------------------------------------
// Deprecations
// ---------------------------------------------------------------------------

/// When a deprecated spelling stops being accepted. Spec Appendix
/// "Deprecation schedule".
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum Horizon {
    /// Removed once `tmark fmt` ships and can rewrite it.
    Fmt,
    /// Part of the compatibility promise; not scheduled for removal.
    Indefinite,
    /// Never shipped in a release; accepted for draft-2 documents only.
    NeverShipped,
}

/// One row of the deprecation table.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Deprecation {
    /// Stable identifier used by parsers and lint messages.
    pub id: &'static str,
    /// The deprecated spelling, as the table writes it.
    pub spelling: &'static str,
    /// The canonical replacement.
    pub canonical: &'static str,
    /// The draft that deprecated it (`"draft 2"`, `"draft 3"`, `"none"`).
    pub status: &'static str,
    pub horizon: Horizon,
}

const fn dep(
    id: &'static str,
    spelling: &'static str,
    canonical: &'static str,
    status: &'static str,
    horizon: Horizon,
) -> Deprecation {
    Deprecation {
        id,
        spelling,
        canonical,
        status,
        horizon,
    }
}

/// Spec Table "Deprecated spellings and their horizons", in table order.
pub const DEPRECATIONS: &[Deprecation] = &[
    dep(
        "counter-brace",
        "#{prefix:key}",
        "#(prefix:key)",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "pandoc-citation",
        "[@key, locator]",
        "@[key, locator]",
        "draft 3",
        Horizon::Indefinite,
    ),
    dep(
        "footnote-citation",
        "[^key], ^[k1,k2]",
        "@key, @[k1; k2]",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "backend-role",
        "{latex}[…], {typst}[…], {html}[…]",
        "{raw latex}(…)",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "slash-raw-block",
        "/// latex … ///",
        "latex raw fence",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "render-fence",
        "latex render fence",
        "latex raw",
        "draft 3",
        Horizon::NeverShipped,
    ),
    dep(
        "slash-caption",
        "/// caption, /// figure-caption",
        "Kind: … {#id} caption line",
        "draft 2",
        Horizon::Fmt,
    ),
    dep(
        "index-colon-registry",
        "{index:registry}[…]",
        "{index registry=…}[…]",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "index-suffix",
        "{index}[…]{b} / {i}",
        "{index main=true}[…] / content markup",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "margin-role",
        "{margin}[…], {margin}[…]{l} / {r} / {o} / {i}",
        "{aside}[…], {aside side=left}[…]",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "margin-container",
        "::: margin",
        "::: aside",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "snippet",
        "--8<-- \"file\"",
        "{include}(file)",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "doi-url",
        "@https://doi.org/…",
        "@doi:…",
        "draft 3",
        Horizon::Indefinite,
    ),
    dep(
        "gls-link",
        "[](gls:term)",
        "@gls:term",
        "draft 2",
        Horizon::Fmt,
    ),
    dep(
        "bare-mermaid",
        "bare mermaid fence",
        "mermaid image",
        "draft 3",
        Horizon::Indefinite,
    ),
    dep(
        "caption-before",
        "Table: line before the table",
        "Table: line after",
        "draft 3",
        Horizon::Indefinite,
    ),
    dep(
        "bang-callout",
        "!!! / ??? callouts",
        "::: type {…}",
        "draft 2",
        Horizon::Indefinite,
    ),
    dep(
        "frontmatter-sources",
        "top-level bibliography, crossrefs",
        "sources.*",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "frontmatter-declare",
        "top-level counters, admonitions, glossary, acronyms",
        "declare.*",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "admonition-style",
        "admonitions.<type>.icon / .color",
        "press.callouts.<type>",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "callout-style",
        "press.callout_style",
        "press.callouts.style",
        "draft 3",
        Horizon::Fmt,
    ),
    dep(
        "no-promote-title",
        "--no-promote-title CLI flag",
        "title: null",
        "none",
        Horizon::Indefinite,
    ),
];

pub fn deprecation(id: &str) -> Option<&'static Deprecation> {
    DEPRECATIONS.iter().find(|d| d.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles() {
        let code = role("code").unwrap();
        assert_eq!(code.arg, ArgStyle::Content);
        assert_eq!(code.principal, Some("lang"));
        assert_eq!(role("index").unwrap().arg, ArgStyle::ContentMany);
        assert_eq!(role("raw").unwrap().arg, ArgStyle::Argument);
        assert_eq!(role("margin").unwrap().replaced_by, Some("aside"));
        assert!(role("foo").is_none());
        assert!(role("Code").is_none(), "role names are case-sensitive");
        for r in ROLES {
            if let Some(p) = r.principal {
                assert!(r.keys.contains(&p), "{}: principal key is accepted", r.name);
            }
        }
    }

    #[test]
    fn node_words() {
        assert_eq!(node_word("table-config").unwrap().node, "TableConfig");
        assert_eq!(default_node_word("python").word, "code");
        assert_eq!(default_node_word("mermaid").word, "image");
        assert!(node_word("render").is_none());
    }

    #[test]
    fn prefixes() {
        assert_eq!(prefix("fig").unwrap().label, Some("Figure"));
        assert_eq!(prefix("Fig").unwrap().name, "fig");
        assert_eq!(prefix("gls").unwrap().scope, None);
        assert!(prefix("fw").is_none());
        for p in PREFIXES {
            assert!(
                role(p.name).is_none(),
                "{}: a prefix never shadows a role",
                p.name
            );
        }
    }

    #[test]
    fn admonitions_features_deprecations() {
        assert_eq!(admonition("lemma").unwrap().counter, Some("thm"));
        assert_eq!(admonition("proof").unwrap().counter, None);
        assert!(admonition("solution").is_none());
        assert!(feature("paragraph.lead").unwrap().default);
        assert!(!feature("figures.exec").unwrap().default);
        assert_eq!(FEATURES.len(), 7);
        assert_eq!(deprecation("margin-role").unwrap().horizon, Horizon::Fmt);
        assert_eq!(DEPRECATIONS.len(), 22);
        let mut ids: Vec<_> = DEPRECATIONS.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), DEPRECATIONS.len(), "deprecation ids are unique");
    }
}
