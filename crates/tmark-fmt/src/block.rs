//! Block nodes to their canonical spelling (spec §Structure, §Captions and
//! floats, §Containers, §Raw passthrough, §Includes; design 04 "What
//! normalises").

use tmark_ir::{
    Align, Attrs, Block, Cell, Column, ColumnConfig, Document, Inline, ListItem, Row, Table,
    TableConfig, TableModel, TableSettings, Task,
};

use crate::attrs;
use crate::escape::Context;
use crate::inline::{inlines, side_name};
use crate::out::Out;

/// Writes a whole document.
pub fn document(out: &mut Out, doc: &Document) {
    if !doc.front_matter.raw.is_empty() {
        out.push(doc.front_matter.raw.trim_end_matches('\n'));
        out.push("\n");
        if !doc.blocks.is_empty() || !doc.footnotes.is_empty() || !doc.abbreviations.is_empty() {
            out.blank_line();
        }
    }
    blocks(out, &doc.blocks);
    for note in &doc.footnotes {
        separate(out);
        out.push("[^");
        out.push(&note.label);
        out.push("]: ");
        out.push_prefix("    ");
        blocks(out, &note.content);
        out.pop_prefix();
    }
    if !doc.abbreviations.is_empty() {
        separate(out);
        for (i, abbr) in doc.abbreviations.iter().enumerate() {
            if i > 0 {
                out.ensure_newline();
            }
            out.push("*[");
            out.push(&abbr.key);
            out.push("]: ");
            out.push(&abbr.expansion);
        }
    }
    out.ensure_newline();
}

/// A blank line between blocks, none before the first.
fn separate(out: &mut Out) {
    if !out.is_empty() {
        out.blank_line();
    }
}

/// Writes blocks separated by blank lines.
pub fn blocks(out: &mut Out, blocks: &[Block]) {
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            out.blank_line();
        }
        block(out, b);
    }
}

pub fn block(out: &mut Out, b: &Block) {
    match b {
        Block::Para(p) => {
            if let Some(fenced) = generated_image(&p.content) {
                fenced_image(out, fenced);
                return;
            }
            if let [Inline::Aside(aside)] = p.content.as_slice() {
                if aside.content.len() > 1
                    || !matches!(aside.content.first(), Some(Block::Plain(_)))
                {
                    let mut attrs = Attrs::new();
                    if let Some(side) = aside.side {
                        attrs.kv.push(("side".into(), side_name(side).into()));
                    }
                    container(out, "aside", &attrs, &aside.content);
                    return;
                }
            }
            if let Some(lead) = &p.lead {
                out.push("{lead}[");
                inlines(
                    out,
                    lead,
                    Context {
                        in_group: true,
                        ..Context::default()
                    },
                );
                out.push("] ");
                inlines(out, &p.content, Context::default());
            } else {
                inlines(
                    out,
                    &p.content,
                    Context {
                        block_start: true,
                        ..Context::default()
                    },
                );
            }
            out.ensure_newline();
        }
        Block::Plain(p) => {
            inlines(
                out,
                &p.content,
                Context {
                    block_start: true,
                    ..Context::default()
                },
            );
            out.ensure_newline();
        }
        Block::Header(h) => {
            out.push(&"#".repeat(usize::from(h.level).clamp(1, 6)));
            out.push(" ");
            inlines(
                out,
                &h.content,
                Context {
                    block_start: true,
                    ..Context::default()
                },
            );
            attrs::write(out, &h.attrs, " ");
            out.ensure_newline();
        }
        Block::CodeBlock(c) => {
            let mut info = c.lang.clone().unwrap_or_default();
            // A language whose default node word is not `code` (mermaid)
            // needs the word to stay a listing (spec §Data directives).
            if let Some(lang) = &c.lang {
                if tmark_ir::registry::default_node_word(lang).word != "code" {
                    info.push_str(" code");
                }
            }
            for (k, v) in &c.options.kv {
                info.push(' ');
                info.push_str(k);
                info.push_str(&format!("=\"{}\"", v.replace('"', "\\\"")));
            }
            fence(out, info.trim(), &c.text);
        }
        Block::BlockQuote(q) => {
            out.push("> ");
            out.push_prefix("> ");
            blocks(out, &q.content);
            if !q.attrs.is_empty() {
                // `{.epigraph}` after the last paragraph.
                out.ensure_newline();
                out.push("{");
                out.push(&attrs::items(&q.attrs));
                out.push("}");
            }
            out.pop_prefix();
            out.ensure_newline();
        }
        Block::BulletList(l) => list(out, &l.items, None),
        Block::OrderedList(l) => list(out, &l.items, Some(l.start)),
        Block::DefinitionList(d) => {
            for (i, (term, definitions)) in d.items.iter().enumerate() {
                if i > 0 {
                    out.blank_line();
                }
                inlines(
                    out,
                    term,
                    Context {
                        block_start: true,
                        ..Context::default()
                    },
                );
                out.ensure_newline();
                for definition in definitions {
                    out.push(":   ");
                    out.push_prefix("    ");
                    blocks(out, definition);
                    out.pop_prefix();
                    out.ensure_newline();
                }
            }
        }
        Block::HorizontalRule(_) => out.push("---\n"),
        Block::Table(t) => table(out, t),
        Block::TableConfig(c) => table_config(out, c),
        Block::Caption(c) => {
            out.push(match c.kind {
                tmark_ir::CaptionKind::Table => "Table: ",
                tmark_ir::CaptionKind::Figure => "Figure: ",
                tmark_ir::CaptionKind::Listing => "Listing: ",
            });
            inlines(out, &c.content, Context::default());
            attrs::write(out, &c.attrs, " ");
            out.ensure_newline();
        }
        Block::Figure(f) => container(out, "figure", &f.attrs, &f.content),
        Block::Admonition(a) => {
            let mut attrs = Attrs::new();
            if let Some(title) = &a.title {
                let mut buf = Out::new();
                inlines(
                    &mut buf,
                    title,
                    Context {
                        in_group: true,
                        ..Context::default()
                    },
                );
                attrs
                    .kv
                    .push(("title".into(), buf.finish().trim_end().to_string()));
            }
            attrs.id = a.attrs.id.clone();
            attrs.classes = a.attrs.classes.clone();
            attrs.kv.extend(a.attrs.kv.iter().cloned());
            container(out, &a.kind, &attrs, &a.content);
        }
        Block::Div(d) => container(out, &d.name, &d.attrs, &d.content),
        Block::MathBlock(m) => {
            out.push("$$\n");
            out.push(&m.text);
            out.push("\n$$");
            attrs::write(out, &m.attrs, " ");
            out.ensure_newline();
        }
        Block::RawBlock(r) => fence(out, &format!("{} raw", r.format), &r.text),
        Block::Include(i) => {
            out.push("{include");
            if let Some(base) = &i.base {
                out.push(" base=");
                out.push(&attrs::value(base));
            }
            out.push("}(");
            out.push(&i.path);
            out.push(")\n");
        }
        Block::Comment(c) => {
            out.push("<!--");
            out.push(&c.text);
            out.push("-->\n");
        }
    }
}

/// A fenced block: backticks longer than any run inside, minimum three.
fn fence(out: &mut Out, info: &str, text: &str) {
    let longest = text
        .lines()
        .map(|l| l.trim_start().chars().take_while(|c| *c == '`').count())
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    out.push(&fence);
    out.push(info);
    out.push("\n");
    if !text.is_empty() {
        out.push(text);
        out.push("\n");
    }
    out.push(&fence);
    out.push("\n");
}

/// A `::: name {attrs}` container; the fence is longer than any container
/// fence inside.
fn container(out: &mut Out, name: &str, attrs: &Attrs, content: &[Block]) {
    let fence = ":".repeat(3 + container_depth(content));
    out.push(&fence);
    out.push(" ");
    out.push(name);
    attrs::write(out, attrs, " ");
    out.push("\n");
    blocks(out, content);
    out.ensure_newline();
    out.push(&fence);
    out.push("\n");
}

/// How many container levels `blocks` hold.
fn container_depth(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .map(|b| match b {
            Block::Figure(f) => 1 + container_depth(&f.content),
            Block::Admonition(a) => 1 + container_depth(&a.content),
            Block::Div(d) => 1 + container_depth(&d.content),
            Block::BlockQuote(q) => container_depth(&q.content),
            Block::BulletList(l) => items_depth(&l.items),
            Block::OrderedList(l) => items_depth(&l.items),
            Block::Para(p) => match p.content.as_slice() {
                [Inline::Aside(a)] if a.content.len() > 1 => 1 + container_depth(&a.content),
                _ => 0,
            },
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

fn items_depth(items: &[ListItem]) -> usize {
    items
        .iter()
        .map(|i| container_depth(&i.content))
        .max()
        .unwrap_or(0)
}

fn list(out: &mut Out, items: &[ListItem], start: Option<u32>) {
    let loose = items.iter().any(|i| i.content.len() > 1);
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            if loose {
                out.blank_line();
            } else {
                out.ensure_newline();
            }
        }
        let marker = match start {
            Some(start) => format!("{}. ", start + i as u32),
            None => "- ".to_string(),
        };
        out.push(&marker);
        out.push_prefix(&" ".repeat(marker.len()));
        if let Some(task) = item.task {
            out.push(match task {
                Task::Open => "[ ] ",
                Task::Done => "[x] ",
                Task::Partial => "[.] ",
            });
        }
        blocks(out, &item.content);
        out.pop_prefix();
        out.ensure_newline();
    }
}

/// A `Para` holding one generated image (`python image`, bare `mermaid`).
fn generated_image(content: &[Inline]) -> Option<&tmark_ir::Image> {
    match content {
        [Inline::Image(image)] if image.src.is_empty() && image.attrs.get("generate").is_some() => {
            Some(image)
        }
        _ => None,
    }
}

fn fenced_image(out: &mut Out, image: &tmark_ir::Image) {
    let lang = image.attrs.get("generate").unwrap_or_default();
    let code = image.attrs.get("code").unwrap_or_default();
    let mut info = format!("{lang} image");
    for (k, v) in &image.attrs.kv {
        if k == "generate" || k == "code" {
            continue;
        }
        info.push(' ');
        info.push_str(k);
        info.push_str(&format!("=\"{}\"", v.replace('"', "\\\"")));
    }
    fence(out, &info, code);
}

// -------------------------------------------------------------------- tables

/// A table prints as a pipe table when the model is plain, else as a
/// `yaml table` fence (spec §Table).
fn table(out: &mut Out, t: &Table) {
    if is_plain(&t.model) {
        pipe_table(out, &t.model);
    } else {
        fence(out, "yaml table", &yaml_table(&t.model));
    }
    let _ = &t.attrs;
}

fn is_plain(model: &TableModel) -> bool {
    model.settings == TableSettings::default()
        && model.footer.is_empty()
        && model.columns.iter().all(|c| match c {
            Column::Leaf(leaf) => leaf.config.width.is_none() && leaf.config.width_group.is_none(),
            Column::Group(_) => false,
        })
        && model.rows.iter().all(|r| match r {
            Row::Data(d) => {
                !d.named
                    && d.cells
                        .iter()
                        .all(|c| c.rows == 1 && c.cols == 1 && !c.absorbed && c.align.is_none())
            }
            Row::Separator(_) => false,
        })
}

fn cell_text(content: &[Inline], in_cell: bool) -> String {
    let mut buf = Out::new();
    inlines(
        &mut buf,
        content,
        Context {
            in_cell,
            ..Context::default()
        },
    );
    buf.finish().trim_end().to_string()
}

fn pipe_table(out: &mut Out, model: &TableModel) {
    let mut header = Vec::new();
    let mut delimiters = Vec::new();
    for column in &model.columns {
        if let Column::Leaf(leaf) = column {
            header.push(leaf.name.clone().unwrap_or_default());
            delimiters.push(match leaf.config.align {
                Some(Align::Left) => ":--".to_string(),
                Some(Align::Center) => ":-:".to_string(),
                Some(Align::Right) => "--:".to_string(),
                _ => "---".to_string(),
            });
        }
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in &model.rows {
        if let Row::Data(d) = row {
            rows.push(
                d.cells
                    .iter()
                    .map(|c| cell_text(&c.content, true))
                    .collect(),
            );
        }
    }
    let mut widths: Vec<usize> = header.iter().map(|h| h.chars().count().max(3)).collect();
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.chars().count());
            }
        }
    }
    let line = |cells: &[String], out: &mut Out| {
        out.push("|");
        for (i, cell) in cells.iter().enumerate() {
            out.push(" ");
            out.push(cell);
            let width = widths.get(i).copied().unwrap_or(0);
            out.push(&" ".repeat(width.saturating_sub(cell.chars().count())));
            out.push(" |");
        }
        out.push("\n");
    };
    line(&header, out);
    let delimiter_row: Vec<String> = delimiters
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let width = widths[i];
            match d.as_str() {
                ":--" => format!(":{}", "-".repeat(width - 1)),
                ":-:" => format!(":{}:", "-".repeat(width - 2)),
                "--:" => format!("{}:", "-".repeat(width - 1)),
                _ => "-".repeat(width),
            }
        })
        .collect();
    line(&delimiter_row, out);
    for row in &rows {
        line(row, out);
    }
}

// ---------------------------------------------------------------- YAML forms

/// A YAML scalar in flow style: plain when safe, double-quoted otherwise.
fn yaml_scalar(s: &str) -> String {
    let plain = !s.is_empty()
        && !s.starts_with(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '-' | '?'
                        | ':'
                        | ','
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '#'
                        | '&'
                        | '*'
                        | '!'
                        | '|'
                        | '>'
                        | '\''
                        | '"'
                        | '%'
                        | '@'
                        | '`'
                        | '~'
                )
        })
        && !s.ends_with(char::is_whitespace)
        && !s.contains([',', '[', ']', '{', '}', '#', '"', '\n'])
        && !s.contains(": ")
        && !s.ends_with(':')
        && !matches!(
            s,
            "null"
                | "true"
                | "false"
                | "yes"
                | "no"
                | "on"
                | "off"
                | "~"
                | "Null"
                | "True"
                | "False"
        );
    if plain {
        s.to_string()
    } else {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn yaml_config(config: &ColumnConfig) -> Vec<String> {
    let mut parts = Vec::new();
    if let Some(align) = config.align {
        parts.push(format!("align: {}", align_name(align)));
    }
    if let Some(width) = &config.width {
        parts.push(format!("width: {}", yaml_scalar(width)));
    }
    if let Some(group) = &config.width_group {
        parts.push(format!("width_group: {}", yaml_scalar(group)));
    }
    parts
}

fn align_name(align: Align) -> &'static str {
    match align {
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
        Align::Justify => "justify",
    }
}

fn yaml_column(column: &Column) -> String {
    match column {
        Column::Leaf(leaf) => {
            let config = yaml_config(&leaf.config);
            match (&leaf.name, config.is_empty()) {
                (Some(name), true) => yaml_scalar(name),
                (name, _) => {
                    let mut parts = Vec::new();
                    if let Some(name) = name {
                        parts.push(format!("name: {}", yaml_scalar(name)));
                    }
                    parts.extend(config);
                    format!("{{{}}}", parts.join(", "))
                }
            }
        }
        Column::Group(group) => {
            let mut parts = vec![format!("name: {}", yaml_scalar(&group.name))];
            parts.extend(yaml_config(&group.config));
            let columns: Vec<String> = group.columns.iter().map(yaml_column).collect();
            parts.push(format!("columns: [{}]", columns.join(", ")));
            format!("{{{}}}", parts.join(", "))
        }
    }
}

fn yaml_cell(cell: &Cell) -> String {
    if cell.absorbed {
        return "~".to_string();
    }
    let value = yaml_scalar(&cell_text(&cell.content, false));
    if cell.rows == 1 && cell.cols == 1 && cell.align.is_none() {
        return value;
    }
    let mut parts = vec![format!("value: {value}")];
    if cell.rows != 1 {
        parts.push(format!("rows: {}", cell.rows));
    }
    if cell.cols != 1 {
        parts.push(format!("cols: {}", cell.cols));
    }
    if let Some(align) = cell.align {
        parts.push(format!("align: {}", align_name(align)));
    }
    format!("{{{}}}", parts.join(", "))
}

fn yaml_rows(rows: &[Row]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            Row::Separator(s) => {
                let mut parts = vec!["separator: true".to_string()];
                if let Some(label) = &s.label {
                    parts.push(format!("label: {}", yaml_scalar(label)));
                }
                if s.double_rule {
                    parts.push("double_rule: true".to_string());
                }
                format!("  - {{{}}}", parts.join(", "))
            }
            Row::Data(d) => {
                let cells: Vec<String> = d.cells.iter().map(yaml_cell).collect();
                if d.named {
                    let (label, rest) = cells
                        .split_first()
                        .map_or((String::new(), &[][..]), |(l, r)| (l.clone(), r));
                    format!("  {}: [{}]", label, rest.join(", "))
                } else {
                    format!("  - [{}]", cells.join(", "))
                }
            }
        })
        .collect()
}

fn yaml_settings(settings: &TableSettings) -> Vec<String> {
    let mut lines = Vec::new();
    if settings.width != TableSettings::default().width {
        lines.push(format!("width: {}", yaml_scalar(&settings.width)));
    }
    if let Some(placement) = &settings.placement {
        lines.push(format!("placement: {}", yaml_scalar(placement)));
    }
    if let Some(long) = settings.long {
        lines.push(format!("long: {long}"));
    }
    lines
}

fn yaml_table(model: &TableModel) -> String {
    let mut lines = Vec::new();
    lines.push("columns:".to_string());
    for column in &model.columns {
        lines.push(format!("  - {}", yaml_column(column)));
    }
    if !model.rows.is_empty() {
        lines.push("rows:".to_string());
        lines.extend(yaml_rows(&model.rows));
    }
    if !model.footer.is_empty() {
        lines.push("footer:".to_string());
        lines.extend(yaml_rows(&model.footer));
    }
    lines.extend(yaml_settings(&model.settings));
    lines.join("\n")
}

fn table_config(out: &mut Out, config: &TableConfig) {
    let mut lines = vec!["columns:".to_string()];
    for column in &config.columns {
        lines.push(format!("  - {{{}}}", yaml_config(column).join(", ")));
    }
    lines.extend(yaml_settings(&config.settings));
    fence(out, "yaml table-config", &lines.join("\n"));
}
