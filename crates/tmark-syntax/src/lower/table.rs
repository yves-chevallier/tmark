//! Tables: GFM pipe tables and `yaml table` / `yaml table-config` fences
//! lowered to the semantic `TableModel`. Spec §Table. The YAML payloads
//! are read by `table_yaml` (the port of the Python schema); this file
//! parses the cell text as inline Markdown and reports the findings.

use tmark_ir::{
    Align, Cell, Column, ColumnConfig, DataRow, LeafColumn, Row, Separator, Span, TableModel,
    TableSettings,
};
use tmark_markdown::mdast::{AlignKind, Node};

use super::table_yaml::{self, RawRow};
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

    /// A `yaml table` fence. `Err` when the payload is not YAML or not a
    /// mapping (the fence stays a code block); otherwise the model and
    /// whether a `table-*` diagnostic was reported, in which case the
    /// model is a best effort and the caller keeps the source.
    pub fn lower_yaml_table(
        &mut self,
        text: &str,
        span: Span,
    ) -> Result<(TableModel, bool), String> {
        let raw = table_yaml::parse_table(text)?;
        let rejected = !raw.findings.is_empty();
        for (code, message) in raw.findings {
            self.diag(code, span, message);
        }
        let model = TableModel {
            settings: raw.settings,
            columns: raw.columns,
            rows: self.rows(raw.rows, span),
            footer: self.rows(raw.footer, span),
        };
        Ok((model, rejected))
    }

    /// A `yaml table-config` fence; same contract as [`Self::lower_yaml_table`].
    pub fn lower_yaml_table_config(
        &mut self,
        text: &str,
        span: Span,
    ) -> Result<(Vec<ColumnConfig>, TableSettings, bool), String> {
        let raw = table_yaml::parse_table_config(text)?;
        let rejected = !raw.findings.is_empty();
        for (code, message) in raw.findings {
            self.diag(code, span, message);
        }
        Ok((raw.columns, raw.settings, rejected))
    }

    fn rows(&mut self, rows: Vec<RawRow>, span: Span) -> Vec<Row> {
        rows.into_iter()
            .map(|row| match row {
                RawRow::Separator { label, double_rule } => {
                    Row::Separator(Separator { label, double_rule })
                }
                RawRow::Data { cells, named } => Row::Data(DataRow {
                    cells: cells
                        .into_iter()
                        .map(|cell| {
                            if cell.absorbed {
                                Cell::absorbed()
                            } else {
                                Cell {
                                    content: self.lower_fragment(&cell.text, span),
                                    rows: cell.rows,
                                    cols: cell.cols,
                                    align: cell.align,
                                    absorbed: false,
                                }
                            }
                        })
                        .collect(),
                    named,
                }),
            })
            .collect()
    }
}
