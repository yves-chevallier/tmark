//! The semantic table model shared by pipe tables, grid tables and
//! `yaml table` fences.
//!
//! Spec §Table. Design: `design/03-ir.md` §Tables: a port of the data model
//! of `texsmith.extensions.tables.schema` (validation stays out; a writer
//! reads a trusted, dense leaf matrix).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::node::Inline;

/// Horizontal alignment of a column or cell. Spec §Table (`l|c|r|j`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum Align {
    #[serde(rename = "l")]
    Left,
    #[serde(rename = "c")]
    Center,
    #[serde(rename = "r")]
    Right,
    #[serde(rename = "j")]
    Justify,
}

/// Knobs of the `table:` section of a `yaml table` or `yaml table-config`
/// payload. Spec §Table rung 5 (`long` and `placement`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TableSettings {
    /// `auto`, `X`, `NN%` or an opaque backend length.
    #[serde(default = "default_width")]
    pub width: String,
    /// Float placement letters (`htbp`), backend-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
    /// `Some(true)` forces a multi-page table, `Some(false)` forbids it,
    /// `None` is `auto`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long: Option<bool>,
}

fn default_width() -> String {
    "auto".into()
}

impl Default for TableSettings {
    fn default() -> Self {
        TableSettings {
            width: default_width(),
            placement: None,
            long: None,
        }
    }
}

/// The optional layout trio every column-like entry carries.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ColumnConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// `X`, `NN%` or an opaque backend length.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    /// Columns sharing a group get the same computed width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_group: Option<String>,
}

/// A terminal column. `name` is `None` when the column has no header label;
/// a table whose columns all lack a name has no header row.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LeafColumn {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(flatten)]
    pub config: ColumnConfig,
}

/// A header group over an ordered list of sub-columns (recursive).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ColumnGroup {
    pub name: String,
    pub columns: Vec<Column>,
    #[serde(flatten)]
    pub config: ColumnConfig,
}

/// Spec §Table rung 5: "Grouped headers (recursive `columns:`)".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum Column {
    Leaf(LeafColumn),
    Group(ColumnGroup),
}

impl Column {
    /// The terminal columns underneath, in order.
    pub fn leaves(&self) -> Vec<&LeafColumn> {
        match self {
            Column::Leaf(leaf) => vec![leaf],
            Column::Group(group) => group.columns.iter().flat_map(Column::leaves).collect(),
        }
    }

    /// Depth of the header hierarchy, 1 for a leaf.
    pub fn depth(&self) -> usize {
        match self {
            Column::Leaf(_) => 1,
            Column::Group(group) => 1 + group.columns.iter().map(Column::depth).max().unwrap_or(0),
        }
    }
}

/// One slot of the leaf matrix.
///
/// A spanning cell carries `rows`/`cols` greater than 1 and the slots it
/// covers are present too, marked `absorbed` (the `~` of the YAML form), so
/// every data row has exactly one cell per leaf column.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Cell {
    /// Inline Markdown; empty for an empty or absorbed cell.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Inline>,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub rows: u32,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub cols: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// Covered by a spanning cell above or to the left.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub absorbed: bool,
}

fn one() -> u32 {
    1
}

fn is_one(n: &u32) -> bool {
    *n == 1
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            content: Vec::new(),
            rows: 1,
            cols: 1,
            align: None,
            absorbed: false,
        }
    }
}

impl Cell {
    pub fn new(content: Vec<Inline>) -> Self {
        Cell {
            content,
            ..Cell::default()
        }
    }

    pub fn absorbed() -> Self {
        Cell {
            absorbed: true,
            ..Cell::default()
        }
    }
}

/// A horizontal rule between rows, optionally labelled.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Separator {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub double_rule: bool,
}

/// A row of data: one cell per leaf column, in column order. In the YAML
/// form the first leaf column holds the row label.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DataRow {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<Cell>,
    /// Written in named-row mode (`{label: {Column: value}}`) rather than
    /// positionally; the printer re-emits the same mode.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub named: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum Row {
    Data(DataRow),
    Separator(Separator),
}

/// Spec §Table: the model every rung of the ladder lowers to. Pipe tables
/// produce one with no spans and no groups.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TableModel {
    #[serde(default)]
    pub settings: TableSettings,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<Column>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<Row>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footer: Vec<Row>,
}

impl TableModel {
    /// Number of leaf columns.
    pub fn leaf_count(&self) -> usize {
        self.columns.iter().map(|c| c.leaves().len()).sum()
    }

    /// Depth of the header hierarchy, 0 for a table without columns.
    pub fn header_depth(&self) -> usize {
        self.columns.iter().map(Column::depth).max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_and_depth() {
        let model = TableModel {
            columns: vec![
                Column::Leaf(LeafColumn {
                    name: Some("Fruit".into()),
                    ..Default::default()
                }),
                Column::Group(ColumnGroup {
                    name: "Warehouses".into(),
                    columns: vec![
                        Column::Leaf(LeafColumn::default()),
                        Column::Leaf(LeafColumn::default()),
                    ],
                    config: ColumnConfig::default(),
                }),
            ],
            ..Default::default()
        };
        assert_eq!(model.leaf_count(), 3);
        assert_eq!(model.header_depth(), 2);
        let json = serde_json::to_value(&model).unwrap();
        assert_eq!(json["columns"][1]["type"], "Group");
        assert_eq!(json["settings"]["width"], "auto");
        let back: TableModel = serde_json::from_value(json).unwrap();
        assert_eq!(back, model);
    }
}
