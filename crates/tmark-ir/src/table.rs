//! The semantic table model shared by pipe tables, grid tables and
//! `yaml table` fences.
//!
//! Spec §Table. Design: `design/03-ir.md` §Tables: a port of the data model
//! of `texsmith.extensions.tables.schema` (the validated pydantic tree).
//! Every type names the Python class it mirrors and every field the Python
//! field, so that the two stay one definition: the Python side is
//! regenerated from the JSON schema of these types.
//!
//! The one deliberate difference: Python keeps a row's `label` next to a
//! list of top-level cells and expands them into a dense `LeafMatrix` on
//! validation; here the matrix *is* the model (`DataRow.cells`, one `Cell`
//! per leaf column, label first) and the printer regroups it into the
//! Python row shapes. Validation lives in `tmark-syntax` (shapes the
//! matrix cannot hold) and `tmark-lint` (values it holds but a backend
//! refuses); see `design/05-diagnostics.md`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::node::Inline;

/// Horizontal alignment of a column or cell. Spec §Table (`l|c|r|j`).
/// Mirrors Python `Align = Literal["l", "c", "r", "j"]`; the long forms
/// (`left`, `center`, `centre`, `right`, `justify`, `justified`) are
/// accepted by the parser and normalised here (`ALIGN_ALIASES`).
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

impl Align {
    /// The alias table of `texsmith.extensions.tables.constants.ALIGN_ALIASES`,
    /// case-insensitive after trimming.
    pub fn parse(value: &str) -> Option<Align> {
        match value.trim().to_ascii_lowercase().as_str() {
            "l" | "left" => Some(Align::Left),
            "c" | "center" | "centre" => Some(Align::Center),
            "r" | "right" => Some(Align::Right),
            "j" | "justify" | "justified" => Some(Align::Justify),
            _ => None,
        }
    }

    /// The long spelling the printer writes.
    pub fn name(self) -> &'static str {
        match self {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
            Align::Justify => "justify",
        }
    }
}

/// Knobs of the `table:` section of a `yaml table` or `yaml table-config`
/// payload. Spec §Table rung 5 (`long` and `placement`). Mirrors Python
/// `TableSettings` (`extra="forbid"`: an unknown key is `table-unknown-key`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TableSettings {
    /// Python `TableSettings.width` (`"auto"` by default): `auto`, `X`,
    /// `NN%` or an opaque backend length. `x` is normalised to `X`
    /// (`_validate_width`); the percentage range is `table-width`.
    #[serde(default = "default_width")]
    pub width: String,
    /// Python `TableSettings.placement`: float placement letters (`htbp`),
    /// backend-specific; `_PLACEMENT_RE` is the lint `table-placement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
    /// Python `TableSettings.long: bool | Literal["auto"]`: `Some(true)`
    /// forces a multi-page table, `Some(false)` forbids it, `None` is
    /// `auto` (the default; written `long: auto` reads back as `None`).
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

/// The optional layout trio every column-like entry carries. Mirrors
/// Python `_ColumnAttrs` (and `ColumnConfig`, the entries of a
/// `yaml table-config` fence, which adds nothing to it).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ColumnConfig {
    /// Python `_ColumnAttrs.align`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// Python `_ColumnAttrs.width`: `X`, `NN%` or an opaque backend length
    /// (`auto` on a column is also a flexible column).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    /// Python `_ColumnAttrs.width_group` (YAML key `width-group`, the
    /// underscore spelling is accepted too): columns sharing a group get
    /// the same computed width. Any scalar, coerced to text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_group: Option<String>,
}

/// A terminal column. Mirrors Python `LeafColumn`: `name` is `None` when
/// the column has no header label; a table whose columns all lack a name
/// has no header row. A bare scalar column descriptor (`Fruit`, `2024`) is
/// a leaf named by its text.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LeafColumn {
    /// Python `LeafColumn.name`: the header as plain text. It is the key
    /// of named-row mode and what a diagnostic names the column by, so it
    /// stays a string even when the header carries markup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The header cell parsed as inline Markdown (spec §Table: "Inline
    /// Markdown survives inside cells in all forms"). Empty when the
    /// header is plain text, which is the common case: writers render
    /// `title` when it is set and escape `name` otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    #[serde(flatten)]
    pub config: ColumnConfig,
}

/// A header group over an ordered list of sub-columns (recursive).
/// Mirrors Python `ColumnGroup` (`name` required, `columns` non-empty).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ColumnGroup {
    /// Python `ColumnGroup.name`: the header as plain text, like
    /// [`LeafColumn::name`].
    pub name: String,
    /// The group header parsed as inline Markdown, like
    /// [`LeafColumn::title`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    /// Python `ColumnGroup.columns`.
    pub columns: Vec<Column>,
    #[serde(flatten)]
    pub config: ColumnConfig,
}

/// Spec §Table rung 5: "Grouped headers (recursive `columns:`)". Mirrors
/// Python `Column = LeafColumn | ColumnGroup`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum Column {
    Leaf(LeafColumn),
    Group(ColumnGroup),
}

impl Column {
    /// The terminal columns underneath, in order (Python `column_leaves`).
    pub fn leaves(&self) -> Vec<&LeafColumn> {
        match self {
            Column::Leaf(leaf) => vec![leaf],
            Column::Group(group) => group.columns.iter().flat_map(Column::leaves).collect(),
        }
    }

    /// Number of terminal columns underneath (Python `leaf_count`).
    pub fn leaf_count(&self) -> usize {
        match self {
            Column::Leaf(_) => 1,
            Column::Group(group) => group.columns.iter().map(Column::leaf_count).sum(),
        }
    }

    /// Depth of the header hierarchy, 1 for a leaf (Python `header_depth`).
    pub fn depth(&self) -> usize {
        match self {
            Column::Leaf(_) => 1,
            Column::Group(group) => 1 + group.columns.iter().map(Column::depth).max().unwrap_or(0),
        }
    }

    /// The layout trio of the column itself.
    pub fn config(&self) -> &ColumnConfig {
        match self {
            Column::Leaf(leaf) => &leaf.config,
            Column::Group(group) => &group.config,
        }
    }

    /// The header text, `None` for an unnamed leaf.
    pub fn name(&self) -> Option<&str> {
        match self {
            Column::Leaf(leaf) => leaf.name.as_deref(),
            Column::Group(group) => Some(&group.name),
        }
    }

    /// The header as inline Markdown, empty when it is plain text (then
    /// [`Column::name`] is the whole header).
    pub fn title(&self) -> &[Inline] {
        match self {
            Column::Leaf(leaf) => &leaf.title,
            Column::Group(group) => &group.title,
        }
    }
}

/// One slot of the leaf matrix. Mirrors Python `LeafCell` (`value`,
/// `absorbed`, `rows`, `cols`, `align`; `origin` is implied by the
/// position). A cell written as `{value, rows, cols, align}` in the YAML
/// form is Python's `RichCell`; a bare scalar is a cell with the defaults.
///
/// A spanning cell carries `rows`/`cols` greater than 1 and the slots it
/// covers are present too, marked `absorbed` (the `~` of the YAML form
/// under a row span; the slots to the right of a column span are absorbed
/// without being written), so every data row has exactly one cell per leaf
/// column.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Cell {
    /// Python `LeafCell.value`, parsed as inline Markdown; empty for an
    /// empty (`~`) or absorbed cell.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Inline>,
    /// Python `RichCell.rows` (>= 1).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub rows: u32,
    /// Python `RichCell.cols` (>= 1).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub cols: u32,
    /// Python `RichCell.align`: a per-cell override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// Python `LeafCell.absorbed`: covered by a spanning cell above or to
    /// the left.
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

    /// A cell with no span, no alignment override and no content: what
    /// `~` means outside a span rectangle.
    pub fn is_empty(&self) -> bool {
        !self.absorbed
            && self.content.is_empty()
            && self.rows == 1
            && self.cols == 1
            && self.align.is_none()
    }
}

/// A horizontal rule between rows, optionally labelled. Mirrors Python
/// `Separator` (written `separator: true` with `label`/`double-rule` next
/// to it, or `separator: {label, double-rule}`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Separator {
    /// Python `Separator.label`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Python `Separator.double_rule` (YAML key `double-rule`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub double_rule: bool,
}

/// A row of data: one cell per leaf column, in column order. Mirrors
/// Python `DataRow` (`label`, `cells`, `source`) after `build_matrix`: the
/// label is the first leaf cell (the first top-level column is the label
/// column), the top-level cells are expanded into leaves.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DataRow {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<Cell>,
    /// Python `DataRow.source == "named"`: written in named-row mode
    /// (`- Label: {Column: value}` or `- {label: …, cells: {…}}`) rather
    /// than positionally; the printer re-emits the same mode.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub named: bool,
}

/// Mirrors Python `Row = Separator | DataRow`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum Row {
    Data(DataRow),
    Separator(Separator),
}

/// Spec §Table: the model every rung of the ladder lowers to. Mirrors
/// Python `Table` (`settings`, `columns`, `rows`, `footer`). Pipe tables
/// produce one with no spans and no groups.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TableModel {
    /// Python `Table.settings`, the `table:` section.
    #[serde(default)]
    pub settings: TableSettings,
    /// Python `Table.columns` (at least two in a `yaml table`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<Column>,
    /// Python `Table.rows`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<Row>,
    /// Python `Table.footer`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footer: Vec<Row>,
}

impl TableModel {
    /// Number of leaf columns (Python `total_leaves`).
    pub fn leaf_count(&self) -> usize {
        self.columns.iter().map(Column::leaf_count).sum()
    }

    /// Depth of the header hierarchy, 0 for a table without columns.
    pub fn header_depth(&self) -> usize {
        self.columns.iter().map(Column::depth).max().unwrap_or(0)
    }

    /// The leaf range `(start, len)` of every top-level column, in order:
    /// what a positional row's items address.
    pub fn top_level_spans(&self) -> Vec<(usize, usize)> {
        let mut start = 0;
        self.columns
            .iter()
            .map(|column| {
                let len = column.leaf_count();
                let span = (start, len);
                start += len;
                span
            })
            .collect()
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
                    title: Vec::new(),
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
        assert_eq!(model.top_level_spans(), vec![(0, 1), (1, 2)]);
        let json = serde_json::to_value(&model).unwrap();
        assert_eq!(json["columns"][1]["type"], "Group");
        assert_eq!(json["settings"]["width"], "auto");
        let back: TableModel = serde_json::from_value(json).unwrap();
        assert_eq!(back, model);
    }

    #[test]
    fn align_aliases() {
        assert_eq!(Align::parse(" Centre "), Some(Align::Center));
        assert_eq!(Align::parse("justified"), Some(Align::Justify));
        assert_eq!(Align::parse("middle"), None);
    }
}
