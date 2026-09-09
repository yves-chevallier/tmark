//! The YAML front matter: raw text, the typed keys TMark reads, and
//! everything else preserved as JSON.
//!
//! Spec §Front matter. Design: `design/03-ir.md` §Front matter. Any key may
//! sit at the root or under `press`; `press` wins. Keys TMark does not own
//! (`template`, `callouts`, `code`, `slots`, `refs`, `details`, …) stay in
//! `extra` untouched.

use std::collections::BTreeMap;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use crate::node::Meta;
use crate::registry::Scope;

/// The front matter of a document. `raw` is the YAML text between the
/// fences, copied byte for byte by the printer (ADR 0004).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FrontMatter {
    /// Span of the whole island, fences included; default when absent.
    #[serde(flatten)]
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raw: String,
    #[serde(default)]
    pub keys: Keys,
    /// Root keys TMark does not own, plus `press` keys it does not own under
    /// `extra.press`. An object, empty when there is nothing.
    #[serde(default = "empty_object", skip_serializing_if = "is_empty_object")]
    pub extra: Value,
    /// Deprecated top-level spellings met while parsing (`bibliography`,
    /// `counters`, …), for the caller to report as `deprecated-frontmatter-key`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deprecated: Vec<String>,
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

fn is_empty_object(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.is_empty()) || v.is_null()
}

impl Default for FrontMatter {
    fn default() -> Self {
        FrontMatter {
            meta: Meta::default(),
            raw: String::new(),
            keys: Keys::default(),
            extra: empty_object(),
            deprecated: Vec::new(),
        }
    }
}

/// The typed subset of the front matter (spec §Front matter). Serialises to
/// the canonical layout: metadata at the root, the rest under `press`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Keys {
    /// `None` when absent (the first heading is promoted), `Some(None)` for
    /// `title: null` (spec §Header: opt out of promotion).
    #[serde(
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(with = "Option<String>")]
    pub title: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<Author>,
    /// ISO date, free text, or `commit`; kept as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Document identifier for cross-document references.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Document language (`fr`, `en-GB`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epigraph: Option<Epigraph>,
    #[serde(skip_serializing_if = "Press::is_empty")]
    pub press: Press,
}

/// Distinguishes a missing field from an explicit `null`.
fn double_option<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

/// Spec §Front matter: `authors: [{name, affiliation}]`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Author {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Spec §BlockQuote: `epigraph: {quote, source}` placed before the first
/// heading.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Epigraph {
    pub quote: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// The `press` namespace, restricted to the keys TMark reads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Press {
    /// What a top-level `#` maps to (`chapter`, `section`, …). Spec §Header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_level: Option<String>,
    #[serde(skip_serializing_if = "Declare::is_empty")]
    pub declare: Declare,
    #[serde(skip_serializing_if = "Sources::is_empty")]
    pub sources: Sources,
    /// Spec §Feature registry: dotted name to switch.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub features: BTreeMap<String, bool>,
}

impl Press {
    pub fn is_empty(&self) -> bool {
        self.base_level.is_none()
            && self.declare.is_empty()
            && self.sources.is_empty()
            && self.features.is_empty()
    }
}

/// `declare`: what things *are*. Spec §Front matter, §Counters, §Admonition,
/// §Glossary and acronyms.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Declare {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub counters: BTreeMap<String, CounterDecl>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub admonitions: BTreeMap<String, AdmonitionDecl>,
    /// Structure owned by TeXSmith for now; kept as JSON.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub glossary: Value,
    /// Structure owned by TeXSmith for now; kept as JSON.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub acronyms: Value,
}

impl Declare {
    pub fn is_empty(&self) -> bool {
        self.counters.is_empty()
            && self.admonitions.is_empty()
            && self.glossary.is_null()
            && self.acronyms.is_null()
    }
}

/// A user-declared (or overridden) counter. Spec §Counters.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct CounterDecl {
    /// Label word, used in references and diagnostics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Python format string over `n`, `prefix`, `key`; default `"{n}"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    /// Template a reference renders, with `{name}` and `{number}` fields.
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// A declared admonition type. Spec §Admonition.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AdmonitionDecl {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Section title under the `reference` strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Text of the "See page N" link, with a `{page}` field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Counter prefix of a theorem-type admonition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counter: Option<String>,
}

/// `sources`: where references resolve. Spec §Bibliography, §Cross-document
/// references.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Sources {
    /// A map of key to DOI URL or pybtex-shaped entry, or a list of `.bib`
    /// paths; kept as JSON, `tmark-registry` interprets it.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub bibliography: Value,
    /// Alias to inventory path.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub crossrefs: BTreeMap<String, String>,
}

impl Sources {
    pub fn is_empty(&self) -> bool {
        self.bibliography.is_null() && self.crossrefs.is_empty()
    }
}

/// Why the front matter could not be read. The caller reports it as
/// `frontmatter-yaml` and keeps `raw`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontMatterError {
    pub message: String,
    /// 1-based line and column inside `raw`, when the YAML parser knows them.
    pub line: Option<u32>,
    pub col: Option<u32>,
}

impl fmt::Display for FrontMatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.col) {
            (Some(l), Some(c)) => write!(f, "{}:{}: {}", l, c, self.message),
            _ => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for FrontMatterError {}

impl From<serde_yaml_ng::Error> for FrontMatterError {
    fn from(err: serde_yaml_ng::Error) -> Self {
        let location = err.location();
        FrontMatterError {
            message: err.to_string(),
            line: location.as_ref().map(|l| l.line() as u32),
            col: location.map(|l| l.column() as u32),
        }
    }
}

fn error(message: impl Into<String>) -> FrontMatterError {
    FrontMatterError {
        message: message.into(),
        line: None,
        col: None,
    }
}

/// Root-level metadata keys TMark owns.
const ROOT_KEYS: &[&str] = &[
    "title", "subtitle", "authors", "date", "id", "lang", "epigraph",
];
/// `press` keys TMark owns.
const PRESS_KEYS: &[&str] = &["base_level", "declare", "sources", "features"];
/// Deprecated top-level spellings and the group they moved to (Appendix
/// "Deprecation schedule").
const DEPRECATED_KEYS: &[(&str, &str)] = &[
    ("bibliography", "sources"),
    ("crossrefs", "sources"),
    ("counters", "declare"),
    ("admonitions", "declare"),
    ("glossary", "declare"),
    ("acronyms", "declare"),
];

/// Parses the YAML text between the `---` fences.
///
/// Every owned key is looked up under `press` first, then at the root; a
/// deprecated top-level spelling fills its canonical group when that group
/// does not already set it and is recorded in `deprecated`. Unknown keys are
/// kept in `extra`.
pub fn parse(raw: &str) -> Result<FrontMatter, FrontMatterError> {
    let value: Value = serde_yaml_ng::from_str(raw)?;
    let mut root = match value {
        Value::Null => Map::new(),
        Value::Object(map) => map,
        _ => return Err(error("front matter must be a YAML mapping")),
    };
    let mut press = match root.remove("press") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(map)) => map,
        Some(_) => return Err(error("`press` must be a mapping")),
    };

    let mut keys = Map::new();
    let mut press_keys = Map::new();
    for key in ROOT_KEYS {
        if let Some(v) = take(&mut press, &mut root, key) {
            keys.insert((*key).into(), v);
        }
    }
    for key in PRESS_KEYS {
        if let Some(v) = take(&mut press, &mut root, key) {
            press_keys.insert((*key).into(), v);
        }
    }

    let mut deprecated = Vec::new();
    for (key, group) in DEPRECATED_KEYS {
        let Some(v) = take(&mut press, &mut root, key) else {
            continue;
        };
        deprecated.push((*key).to_string());
        let group_map = press_keys
            .entry(*group)
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(group_map) = group_map else {
            return Err(error(format!("`{group}` must be a mapping")));
        };
        group_map.entry(*key).or_insert(v);
    }

    if let Some(date) = keys.get_mut("date") {
        // YAML may type a date-like scalar; the spec keeps it as text.
        if let Value::Number(n) = date {
            *date = Value::String(n.to_string());
        }
    }
    keys.insert("press".into(), Value::Object(press_keys));
    let keys: Keys = serde_json::from_value(Value::Object(keys))
        .map_err(|e| error(format!("invalid front matter: {e}")))?;

    if !press.is_empty() {
        root.insert("press".into(), Value::Object(press));
    }
    Ok(FrontMatter {
        meta: Meta::default(),
        raw: raw.to_string(),
        keys,
        extra: Value::Object(root),
        deprecated,
    })
}

/// `press` wins over the root; the loser is dropped.
fn take(press: &mut Map<String, Value>, root: &mut Map<String, Value>, key: &str) -> Option<Value> {
    let from_root = root.remove(key);
    press.remove(key).or(from_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_and_press_precedence() {
        let fm = parse(
            "title: Root title\ndate: 2025-03-15\nlang: fr\npress:\n  title: Press title\n  base_level: chapter\n  features: {figures.exec: true}\n",
        )
        .unwrap();
        assert_eq!(fm.keys.title, Some(Some("Press title".into())));
        assert_eq!(fm.keys.date.as_deref(), Some("2025-03-15"));
        assert_eq!(fm.keys.lang.as_deref(), Some("fr"));
        assert_eq!(fm.keys.press.base_level.as_deref(), Some("chapter"));
        assert_eq!(fm.keys.press.features.get("figures.exec"), Some(&true));
        assert!(fm.deprecated.is_empty());
        assert!(is_empty_object(&fm.extra));
    }

    #[test]
    fn title_null_is_distinct_from_absent() {
        assert_eq!(parse("title: null\n").unwrap().keys.title, Some(None));
        assert_eq!(parse("lang: en\n").unwrap().keys.title, None);
        assert_eq!(parse("").unwrap(), FrontMatter::default());
    }

    #[test]
    fn deprecated_keys_move_to_their_group() {
        let fm = parse(
            "counters:\n  fw: {name: Finding, format: \"FW-{n:02d}\", start: 1, scope: document, ref: \"{number}\"}\nbibliography:\n  ein05: https://doi.org/10.1002/andp.19053221004\npress:\n  crossrefs: {fwrev: build/fw.refs.json}\n  sources:\n    crossrefs: {other: x.json}\n",
        )
        .unwrap();
        assert_eq!(fm.deprecated, vec!["bibliography", "crossrefs", "counters"]);
        let fw = &fm.keys.press.declare.counters["fw"];
        assert_eq!(fw.name.as_deref(), Some("Finding"));
        assert_eq!(fw.scope, Some(Scope::Document));
        assert_eq!(fw.reference.as_deref(), Some("{number}"));
        assert_eq!(
            fm.keys.press.sources.bibliography["ein05"],
            "https://doi.org/10.1002/andp.19053221004"
        );
        // The canonical `sources.crossrefs` wins over the deprecated spelling.
        assert_eq!(fm.keys.press.sources.crossrefs["other"], "x.json");
        assert!(!fm.keys.press.sources.crossrefs.contains_key("fwrev"));
    }

    #[test]
    fn unknown_keys_are_preserved() {
        let fm = parse(
            "title: T\nauthors: [{name: Ada, affiliation: AE}]\ntags: [a, b]\npress:\n  template: book\n  callouts: {style: fancy}\n  declare:\n    admonitions:\n      solution: {name: Solution, group: Solutions}\n",
        )
        .unwrap();
        assert_eq!(fm.keys.authors[0].name, "Ada");
        assert_eq!(fm.extra["tags"], serde_json::json!(["a", "b"]));
        assert_eq!(fm.extra["press"]["template"], "book");
        assert_eq!(fm.extra["press"]["callouts"]["style"], "fancy");
        assert!(fm.extra.get("title").is_none());
        assert_eq!(
            fm.keys.press.declare.admonitions["solution"]
                .group
                .as_deref(),
            Some("Solutions")
        );
        // Round trip through JSON keeps everything.
        let json = serde_json::to_value(&fm).unwrap();
        assert_eq!(
            json["keys"]["press"]["declare"]["admonitions"]["solution"]["name"],
            "Solution"
        );
        let back: FrontMatter = serde_json::from_value(json).unwrap();
        assert_eq!(back, fm);
    }

    #[test]
    fn errors_carry_a_location() {
        let err = parse("title: [unclosed\n").unwrap_err();
        assert!(err.line.is_some());
        assert!(parse("- a list\n").is_err());
        assert!(parse("press: 3\n").is_err());
        assert!(parse("authors: 3\n").is_err());
    }
}
