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
use crate::mkdocs;
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
    // The MkDocs profile moves a table caption before its table.
    let blocks: Vec<&Block> = if out.mkdocs().is_some() {
        mkdocs::order(blocks)
    } else {
        blocks.iter().collect()
    };
    // Two lists of the same kind in a row would merge on re-parse: the
    // second alternates its marker (`*` / `)`), the third goes back.
    let mut alternate = false;
    for (i, b) in blocks.iter().enumerate() {
        let same_list = i > 0
            && matches!(
                (blocks[i - 1], *b),
                (Block::BulletList(_), Block::BulletList(_))
                    | (Block::OrderedList(_), Block::OrderedList(_))
            );
        alternate = same_list && !alternate;
        if i > 0 {
            out.blank_line();
        }
        block_with(out, b, alternate);
    }
}

/// One block, on its own.
pub fn block(out: &mut Out, b: &Block) {
    block_with(out, b, false);
}

/// `alternate`: a list right after a list of the same kind takes the
/// other marker so that the two do not merge on re-parse.
fn block_with(out: &mut Out, b: &Block, alternate: bool) {
    if out.mkdocs().is_some() && mkdocs_block(out, b) {
        return;
    }
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
            info.push_str(&fence_attrs(&c.options, |_| true));
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
        Block::BulletList(l) => list(out, &l.items, None, alternate),
        Block::OrderedList(l) => list(out, &l.items, Some(l.start), alternate),
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
        Block::RawBlock(r) => raw_block(out, r),
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

/// The MkDocs spelling of a block, when the table has one for it
/// (`mkdocs.rs`); `false` falls through to the canonical spelling.
fn mkdocs_block(out: &mut Out, b: &Block) -> bool {
    match b {
        Block::Admonition(a) => mkdocs::admonition(out, a),
        Block::Caption(c) => mkdocs::caption(out, c),
        Block::RawBlock(r) => mkdocs::raw_block(out, r),
        Block::Include(i) => mkdocs::include(out, i),
        Block::Div(d) if d.name == "tabs" => mkdocs::tabs(out, d),
        Block::Div(d) if d.name == "div" => mkdocs::div_markdown(out, d),
        _ => false,
    }
}

/// A raw block: a foreign directive (`format=markdown`) and an HTML block
/// CommonMark reads back as one print as typed (spec §Foreign directive,
/// §Raw passthrough); any other payload is the `raw` fence.
fn raw_block(out: &mut Out, r: &tmark_ir::RawBlock) {
    if r.format == "markdown" || (r.format == "html" && is_html_block(&r.text)) {
        out.push(r.text.trim_end_matches('\n'));
        out.push("\n");
        return;
    }
    fence(out, &format!("{} raw", r.format), &r.text);
}

/// Whether `text`, printed alone between blank lines, is one CommonMark
/// HTML block: it starts like one of the seven kinds (a tag, a comment,
/// a processing instruction, a declaration, CDATA) and holds no blank line
/// (which would end the block early).
pub fn is_html_block(text: &str) -> bool {
    let text = text.trim_end_matches('\n');
    let first = text.lines().next().unwrap_or("");
    let starts = first.starts_with("<!--")
        || first.starts_with("<?")
        || first.starts_with("<![CDATA[")
        || (first.starts_with("<!") && first[2..].starts_with(|c: char| c.is_ascii_alphabetic()))
        || (first.starts_with('<')
            && first[1..]
                .trim_start_matches('/')
                .starts_with(|c: char| c.is_ascii_alphabetic()));
    starts && !text.lines().any(|l| l.trim().is_empty())
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

fn list(out: &mut Out, items: &[ListItem], start: Option<u32>, alternate: bool) {
    let loose = items.iter().any(|i| i.content.len() > 1);
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            if loose {
                out.blank_line();
            } else {
                out.ensure_newline();
            }
        }
        let marker = match (start, alternate) {
            (Some(start), false) => format!("{}. ", start + i as u32),
            (Some(start), true) => format!("{}) ", start + i as u32),
            (None, false) => "- ".to_string(),
            (None, true) => "* ".to_string(),
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
    info.push_str(&fence_attrs(&image.attrs, |k| {
        k != "generate" && k != "code"
    }));
    fence(out, &info, code);
}

/// The options of a fence, with a leading space when there are any: bare
/// `key=value` words (spec §Lexical grammar, family 4), or the braced
/// attribute list when there are classes or an id, the only spelling that
/// carries them (design 12 C28). `keep` filters the keys.
fn fence_attrs(attrs: &Attrs, keep: impl Fn(&str) -> bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    let braced = attrs.id.is_some() || !attrs.classes.is_empty();
    if let Some(id) = &attrs.id {
        parts.push(format!("#{id}"));
    }
    for class in &attrs.classes {
        parts.push(format!(".{class}"));
    }
    for (k, v) in &attrs.kv {
        if keep(k) {
            parts.push(format!("{k}={}", fence_option(v)));
        }
    }
    if parts.is_empty() {
        String::new()
    } else if braced {
        format!(" {{{}}}", parts.join(" "))
    } else {
        format!(" {}", parts.join(" "))
    }
}

/// A quoted fence option. CommonMark processes backslash escapes in an
/// info string before TMark reads it, so the backslashes of `attrs::quoted`
/// are doubled: `\\"` reaches the option parser as `\"`.
fn fence_option(v: &str) -> String {
    attrs::quoted(v).replace('\\', "\\\\")
}

// -------------------------------------------------------------------- tables

/// A table prints as a pipe table when the model is plain, else as a
/// `yaml table` fence (spec §Table).
fn table(out: &mut Out, t: &Table) {
    if let Some(source) = &t.source {
        // A fence the parser reported on: the model is a best effort, the
        // text as typed is what the author must see back (design 03).
        fence(out, "yaml table", source);
    } else if is_plain(&t.model) {
        pipe_table(out, &t.model);
    } else {
        fence(out, "yaml table", &yaml_table(&t.model));
    }
    let _ = &t.attrs;
}

fn is_plain(model: &TableModel) -> bool {
    let breaks = |content: &[Inline]| {
        content
            .iter()
            .any(|i| matches!(i, Inline::SoftBreak(_) | Inline::LineBreak(_)))
    };
    model.settings == TableSettings::default()
        && model.footer.is_empty()
        && model.columns.iter().all(|c| match c {
            Column::Leaf(leaf) => leaf.config.width.is_none() && leaf.config.width_group.is_none(),
            Column::Group(_) => false,
        })
        && model.rows.iter().all(|r| match r {
            Row::Data(d) => {
                !d.named
                    && d.cells.iter().all(|c| {
                        c.rows == 1
                            && c.cols == 1
                            && !c.absorbed
                            && c.align.is_none()
                            && !breaks(&c.content)
                    })
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
            // Names are plain strings: escape them like cell text.
            let name = leaf.name.clone().unwrap_or_default();
            let mut buf = Out::new();
            crate::escape::text(
                &mut buf,
                &name,
                Context {
                    in_cell: true,
                    ..Context::default()
                },
                None,
            );
            header.push(buf.finish().trim_end().to_string());
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
    if plain && !looks_typed(s) {
        s.to_string()
    } else {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        )
    }
}

/// Scalars YAML would read back as something else than the text: floats,
/// exponents, signed and prefixed integers, the null and boolean words in
/// any case. Plain integers survive (`3` reads back as `3`).
fn looks_typed(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "null"
            | "true"
            | "false"
            | "yes"
            | "no"
            | "on"
            | "off"
            | "y"
            | "n"
            | "~"
            | ".inf"
            | "-.inf"
            | ".nan"
    ) {
        return true;
    }
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if s.starts_with('+') && body.starts_with(|c: char| c.is_ascii_digit()) {
        return true;
    }
    if ["0x", "0o", "0b"].iter().any(|p| body.starts_with(p))
        && body.len() > 2
        && body[2..].chars().all(|c| c.is_ascii_alphanumeric())
    {
        return true;
    }
    let numeric = !body.is_empty()
        && body
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '_'))
        && body.chars().any(|c| c.is_ascii_digit());
    numeric && body.contains(['.', 'e', 'E', '_'])
}

fn yaml_config(config: &ColumnConfig) -> Vec<String> {
    let mut parts = Vec::new();
    if let Some(align) = config.align {
        parts.push(format!("align: {}", align.name()));
    }
    if let Some(width) = &config.width {
        parts.push(format!("width: {}", yaml_scalar(width)));
    }
    if let Some(group) = &config.width_group {
        parts.push(format!("width-group: {}", yaml_scalar(group)));
    }
    parts
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
            let columns: Vec<String> = group.columns.iter().map(yaml_column).collect();
            parts.push(format!("columns: [{}]", columns.join(", ")));
            parts.extend(yaml_config(&group.config));
            format!("{{{}}}", parts.join(", "))
        }
    }
}

/// A cell of the leaf matrix in the Python `Scalar | RichCell` shape; an
/// empty cell is `~`.
fn yaml_cell(cell: &Cell) -> String {
    let text = cell_text(&cell.content, false);
    let value = if text.is_empty() {
        "~".to_string()
    } else {
        yaml_scalar(&text)
    };
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
        parts.push(format!("align: {}", align.name()));
    }
    format!("{{{}}}", parts.join(", "))
}

fn is_rich(cell: &Cell) -> bool {
    cell.rows != 1 || cell.cols != 1 || cell.align.is_some()
}

/// The slots a column span of this row covers: absorbed without being
/// written, unlike the slots a row span from above absorbs (`~`).
fn covered_by_colspan(cells: &[Cell]) -> Vec<bool> {
    let mut covered = vec![false; cells.len()];
    for (i, cell) in cells.iter().enumerate() {
        if !cell.absorbed {
            let end = (i + cell.cols as usize).min(cells.len());
            for slot in &mut covered[(i + 1).min(end)..end] {
                *slot = true;
            }
        }
    }
    covered
}

/// The leaves `[cursor, end)` as the items of a group list: `~` under a
/// row span, nothing under a column span, a cell otherwise. Returns the
/// cursor after the last leaf consumed.
fn yaml_leaves(
    cells: &[Cell],
    covered: &[bool],
    mut cursor: usize,
    end: usize,
) -> (Vec<String>, usize) {
    let mut items = Vec::new();
    while cursor < end {
        let cell = &cells[cursor];
        if cell.absorbed {
            if !covered[cursor] {
                items.push("~".to_string());
            }
            cursor += 1;
        } else {
            items.push(yaml_cell(cell));
            cursor += (cell.cols as usize).max(1);
        }
    }
    (items, cursor)
}

/// A positional row: one item per top-level column (Python
/// `_parse_positional_row` inverted): a leaf column's cell, a group's
/// leaves as a list, a rich cell as itself, `~` for every slot a row span
/// absorbs.
fn yaml_positional(cells: &[Cell], spans: &[(usize, usize)]) -> String {
    let covered = covered_by_colspan(cells);
    let mut items = Vec::new();
    let mut cursor = 0;
    for &(start, len) in spans {
        let end = start + len;
        while cursor < end && cursor < cells.len() {
            let cell = &cells[cursor];
            if cell.absorbed {
                if !covered[cursor] {
                    items.push("~".to_string());
                }
                cursor += 1;
            } else if end - cursor == 1 || is_rich(cell) {
                items.push(yaml_cell(cell));
                cursor += (cell.cols as usize).max(1);
            } else {
                let (list, next) = yaml_leaves(cells, &covered, cursor, end);
                items.push(format!("[{}]", list.join(", ")));
                cursor = next;
            }
        }
    }
    // Leaves beyond the declared columns (a model built by hand).
    while cursor < cells.len() {
        items.push(yaml_cell(&cells[cursor]));
        cursor += 1;
    }
    format!("[{}]", items.join(", "))
}

/// A named row (Python `_parse_named_row` inverted): the label, then the
/// top-level data columns that hold something, by name; an unnamed or
/// duplicated column name, a rich label or a cell spanning out of its
/// column have no named spelling and fall back to the positional form.
fn yaml_named(cells: &[Cell], columns: &[Column], spans: &[(usize, usize)]) -> Option<String> {
    let names: Vec<&str> = columns.iter().skip(1).filter_map(Column::name).collect();
    if names
        .iter()
        .enumerate()
        .any(|(i, n)| names[..i].contains(n))
    {
        return None;
    }
    let covered = covered_by_colspan(cells);
    let label = cells
        .first()
        .filter(|c| !c.absorbed && !is_rich(c))
        .map(|c| cell_text(&c.content, false))?;
    let mut entries = Vec::new();
    for (column, &(start, len)) in columns.iter().zip(spans).skip(1) {
        let end = (start + len).min(cells.len());
        let leaves = &cells[start.min(end)..end];
        if leaves.iter().all(|c| c.absorbed || c.is_empty()) {
            continue;
        }
        let name = column.name()?;
        if leaves
            .iter()
            .enumerate()
            .any(|(i, c)| !c.absorbed && start + i + c.cols as usize > end)
        {
            return None;
        }
        let (items, _) = yaml_leaves(cells, &covered, start, end);
        let value = if len == 1 {
            items.into_iter().next().unwrap_or_else(|| "~".to_string())
        } else {
            format!("[{}]", items.join(", "))
        };
        entries.push(format!("{}: {}", yaml_scalar(name), value));
    }
    let cells = format!("{{{}}}", entries.join(", "));
    Some(if label == "separator" {
        format!("{{label: {}, cells: {cells}}}", yaml_scalar(&label))
    } else {
        format!("{}: {cells}", yaml_scalar(&label))
    })
}

fn yaml_rows(rows: &[Row], columns: &[Column], spans: &[(usize, usize)]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            Row::Separator(s) => {
                let mut parts = vec!["separator: true".to_string()];
                if let Some(label) = &s.label {
                    parts.push(format!("label: {}", yaml_scalar(label)));
                }
                if s.double_rule {
                    parts.push("double-rule: true".to_string());
                }
                format!("  - {{{}}}", parts.join(", "))
            }
            Row::Data(d) => {
                let named = if d.named {
                    yaml_named(&d.cells, columns, spans)
                } else {
                    None
                };
                format!(
                    "  - {}",
                    named.unwrap_or_else(|| yaml_positional(&d.cells, spans))
                )
            }
        })
        .collect()
}

/// The `table:` section, empty when every setting is at its default.
fn yaml_settings(settings: &TableSettings) -> Vec<String> {
    let mut lines = Vec::new();
    if settings.width != TableSettings::default().width {
        lines.push(format!("  width: {}", yaml_scalar(&settings.width)));
    }
    if let Some(placement) = &settings.placement {
        lines.push(format!("  placement: {}", yaml_scalar(placement)));
    }
    if let Some(long) = settings.long {
        lines.push(format!("  long: {long}"));
    }
    if !lines.is_empty() {
        lines.insert(0, "table:".to_string());
    }
    lines
}

fn yaml_table(model: &TableModel) -> String {
    let spans = model.top_level_spans();
    let mut lines = yaml_settings(&model.settings);
    lines.push("columns:".to_string());
    for column in &model.columns {
        lines.push(format!("  - {}", yaml_column(column)));
    }
    if !model.rows.is_empty() {
        lines.push("rows:".to_string());
        lines.extend(yaml_rows(&model.rows, &model.columns, &spans));
    }
    if !model.footer.is_empty() {
        lines.push("footer:".to_string());
        lines.extend(yaml_rows(&model.footer, &model.columns, &spans));
    }
    lines.join("\n")
}

fn table_config(out: &mut Out, config: &TableConfig) {
    if let Some(source) = &config.source {
        fence(out, "yaml table-config", source);
        return;
    }
    let mut lines = yaml_settings(&config.settings);
    if !config.columns.is_empty() {
        lines.push("columns:".to_string());
    }
    for column in &config.columns {
        lines.push(format!("  - {{{}}}", yaml_config(column).join(", ")));
    }
    fence(out, "yaml table-config", &lines.join("\n"));
}
