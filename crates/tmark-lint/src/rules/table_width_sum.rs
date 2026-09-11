//! `table-width-sum`: the percentage widths of the columns add up to more
//! than the table (spec §Table). A group's own width counts for its
//! leaves; a column without a percentage counts for nothing.

use tmark_ir::{Block, Code, Column, Diagnostic, NodeRef};

use crate::{Context, Rule};

pub struct TableWidthSum;

impl Rule for TableWidthSum {
    fn code(&self) -> Code {
        Code::TableWidthSum
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        tmark_ir::walk(ctx.doc, &mut |node| {
            let NodeRef::Block(Block::Table(t)) = node else {
                return;
            };
            if t.source.is_some() {
                return;
            }
            let sum: f64 = t.model.columns.iter().map(percent_of).sum();
            if sum > 100.0 + 1e-9 {
                out.push(Diagnostic::new(
                    Code::TableWidthSum,
                    t.meta.span,
                    format!("column widths add up to {sum}%, more than the table"),
                ));
            }
        });
    }
}

fn percent_of(column: &Column) -> f64 {
    if let Some(percent) = column.config().width.as_deref().and_then(super::percent) {
        return percent;
    }
    match column {
        Column::Leaf(_) => 0.0,
        Column::Group(group) => group.columns.iter().map(percent_of).sum(),
    }
}
