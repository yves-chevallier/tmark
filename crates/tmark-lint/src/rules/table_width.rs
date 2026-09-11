//! `table-width`: a table or column width that is empty or a percentage
//! outside (0, 100] (spec §Table; TeXSmith `_validate_width`).

use tmark_ir::{Block, Code, Column, Diagnostic, NodeRef, Span};

use crate::{Context, Rule};

pub struct TableWidth;

impl Rule for TableWidth {
    fn code(&self) -> Code {
        Code::TableWidth
    }

    fn check(&self, ctx: &Context, out: &mut Vec<Diagnostic>) {
        tmark_ir::walk(ctx.doc, &mut |node| {
            let Some((settings, span)) = super::table_settings(node) else {
                return;
            };
            check(&settings.width, "table", span, out);
            match node {
                NodeRef::Block(Block::Table(t)) => {
                    for column in &t.model.columns {
                        columns(column, span, out);
                    }
                }
                NodeRef::Block(Block::TableConfig(c)) => {
                    for (i, column) in c.columns.iter().enumerate() {
                        if let Some(width) = &column.width {
                            check(width, &format!("column {}", i + 1), span, out);
                        }
                    }
                }
                _ => {}
            }
        });
    }
}

fn columns(column: &Column, span: Span, out: &mut Vec<Diagnostic>) {
    let what = match column.name() {
        Some(name) => format!("column `{name}`"),
        None => "an unnamed column".to_string(),
    };
    if let Some(width) = &column.config().width {
        check(width, &what, span, out);
    }
    if let Column::Group(group) = column {
        for child in &group.columns {
            columns(child, span, out);
        }
    }
}

fn check(width: &str, what: &str, span: Span, out: &mut Vec<Diagnostic>) {
    if width.is_empty() {
        out.push(Diagnostic::new(
            Code::TableWidth,
            span,
            format!("{what} has an empty width"),
        ));
        return;
    }
    if let Some(percent) = super::percent(width) {
        if !(percent > 0.0 && percent <= 100.0) {
            out.push(Diagnostic::new(
                Code::TableWidth,
                span,
                format!("{what} width `{width}` must be a percentage in (0, 100]"),
            ));
        }
    }
}
