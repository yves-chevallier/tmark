//! Tables: GFM pipe tables and `yaml table` / `yaml table-config` fences
//! lowered to the semantic `TableModel`. Spec §Table.

use serde::Deserialize;
use tmark_ir::{
    Align, Cell, Column, ColumnConfig, ColumnGroup, DataRow, LeafColumn, Row, Separator, Span,
    TableModel, TableSettings,
};
use tmark_markdown::mdast::{AlignKind, Node};

use super::{plain_text, Ctx, Lowerer};

impl Lowerer {
    /// A GFM pipe table: the header row names the leaf columns.
    pub fn lower_pipe_table(
        &mut self,
        table: &tmark_markdown::mdast::Table,
        ctx: &Ctx,
    ) -> TableModel {
        let mut model = TableModel::default();
        for (row_index, row) in table.children.iter().enumerate() {
            let Node::TableRow(row) = row else { continue };
            let mut cells = Vec::new();
            for (col, cell) in row.children.iter().enumerate() {
                let Node::TableCell(cell) = cell else {
                    continue;
                };
                let content = self.lower_inlines(&cell.children, ctx).inlines;
                if row_index == 0 {
                    let align = table.align.get(col).and_then(|a| match a {
                        AlignKind::Left => Some(Align::Left),
                        AlignKind::Center => Some(Align::Center),
                        AlignKind::Right => Some(Align::Right),
                        AlignKind::None => None,
                    });
                    let name = plain_text(&content);
                    model.columns.push(Column::Leaf(LeafColumn {
                        name: (!name.is_empty()).then_some(name),
                        config: ColumnConfig {
                            align,
                            ..Default::default()
                        },
                    }));
                } else {
                    cells.push(Cell::new(content));
                }
            }
            if row_index > 0 {
                model.rows.push(Row::Data(DataRow {
                    cells,
                    named: false,
                }));
            }
        }
        model
    }

    /// A `yaml table` fence.
    pub fn lower_yaml_table(&mut self, text: &str, span: Span) -> Result<TableModel, String> {
        let table: YamlTable = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
        let mut model = TableModel {
            settings: settings(table.width, table.placement, table.long),
            columns: table.columns.into_iter().map(column).collect(),
            rows: Vec::new(),
            footer: Vec::new(),
        };
        model.rows = self.rows(table.rows, span);
        model.footer = self.rows(table.footer, span);
        Ok(model)
    }

    /// A `yaml table-config` fence.
    pub fn lower_yaml_table_config(
        &mut self,
        text: &str,
    ) -> Result<(Vec<ColumnConfig>, TableSettings), String> {
        let config: YamlTableConfig = serde_yaml_ng::from_str(text).map_err(|e| e.to_string())?;
        Ok((
            config
                .columns
                .into_iter()
                .map(|c| ColumnConfig {
                    align: c.align.and_then(|a| align(&a)),
                    width: c.width,
                    width_group: c.width_group,
                })
                .collect(),
            settings(config.width, config.placement, config.long),
        ))
    }

    fn rows(&mut self, rows: YamlRows, span: Span) -> Vec<Row> {
        let list: Vec<(Option<String>, YamlRow)> = match rows {
            YamlRows::List(rows) => rows.into_iter().map(|r| (None, r)).collect(),
            YamlRows::Named(map) => map
                .into_iter()
                .map(|(label, cells)| (Some(label), YamlRow::Cells(cells)))
                .collect(),
        };
        list.into_iter()
            .map(|(label, row)| match row {
                YamlRow::Separator(sep) => Row::Separator(Separator {
                    label: sep.label,
                    double_rule: sep.double_rule.unwrap_or(false),
                }),
                YamlRow::Cells(cells) => {
                    let mut out = Vec::new();
                    if let Some(label) = &label {
                        let content = self.lower_fragment(label, span);
                        out.push(Cell::new(content));
                    }
                    for cell in cells {
                        out.push(self.cell(cell, span));
                    }
                    Row::Data(DataRow {
                        cells: out,
                        named: label.is_some(),
                    })
                }
            })
            .collect()
    }

    fn cell(&mut self, cell: YamlCell, span: Span) -> Cell {
        match cell {
            YamlCell::Absorbed => Cell::absorbed(),
            YamlCell::Scalar(value) => Cell::new(self.lower_fragment(&scalar(value), span)),
            YamlCell::Rich(rich) => Cell {
                content: self.lower_fragment(&rich.value.map(scalar).unwrap_or_default(), span),
                rows: rich.rows.unwrap_or(1),
                cols: rich.cols.unwrap_or(1),
                align: rich.align.and_then(|a| align(&a)),
                absorbed: false,
            },
        }
    }
}

fn scalar(value: serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::String(s) => s,
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        serde_yaml_ng::Value::Null => String::new(),
        other => serde_yaml_ng::to_string(&other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn settings(
    width: Option<String>,
    placement: Option<String>,
    long: Option<YamlLong>,
) -> TableSettings {
    TableSettings {
        width: width.unwrap_or_else(|| "auto".to_string()),
        placement,
        long: long.map(|l| match l {
            YamlLong::Bool(b) => b,
            YamlLong::Auto(_) => false,
        }),
    }
}

fn align(value: &str) -> Option<Align> {
    match value {
        "l" | "left" => Some(Align::Left),
        "c" | "center" | "centre" => Some(Align::Center),
        "r" | "right" => Some(Align::Right),
        "j" | "justify" => Some(Align::Justify),
        _ => None,
    }
}

fn column(col: YamlColumn) -> Column {
    match col {
        YamlColumn::Name(name) => Column::Leaf(LeafColumn {
            name: Some(name),
            config: ColumnConfig::default(),
        }),
        YamlColumn::Spec(spec) => {
            let config = ColumnConfig {
                align: spec.align.and_then(|a| align(&a)),
                width: spec.width,
                width_group: spec.width_group,
            };
            match spec.columns {
                Some(children) if !children.is_empty() => Column::Group(ColumnGroup {
                    name: spec.name.unwrap_or_default(),
                    columns: children.into_iter().map(column).collect(),
                    config,
                }),
                _ => Column::Leaf(LeafColumn {
                    name: spec.name,
                    config,
                }),
            }
        }
    }
}

// --------------------------------------------------------------------------
// The YAML shapes, deliberately permissive: validation is a later concern.
// --------------------------------------------------------------------------

#[derive(Deserialize)]
struct YamlTable {
    #[serde(default)]
    columns: Vec<YamlColumn>,
    #[serde(default)]
    rows: YamlRows,
    #[serde(default)]
    footer: YamlRows,
    width: Option<String>,
    placement: Option<String>,
    long: Option<YamlLong>,
}

#[derive(Deserialize)]
struct YamlTableConfig {
    #[serde(default)]
    columns: Vec<YamlColumnSpec>,
    width: Option<String>,
    placement: Option<String>,
    long: Option<YamlLong>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum YamlLong {
    Bool(bool),
    Auto(#[allow(dead_code)] String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum YamlColumn {
    Name(String),
    Spec(YamlColumnSpec),
}

#[derive(Deserialize)]
struct YamlColumnSpec {
    name: Option<String>,
    align: Option<String>,
    width: Option<String>,
    width_group: Option<String>,
    columns: Option<Vec<YamlColumn>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum YamlRows {
    List(Vec<YamlRow>),
    Named(indexmap_like::Map),
}

impl Default for YamlRows {
    fn default() -> Self {
        YamlRows::List(Vec::new())
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum YamlRow {
    Separator(YamlSeparator),
    Cells(Vec<YamlCell>),
}

#[derive(Deserialize)]
struct YamlSeparator {
    #[allow(dead_code)]
    separator: bool,
    label: Option<String>,
    double_rule: Option<bool>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum YamlCell {
    Rich(YamlRichCell),
    #[serde(deserialize_with = "absorbed")]
    Absorbed,
    Scalar(serde_yaml_ng::Value),
}

fn absorbed<'de, D: serde::Deserializer<'de>>(d: D) -> Result<(), D::Error> {
    let value = serde_yaml_ng::Value::deserialize(d)?;
    if value.is_null() {
        Ok(())
    } else {
        Err(serde::de::Error::custom("not null"))
    }
}

#[derive(Deserialize)]
pub(crate) struct YamlRichCell {
    value: Option<serde_yaml_ng::Value>,
    rows: Option<u32>,
    cols: Option<u32>,
    align: Option<String>,
}

/// Named rows keep their order: a `Vec` of pairs deserialised from a map.
mod indexmap_like {
    use serde::de::{MapAccess, Visitor};
    use serde::{Deserialize, Deserializer};
    use std::fmt;

    use super::YamlCell;

    pub(crate) struct Map(pub(crate) Vec<(String, Vec<YamlCell>)>);

    impl IntoIterator for Map {
        type Item = (String, Vec<YamlCell>);
        type IntoIter = std::vec::IntoIter<Self::Item>;
        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    impl<'de> Deserialize<'de> for Map {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V;
            impl<'de> Visitor<'de> for V {
                type Value = Map;
                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("a mapping of row labels to cell lists")
                }
                fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Map, A::Error> {
                    let mut out = Vec::new();
                    while let Some((key, value)) = access.next_entry::<String, Vec<YamlCell>>()? {
                        out.push((key, value));
                    }
                    Ok(Map(out))
                }
            }
            d.deserialize_map(V)
        }
    }
}
