//! The `yaml table` and `yaml table-config` payloads: a port of
//! `texsmith.extensions.tables.schema` (`parse_table`, `parse_table_config`,
//! `build_matrix`) that never fails. Spec §Table rung 5.
//!
//! Pure: YAML text in, a dense leaf matrix of text cells plus the
//! diagnostics the Python validator would have raised out. The caller
//! (`lower/table.rs`) parses the cell text as inline Markdown and reports
//! the diagnostics on the fence. Only an unparseable payload is an `Err`:
//! the fence then stays a code block (design 05, `table-yaml`).
//!
//! Where the Python validator is looser or stricter by accident, this port
//! takes the rule the TeXSmith documentation states (design 12, C27):
//!
//! - every slot under a row span is acknowledged with `~`, inside a group
//!   list too; the Python `skip_absorbed` reading (no `~` after a placed
//!   cell) is accepted as a fallback so every row Python accepts is
//!   accepted with the same meaning;
//! - the first top-level column is a column like the others: its cell may
//!   be rich or a group list (Python stringifies the label);
//! - a scalar or list addressed to a partly consumed group fills the
//!   group's *remaining* leaves (Python overruns the group);
//! - in named-row mode an omitted column under a span is simply left
//!   absorbed (Python miscounts it as an extra cell).

use serde_yaml_ng::{Mapping, Value};
use tmark_ir::{Align, Code, Column, ColumnConfig, ColumnGroup, LeafColumn, TableSettings};

/// A diagnostic the validator would have raised: code and message; the
/// span is the fence, added by the caller.
pub type Finding = (Code, String);

/// One slot of the leaf matrix before its text is parsed as Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawCell {
    pub text: String,
    pub rows: u32,
    pub cols: u32,
    pub align: Option<Align>,
    pub absorbed: bool,
}

impl RawCell {
    fn empty() -> Self {
        RawCell {
            text: String::new(),
            rows: 1,
            cols: 1,
            align: None,
            absorbed: false,
        }
    }

    fn absorbed() -> Self {
        RawCell {
            absorbed: true,
            ..RawCell::empty()
        }
    }

    fn scalar(text: String) -> Self {
        RawCell {
            text,
            ..RawCell::empty()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawRow {
    Data {
        cells: Vec<RawCell>,
        named: bool,
    },
    Separator {
        label: Option<String>,
        double_rule: bool,
    },
}

/// `parse_table`'s result: the model with text cells, and what was wrong.
#[derive(Debug, Default)]
pub struct RawTable {
    pub settings: TableSettings,
    pub columns: Vec<Column>,
    pub rows: Vec<RawRow>,
    pub footer: Vec<RawRow>,
    pub findings: Vec<Finding>,
}

/// `parse_table_config`'s result.
#[derive(Debug, Default)]
pub struct RawConfig {
    pub settings: TableSettings,
    pub columns: Vec<ColumnConfig>,
    pub findings: Vec<Finding>,
}

// ---------------------------------------------------------------- entry

/// Python `parse_table`. `Err` only when the text is not YAML or not a
/// mapping.
pub fn parse_table(text: &str) -> Result<RawTable, String> {
    let root = mapping(text, "yaml table")?;
    let mut out = RawTable::default();
    let mut findings = Vec::new();
    unknown_keys(
        &root,
        &["table", "columns", "rows", "footer"],
        "the `yaml table` payload",
        &mut findings,
    );
    out.settings = settings(root.get("table"), &mut findings);
    match root.get("columns") {
        None => findings.push((
            Code::TableColumns,
            "the `yaml table` payload has no `columns`".to_string(),
        )),
        Some(Value::Sequence(items)) => {
            out.columns = items
                .iter()
                .filter_map(|c| column(c, &mut findings))
                .collect();
        }
        Some(_) => findings.push((
            Code::TableColumns,
            "`columns` must be a list of column descriptors".to_string(),
        )),
    }
    if root.contains_key("columns") && out.columns.len() < 2 {
        findings.push((
            Code::TableColumns,
            format!(
                "a `yaml table` needs at least two columns, {} declared",
                out.columns.len()
            ),
        ));
    }
    let spans = top_level_spans(&out.columns);
    let body = rows(root.get("rows"), "rows", &out.columns, &mut findings);
    let footer = rows(root.get("footer"), "footer", &out.columns, &mut findings);
    let body_len = body.len();
    out.rows = expand_section(body, &spans, "body", 0, &mut findings);
    out.footer = expand_section(footer, &spans, "footer", body_len, &mut findings);
    out.findings = findings;
    Ok(out)
}

/// Python `parse_table_config`. `Err` only when the text is not YAML or
/// not a mapping (an empty payload is an empty mapping).
pub fn parse_table_config(text: &str) -> Result<RawConfig, String> {
    let root = mapping(text, "yaml table-config")?;
    let mut findings = Vec::new();
    unknown_keys(
        &root,
        &["table", "columns"],
        "the `yaml table-config` payload",
        &mut findings,
    );
    let settings = settings(root.get("table"), &mut findings);
    let columns = match root.get("columns") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Sequence(items)) => items
            .iter()
            .map(|item| match item {
                Value::Null => ColumnConfig::default(),
                Value::Mapping(map) => {
                    unknown_keys(map, CONFIG_KEYS, "a column entry", &mut findings);
                    config(map, &mut findings)
                }
                other => {
                    findings.push((
                        Code::TableShape,
                        format!(
                            "a `table-config` column entry must be a mapping, got {}",
                            describe(other)
                        ),
                    ));
                    ColumnConfig::default()
                }
            })
            .collect(),
        Some(_) => {
            findings.push((
                Code::TableShape,
                "`columns` must be a list of column entries".to_string(),
            ));
            Vec::new()
        }
    };
    Ok(RawConfig {
        settings,
        columns,
        findings,
    })
}

fn mapping(text: &str, what: &str) -> Result<Mapping, String> {
    let value: Value = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
    match value {
        Value::Mapping(map) => Ok(map),
        Value::Null if what == "yaml table-config" => Ok(Mapping::new()),
        Value::Null => Err("is empty".to_string()),
        other => Err(format!("must be a mapping, got {}", describe(&other))),
    }
}

// ------------------------------------------------------------- settings

/// Python `TableSettings.model_validate(raw.get("table") or {})`.
fn settings(value: Option<&Value>, findings: &mut Vec<Finding>) -> TableSettings {
    let mut settings = TableSettings::default();
    let map = match value {
        None | Some(Value::Null) => return settings,
        Some(Value::Mapping(map)) => map,
        Some(other) => {
            findings.push((
                Code::TableShape,
                format!(
                    "the `table` section must be a mapping, got {}",
                    describe(other)
                ),
            ));
            return settings;
        }
    };
    unknown_keys(
        map,
        &["width", "placement", "long"],
        "the `table` section",
        findings,
    );
    if let Some(width) = map.get("width").and_then(|w| width(w, findings)) {
        settings.width = width;
    }
    match map.get("placement") {
        None | Some(Value::Null) => {}
        Some(Value::String(s)) => settings.placement = Some(s.clone()),
        Some(other) => findings.push((
            Code::TableShape,
            format!("`placement` must be text, got {}", describe(other)),
        )),
    }
    match map.get("long") {
        None | Some(Value::Null) => {}
        Some(Value::Bool(b)) => settings.long = Some(*b),
        Some(Value::String(s)) if s == "auto" => {}
        Some(other) => findings.push((
            Code::TableShape,
            format!(
                "`long` must be `true`, `false` or `auto`, got {}",
                describe(other)
            ),
        )),
    }
    settings
}

/// Python `_validate_width`: text, trimmed, `x` normalised to `X`. The
/// range of a percentage and emptiness are the lint `table-width`.
fn width(value: &Value, findings: &mut Vec<Finding>) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => {
            let trimmed = s.trim();
            Some(if trimmed.eq_ignore_ascii_case("x") {
                "X".to_string()
            } else {
                trimmed.to_string()
            })
        }
        other => {
            findings.push((
                Code::TableShape,
                format!("`width` must be text, got {}", describe(other)),
            ));
            None
        }
    }
}

// -------------------------------------------------------------- columns

const CONFIG_KEYS: &[&str] = &["align", "width", "width-group", "width_group"];
const LEAF_KEYS: &[&str] = &["name", "align", "width", "width-group", "width_group"];
const GROUP_KEYS: &[&str] = &[
    "name",
    "columns",
    "align",
    "width",
    "width-group",
    "width_group",
];

/// Python `_ColumnAttrs`.
fn config(map: &Mapping, findings: &mut Vec<Finding>) -> ColumnConfig {
    let mut config = ColumnConfig::default();
    match map.get("align") {
        None | Some(Value::Null) => {}
        Some(Value::String(s)) => match Align::parse(s) {
            Some(align) => config.align = Some(align),
            None => findings.push((
                Code::TableAlign,
                format!("unknown `align` value `{s}`; expected l, c, r, j or a long form"),
            )),
        },
        Some(other) => findings.push((
            Code::TableShape,
            format!("`align` must be text, got {}", describe(other)),
        )),
    }
    if let Some(value) = map.get("width") {
        config.width = width(value, findings);
    }
    let group = map.get("width-group").or_else(|| map.get("width_group"));
    match group {
        None | Some(Value::Null) => {}
        Some(value) => match text(value) {
            Some(text) => config.width_group = Some(text),
            None => findings.push((
                Code::TableShape,
                format!("`width-group` must be a scalar, got {}", describe(value)),
            )),
        },
    }
    config
}

/// Python `_parse_column`.
fn column(value: &Value, findings: &mut Vec<Finding>) -> Option<Column> {
    match value {
        Value::String(_) | Value::Number(_) => Some(Column::Leaf(LeafColumn {
            name: text(value),
            config: ColumnConfig::default(),
        })),
        Value::Mapping(map) if map.contains_key("columns") => {
            unknown_keys(map, GROUP_KEYS, "a column group", findings);
            let name = match map.get("name") {
                None => {
                    findings.push((
                        Code::TableColumns,
                        "a column group is missing its `name`".to_string(),
                    ));
                    String::new()
                }
                Some(value) => text(value).unwrap_or_else(|| {
                    findings.push((
                        Code::TableShape,
                        format!("a column `name` must be a scalar, got {}", describe(value)),
                    ));
                    String::new()
                }),
            };
            let columns: Vec<Column> = match map.get("columns") {
                Some(Value::Sequence(items)) => {
                    items.iter().filter_map(|c| column(c, findings)).collect()
                }
                _ => Vec::new(),
            };
            if columns.is_empty() {
                findings.push((
                    Code::TableColumns,
                    format!("column group `{name}` has no columns"),
                ));
            }
            Some(Column::Group(ColumnGroup {
                name,
                columns,
                config: config(map, findings),
            }))
        }
        Value::Mapping(map) => {
            unknown_keys(map, LEAF_KEYS, "a column", findings);
            let name = match map.get("name") {
                None | Some(Value::Null) => None,
                Some(value) => {
                    let name = text(value);
                    if name.is_none() {
                        findings.push((
                            Code::TableShape,
                            format!("a column `name` must be a scalar, got {}", describe(value)),
                        ));
                    }
                    name
                }
            };
            Some(Column::Leaf(LeafColumn {
                name,
                config: config(map, findings),
            }))
        }
        other => {
            findings.push((
                Code::TableColumns,
                format!("invalid column descriptor {}", describe(other)),
            ));
            None
        }
    }
}

fn top_level_spans(columns: &[Column]) -> Vec<(usize, usize)> {
    let mut start = 0;
    columns
        .iter()
        .map(|column| {
            let len = column.leaf_count();
            let span = (start, len);
            start += len;
            span
        })
        .collect()
}

// ----------------------------------------------------------------- rows

/// A cell as written, before expansion (Python `CellTopValue`).
#[derive(Clone, Debug)]
enum Item {
    Null,
    Scalar(String),
    Rich(RawCell),
    /// Only at the top level of a row: the leaves of a group.
    List(Vec<Item>),
}

impl Item {
    fn is_null(&self) -> bool {
        matches!(self, Item::Null)
    }
}

/// A row as written.
enum Written {
    Separator {
        label: Option<String>,
        double_rule: bool,
    },
    /// One item per top-level column, the label first.
    Positional { label: String, items: Vec<Item> },
    /// One item per top-level data column; `None` for an omitted column.
    Named {
        label: String,
        items: Vec<Option<Item>>,
    },
}

/// Python `_parse_row` over a section.
fn rows(
    value: Option<&Value>,
    key: &str,
    columns: &[Column],
    findings: &mut Vec<Finding>,
) -> Vec<Written> {
    match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Sequence(items)) => items
            .iter()
            .enumerate()
            .filter_map(|(i, row)| self::row(row, i, key, columns, findings))
            .collect(),
        Some(other) => {
            findings.push((
                Code::TableShape,
                format!("`{key}` must be a list of rows, got {}", describe(other)),
            ));
            Vec::new()
        }
    }
}

fn row(
    value: &Value,
    index: usize,
    section: &str,
    columns: &[Column],
    findings: &mut Vec<Finding>,
) -> Option<Written> {
    match value {
        Value::Mapping(map) if map.contains_key("separator") => separator(map, findings),
        Value::Sequence(items) => {
            if items.is_empty() {
                findings.push((
                    Code::TableShape,
                    format!("`{section}` row {index} is empty"),
                ));
                return None;
            }
            let label = text(&items[0]).unwrap_or_else(|| format!("#{index}"));
            Some(Written::Positional {
                label: label.clone(),
                items: items
                    .iter()
                    .map(|item| item_top(item, &label, findings))
                    .collect(),
            })
        }
        Value::Mapping(map) => named_row(map, index, columns, findings),
        other => {
            findings.push((
                Code::TableShape,
                format!(
                    "`{section}` row {index} must be a list or a mapping, got {}",
                    describe(other)
                ),
            ));
            None
        }
    }
}

/// Python `_parse_separator`.
fn separator(map: &Mapping, findings: &mut Vec<Finding>) -> Option<Written> {
    let body = match map.get("separator") {
        Some(Value::Bool(true)) => {
            unknown_keys(
                map,
                &["separator", "label", "double-rule", "double_rule"],
                "a separator",
                findings,
            );
            map.clone()
        }
        Some(Value::Mapping(inner)) => {
            let extras: Vec<String> = map
                .keys()
                .filter_map(text)
                .filter(|k| k != "separator")
                .collect();
            if !extras.is_empty() {
                findings.push((
                    Code::TableShape,
                    format!(
                        "separator carries the keys {} alongside a mapping body; move them inside `separator`",
                        extras.join(", ")
                    ),
                ));
            }
            unknown_keys(
                inner,
                &["label", "double-rule", "double_rule"],
                "a separator",
                findings,
            );
            inner.clone()
        }
        Some(other) => {
            findings.push((
                Code::TableShape,
                format!(
                    "`separator` must be `true` or a mapping, got {}",
                    describe(other)
                ),
            ));
            return None;
        }
        None => unreachable!("checked by the caller"),
    };
    let label = match body.get("label") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let label = text(value);
            if label.is_none() {
                findings.push((
                    Code::TableShape,
                    format!("a separator `label` must be text, got {}", describe(value)),
                ));
            }
            label
        }
    };
    let double_rule = match body.get("double-rule").or_else(|| body.get("double_rule")) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(other) => {
            findings.push((
                Code::TableShape,
                format!("`double-rule` must be a boolean, got {}", describe(other)),
            ));
            false
        }
    };
    Some(Written::Separator { label, double_rule })
}

/// Python `_parse_named_row`.
fn named_row(
    map: &Mapping,
    index: usize,
    columns: &[Column],
    findings: &mut Vec<Finding>,
) -> Option<Written> {
    let (label, cells) = if map.contains_key("label") && map.contains_key("cells") {
        unknown_keys(map, &["label", "cells"], "a named row", findings);
        (
            map.get("label").and_then(text).unwrap_or_default(),
            map.get("cells").expect("checked"),
        )
    } else if map.len() == 1 {
        let (key, value) = map.iter().next().expect("one entry");
        (text(key).unwrap_or_else(|| format!("#{index}")), value)
    } else {
        findings.push((
            Code::TableShape,
            format!(
                "row {index} is neither a list nor a named row (one label key, or `label` and `cells`)"
            ),
        ));
        return None;
    };
    let Value::Mapping(cells) = cells else {
        findings.push((
            Code::TableShape,
            format!(
                "named row `{label}` must map column names to cell values, got {}",
                describe(cells)
            ),
        ));
        return None;
    };
    let data_columns = columns.get(1..).unwrap_or_default();
    let known: Vec<&str> = data_columns.iter().filter_map(Column::name).collect();
    for key in cells.keys() {
        let key = text(key).unwrap_or_default();
        if !known.contains(&key.as_str()) {
            let mut available = known.clone();
            available.sort_unstable();
            findings.push((
                Code::TableColumnUnknown,
                format!(
                    "named row `{label}` names an unknown column `{key}`; available columns: {}",
                    available.join(", ")
                ),
            ));
        }
    }
    let items = data_columns
        .iter()
        .map(|column| {
            column
                .name()
                .and_then(|name| cells.get(name))
                .map(|value| item_top(value, &label, findings))
        })
        .collect();
    Some(Written::Named { label, items })
}

/// Python `_parse_cell_top`.
fn item_top(value: &Value, label: &str, findings: &mut Vec<Finding>) -> Item {
    match value {
        Value::Sequence(items) => Item::List(
            items
                .iter()
                .map(|item| item_leaf(item, label, findings))
                .collect(),
        ),
        other => item_leaf(other, label, findings),
    }
}

/// Python `_parse_cell_leaf`.
fn item_leaf(value: &Value, label: &str, findings: &mut Vec<Finding>) -> Item {
    match value {
        Value::Null => Item::Null,
        Value::String(_) | Value::Number(_) | Value::Bool(_) => {
            Item::Scalar(text(value).unwrap_or_default())
        }
        Value::Mapping(map) if map.contains_key("value") => {
            unknown_keys(
                map,
                &["value", "rows", "cols", "align"],
                &format!("a cell of row `{label}`"),
                findings,
            );
            let mut cell = RawCell::empty();
            match map.get("value") {
                Some(Value::Null) | None => {}
                Some(value) => match text(value) {
                    Some(text) => cell.text = text,
                    None => findings.push((
                        Code::TableShape,
                        format!(
                            "row `{label}`: a cell `value` must be a scalar, got {}",
                            describe(value)
                        ),
                    )),
                },
            }
            cell.rows = span_count(map.get("rows"), "rows", label, findings);
            cell.cols = span_count(map.get("cols"), "cols", label, findings);
            match map.get("align") {
                None | Some(Value::Null) => {}
                Some(Value::String(s)) => match Align::parse(s) {
                    Some(align) => cell.align = Some(align),
                    None => findings.push((
                        Code::TableAlign,
                        format!(
                            "row `{label}`: unknown `align` value `{s}`; expected l, c, r, j or a long form"
                        ),
                    )),
                },
                Some(other) => findings.push((
                    Code::TableShape,
                    format!("row `{label}`: `align` must be text, got {}", describe(other)),
                )),
            }
            Item::Rich(cell)
        }
        other => {
            findings.push((
                Code::TableShape,
                format!(
                    "row `{label}`: invalid cell value {}; a cell is a scalar, `~`, or {{value, rows, cols, align}}",
                    describe(other)
                ),
            ));
            Item::Null
        }
    }
}

fn span_count(value: Option<&Value>, key: &str, label: &str, findings: &mut Vec<Finding>) -> u32 {
    match value {
        None | Some(Value::Null) => 1,
        Some(Value::Number(n)) => match n.as_u64() {
            Some(n) if n >= 1 => u32::try_from(n).unwrap_or(u32::MAX),
            _ => {
                findings.push((
                    Code::TableShape,
                    format!("row `{label}`: `{key}` must be >= 1, got {n}"),
                ));
                1
            }
        },
        Some(other) => {
            findings.push((
                Code::TableShape,
                format!(
                    "row `{label}`: `{key}` must be an integer, got {}",
                    describe(other)
                ),
            ));
            1
        }
    }
}

// ------------------------------------------------------------ expansion

/// An active row span: `(origin_row, rows, origin_col, cols)`.
type Active = (usize, u32, usize, u32);

/// Python `_build_section`: expand the written rows of a section into the
/// dense leaf matrix, carrying row spans over.
fn expand_section(
    rows: Vec<Written>,
    spans: &[(usize, usize)],
    section: &str,
    first_index: usize,
    findings: &mut Vec<Finding>,
) -> Vec<RawRow> {
    let n = spans.iter().map(|(_, len)| len).sum::<usize>();
    let mut out = Vec::with_capacity(rows.len());
    let mut active: Vec<Active> = Vec::new();
    for (i, row) in rows.into_iter().enumerate() {
        let absolute = first_index + i;
        let (label, filled) = match row {
            Written::Separator { label, double_rule } => {
                if active
                    .iter()
                    .any(|(origin, rows, _, _)| absolute < origin + *rows as usize)
                {
                    findings.push((
                        Code::TableSpan,
                        format!("{section} row {i}: separator falls inside an active row span"),
                    ));
                }
                out.push(RawRow::Separator { label, double_rule });
                continue;
            }
            Written::Positional { label, items } => {
                let mut leaf_row: Vec<Option<RawCell>> = vec![None; n];
                apply_active(&mut leaf_row, &active, absolute, section, findings);
                let mut strict = Vec::new();
                let mut row = leaf_row.clone();
                place_positional(&items, &mut row, spans, &label, section, true, &mut strict);
                if !strict.is_empty() {
                    // Python's reading: absorbed slots after a placed cell need
                    // no `~`. Accept it when it makes the row valid.
                    let mut lenient = Vec::new();
                    let mut second = leaf_row.clone();
                    place_positional(
                        &items,
                        &mut second,
                        spans,
                        &label,
                        section,
                        false,
                        &mut lenient,
                    );
                    if lenient.is_empty() {
                        row = second;
                    } else {
                        findings.append(&mut strict);
                    }
                }
                (label, fill(row, false))
            }
            Written::Named { label, items } => {
                let mut leaf_row: Vec<Option<RawCell>> = vec![None; n];
                apply_active(&mut leaf_row, &active, absolute, section, findings);
                place_named(&label, &items, &mut leaf_row, spans, section, findings);
                (label, fill(leaf_row, true))
            }
        };
        let _ = label;
        let (cells, named) = filled;
        for (col, cell) in cells.iter().enumerate() {
            if !cell.absorbed && cell.rows > 1 {
                active.push((absolute, cell.rows, col, cell.cols));
            }
        }
        active.retain(|(origin, rows, _, _)| absolute + 1 < origin + *rows as usize);
        out.push(RawRow::Data { cells, named });
    }
    if !active.is_empty() {
        let mut cols: Vec<String> = active.iter().map(|(_, _, c, _)| c.to_string()).collect();
        cols.sort_unstable();
        findings.push((
            Code::TableSpan,
            format!(
                "{section}: row spans at leaf columns {} extend past the last row of the section",
                cols.join(", ")
            ),
        ));
    }
    out
}

/// Python `_apply_active_spans`.
fn apply_active(
    row: &mut [Option<RawCell>],
    active: &[Active],
    absolute: usize,
    section: &str,
    findings: &mut Vec<Finding>,
) {
    for (origin, rows, col, cols) in active {
        if absolute >= origin + *rows as usize {
            continue;
        }
        for offset in 0..*cols as usize {
            let pos = col + offset;
            if pos >= row.len() {
                break;
            }
            if row[pos].is_some() {
                findings.push((
                    Code::TableSpan,
                    format!("{section}: overlapping row spans at leaf column {pos}"),
                ));
            } else {
                row[pos] = Some(RawCell::absorbed());
            }
        }
    }
}

/// Python `_expand_data_row` for a positional row: the items address the
/// top-level columns in order; `strict` demands a `~` for every absorbed
/// slot, its negation is Python's `skip_absorbed` reading.
fn place_positional(
    items: &[Item],
    row: &mut [Option<RawCell>],
    spans: &[(usize, usize)],
    label: &str,
    section: &str,
    strict: bool,
    findings: &mut Vec<Finding>,
) {
    let n = row.len();
    let mut cursor = 0;
    let mut col = 0;
    let advance = |cursor: usize, col: &mut usize| {
        while *col < spans.len() && cursor >= spans[*col].0 + spans[*col].1 {
            *col += 1;
        }
    };
    for item in items {
        if cursor < n && row[cursor].is_some() {
            // An absorbed slot carried over from a row span above.
            if !item.is_null() {
                findings.push((
                    Code::TableSpan,
                    format!(
                        "{section} row `{label}`: a value is written at leaf column {cursor}, which a row span above absorbs"
                    ),
                ));
            }
            cursor += 1;
            advance(cursor, &mut col);
            continue;
        }
        if col >= spans.len() {
            findings.push((
                Code::TableRowWidth,
                format!(
                    "{section} row `{label}` has extra cells beyond the {} declared columns",
                    spans.len()
                ),
            ));
            break;
        }
        let end = spans[col].0 + spans[col].1;
        let remaining = end - cursor;
        match item {
            Item::List(list) => {
                place_list(list, row, cursor, remaining, label, section, findings);
                cursor = end;
                col += 1;
            }
            Item::Rich(cell) => {
                cursor = place_rich(cell, row, cursor, n, label, section, findings);
                advance(cursor, &mut col);
            }
            Item::Null | Item::Scalar(_) => {
                let text = match item {
                    Item::Scalar(text) => text.clone(),
                    _ => String::new(),
                };
                for pos in cursor..end {
                    set(
                        row,
                        pos,
                        RawCell::scalar(text.clone()),
                        label,
                        section,
                        findings,
                    );
                }
                cursor = end;
                col += 1;
            }
        }
        if !strict {
            while cursor < n && row[cursor].is_some() {
                cursor += 1;
            }
            advance(cursor, &mut col);
        }
    }
    if cursor < n {
        findings.push((
            Code::TableRowWidth,
            format!("{section} row `{label}` covers {cursor} leaf cells; expected {n}"),
        ));
    }
}

/// Python `_place_list`: a list fills the remaining leaves of the current
/// column, a rich cell inside it spanning within them; a `~` under a row
/// span acknowledges the absorbed slot.
fn place_list(
    items: &[Item],
    row: &mut [Option<RawCell>],
    cursor: usize,
    remaining: usize,
    label: &str,
    section: &str,
    findings: &mut Vec<Finding>,
) {
    if items.len() > remaining {
        findings.push((
            Code::TableRowWidth,
            format!(
                "{section} row `{label}`: a list of {} cells for a column with only {remaining} sub-columns",
                items.len()
            ),
        ));
    }
    let mut offset = 0;
    let mut index = 0;
    while offset < remaining {
        let pos = cursor + offset;
        let item = items.get(index);
        index += 1;
        if row[pos].is_some() {
            if !matches!(item, None | Some(Item::Null)) {
                findings.push((
                    Code::TableSpan,
                    format!(
                        "{section} row `{label}`: a value is written at leaf column {pos}, which a row span above absorbs"
                    ),
                ));
            }
            offset += 1;
            continue;
        }
        match item {
            Some(Item::Rich(cell)) => {
                let mut cell = cell.clone();
                if offset + cell.cols as usize > remaining {
                    findings.push((
                        Code::TableSpan,
                        format!(
                            "{section} row `{label}`: a cell in a list spans {} leaf columns, exceeding the group's remaining {}",
                            cell.cols,
                            remaining - offset
                        ),
                    ));
                    cell.cols = (remaining - offset) as u32;
                }
                let cols = cell.cols as usize;
                set(row, pos, cell, label, section, findings);
                for extra in 1..cols {
                    set(
                        row,
                        pos + extra,
                        RawCell::absorbed(),
                        label,
                        section,
                        findings,
                    );
                }
                offset += cols;
            }
            Some(Item::List(_)) => {
                findings.push((
                    Code::TableShape,
                    format!("{section} row `{label}`: a list cannot nest inside a group list"),
                ));
                set(row, pos, RawCell::empty(), label, section, findings);
                offset += 1;
            }
            Some(Item::Scalar(text)) => {
                set(
                    row,
                    pos,
                    RawCell::scalar(text.clone()),
                    label,
                    section,
                    findings,
                );
                offset += 1;
            }
            Some(Item::Null) | None => {
                set(row, pos, RawCell::empty(), label, section, findings);
                offset += 1;
            }
        }
    }
}

/// Python `_place_rich`: returns the cursor after the span.
fn place_rich(
    cell: &RawCell,
    row: &mut [Option<RawCell>],
    cursor: usize,
    n: usize,
    label: &str,
    section: &str,
    findings: &mut Vec<Finding>,
) -> usize {
    let mut cell = cell.clone();
    if cursor + cell.cols as usize > n {
        findings.push((
            Code::TableSpan,
            format!(
                "{section} row `{label}`: a cell with cols={} extends past the {n} declared leaf columns",
                cell.cols
            ),
        ));
        cell.cols = (n - cursor) as u32;
    }
    let cols = cell.cols as usize;
    set(row, cursor, cell, label, section, findings);
    for extra in 1..cols {
        set(
            row,
            cursor + extra,
            RawCell::absorbed(),
            label,
            section,
            findings,
        );
    }
    cursor + cols
}

/// A named row: each present column value fills its own column's free
/// leaves; an omitted column is left empty (or absorbed).
fn place_named(
    label: &str,
    items: &[Option<Item>],
    row: &mut [Option<RawCell>],
    spans: &[(usize, usize)],
    section: &str,
    findings: &mut Vec<Finding>,
) {
    let n = row.len();
    if let Some(&(start, len)) = spans.first() {
        for slot in &mut row[start..(start + len).min(n)] {
            if slot.is_none() {
                *slot = Some(RawCell::scalar(label.to_string()));
            }
        }
    }
    for (item, &(start, len)) in items.iter().zip(spans.iter().skip(1)) {
        let Some(item) = item else { continue };
        let mut cursor = start;
        let end = start + len;
        while cursor < end && row[cursor].is_some() {
            cursor += 1;
        }
        if cursor >= end {
            if !item.is_null() {
                findings.push((
                    Code::TableSpan,
                    format!(
                        "{section} row `{label}`: a value is written at leaf column {start}, which a span absorbs"
                    ),
                ));
            }
            continue;
        }
        let remaining = end - cursor;
        match item {
            Item::List(list) => {
                place_list(list, row, cursor, remaining, label, section, findings);
            }
            Item::Rich(cell) => {
                place_rich(cell, row, cursor, n, label, section, findings);
            }
            Item::Null | Item::Scalar(_) => {
                let text = match item {
                    Item::Scalar(text) => text.clone(),
                    _ => String::new(),
                };
                for slot in &mut row[cursor..end.min(n)] {
                    if slot.is_none() {
                        *slot = Some(RawCell::scalar(text.clone()));
                    }
                }
            }
        }
    }
}

/// Python `_set_cell`: a collision is `table-span`; the first writer wins.
fn set(
    row: &mut [Option<RawCell>],
    pos: usize,
    cell: RawCell,
    label: &str,
    section: &str,
    findings: &mut Vec<Finding>,
) {
    if pos >= row.len() {
        return;
    }
    if row[pos].is_some() {
        findings.push((
            Code::TableSpan,
            format!("{section} row `{label}`: cell collides at leaf column {pos}"),
        ));
    } else {
        row[pos] = Some(cell);
    }
}

/// Unfilled slots become empty cells (the row is already reported).
fn fill(row: Vec<Option<RawCell>>, named: bool) -> (Vec<RawCell>, bool) {
    (
        row.into_iter()
            .map(|slot| slot.unwrap_or_else(RawCell::empty))
            .collect(),
        named,
    )
}

// -------------------------------------------------------------- helpers

/// Python `str(scalar)` for YAML scalars; `None` for a collection. Null
/// is the empty text.
fn text(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some(String::new()),
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => None,
    }
}

fn describe(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("the boolean `{b}`"),
        Value::Number(n) => format!("the number `{n}`"),
        Value::String(s) => format!("the text `{s}`"),
        Value::Sequence(_) => "a list".to_string(),
        Value::Mapping(_) => "a mapping".to_string(),
        Value::Tagged(_) => "a tagged value".to_string(),
    }
}

/// Python `extra="forbid"`.
fn unknown_keys(map: &Mapping, allowed: &[&str], what: &str, findings: &mut Vec<Finding>) {
    for key in map.keys() {
        let key = text(key).unwrap_or_else(|| describe(key));
        if !allowed.contains(&key.as_str()) {
            findings.push((
                Code::TableUnknownKey,
                format!(
                    "{what} has an unknown key `{key}`; expected one of {}",
                    allowed
                        .iter()
                        .filter(|k| !k.contains('_'))
                        .map(|k| format!("`{k}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(table: &RawTable) -> Vec<Code> {
        table.findings.iter().map(|(c, _)| *c).collect()
    }

    fn texts(row: &RawRow) -> Vec<String> {
        match row {
            RawRow::Data { cells, .. } => cells
                .iter()
                .map(|c| {
                    if c.absorbed {
                        "~".into()
                    } else {
                        c.text.clone()
                    }
                })
                .collect(),
            RawRow::Separator { .. } => vec!["---".into()],
        }
    }

    #[test]
    fn grouped_lists_fill_leaves() {
        let t = parse_table(
            "columns:\n  - Product\n  - {name: FY23, columns: [Q1, Q2]}\n  - {name: FY24, columns: [Q1, Q2], width-group: q}\nrows:\n  - [Apples, [1, 2], [3, 4]]\n  - [Pears, 5, [~, 6]]\n",
        )
        .unwrap();
        assert!(codes(&t).is_empty(), "{:?}", t.findings);
        assert_eq!(texts(&t.rows[0]), ["Apples", "1", "2", "3", "4"]);
        assert_eq!(texts(&t.rows[1]), ["Pears", "5", "5", "", "6"]);
        assert_eq!(t.columns[2].config().width_group.as_deref(), Some("q"));
    }

    #[test]
    fn named_rows_match_top_level_columns() {
        let t = parse_table(
            "columns:\n  - Product\n  - {name: FY23, columns: [Q1, Q2]}\n  - {name: FY24, columns: [Q1, Q2]}\nrows:\n  - Apples: {FY23: [1, 2], FY24: [3, 4]}\n  - label: Cherries\n    cells: {FY23: [~, 5]}\n  - 2024: {Acutal: [6, 7]}\n",
        )
        .unwrap();
        assert_eq!(codes(&t), [Code::TableColumnUnknown]);
        assert_eq!(texts(&t.rows[0]), ["Apples", "1", "2", "3", "4"]);
        assert_eq!(texts(&t.rows[1]), ["Cherries", "", "5", "", ""]);
        assert_eq!(texts(&t.rows[2]), ["2024", "", "", "", ""]);
        assert!(matches!(t.rows[1], RawRow::Data { named: true, .. }));
    }

    #[test]
    fn spans_absorb_and_need_acknowledgement() {
        let t = parse_table(
            "columns: [A, B, C, D]\nrows:\n  - [r1, {value: Block, rows: 2, cols: 3, align: c}]\n  - [r2, ~, ~, ~]\n  - [r3, x, y, z]\n",
        )
        .unwrap();
        assert!(codes(&t).is_empty(), "{:?}", t.findings);
        assert_eq!(texts(&t.rows[0]), ["r1", "Block", "~", "~"]);
        assert_eq!(texts(&t.rows[1]), ["r2", "~", "~", "~"]);
        let bad = parse_table(
            "columns: [A, B, C, D]\nrows:\n  - [r1, {value: Block, rows: 2, cols: 2}]\n  - [r2, a, b, c]\n",
        )
        .unwrap();
        assert!(codes(&bad).contains(&Code::TableSpan), "{:?}", bad.findings);
    }

    #[test]
    fn python_reading_without_acknowledgement_is_accepted() {
        let t = parse_table(
            "columns: [Article, Status, Editor, Pages]\nrows:\n  - [Alpha, Draft, {value: Maria, rows: 2}, 12]\n  - [Beta, Review, 18]\n",
        )
        .unwrap();
        assert!(codes(&t).is_empty(), "{:?}", t.findings);
        assert_eq!(texts(&t.rows[1]), ["Beta", "Review", "~", "18"]);
    }

    #[test]
    fn ragged_rows_and_unknown_keys() {
        let t = parse_table("columns: [A, B, C, D]\nrows:\n  - [x, 1, 2]\n").unwrap();
        assert_eq!(codes(&t), [Code::TableRowWidth]);
        assert_eq!(texts(&t.rows[0]), ["x", "1", "2", ""]);
        let t = parse_table("columns: [A, B]\nrows: []\ncolour: red\ntable: {wdith: 1}\n").unwrap();
        assert_eq!(codes(&t), [Code::TableUnknownKey, Code::TableUnknownKey]);
    }

    #[test]
    fn settings_and_separators() {
        let t = parse_table(
            "table: {width: x, placement: htbp, long: auto}\ncolumns: [A, B]\nrows:\n  - separator: {label: S, double-rule: true}\n  - {separator: true, label: T}\n",
        )
        .unwrap();
        assert!(codes(&t).is_empty(), "{:?}", t.findings);
        assert_eq!(t.settings.width, "X");
        assert_eq!(t.settings.long, None);
        assert_eq!(
            t.rows[0],
            RawRow::Separator {
                label: Some("S".into()),
                double_rule: true
            }
        );
    }

    #[test]
    fn invalid_payloads_are_errors() {
        assert!(parse_table("- a\n- b\n").is_err());
        assert!(parse_table("").is_err());
        assert!(parse_table("columns: [a\n").is_err());
        let config = parse_table_config("").unwrap();
        assert!(config.columns.is_empty() && config.findings.is_empty());
        let config =
            parse_table_config("columns: [~, {align: left, width: X}]\ntable: {long: true}\n")
                .unwrap();
        assert_eq!(config.columns[1].align, Some(Align::Left));
        assert_eq!(config.settings.long, Some(true));
    }
}
