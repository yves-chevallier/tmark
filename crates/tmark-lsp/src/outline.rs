//! Document symbols and folding ranges from the IR.
//!
//! Design: `08-lsp.md` §Features: symbols are headers, captions,
//! containers and counter items; folds are containers, admonitions,
//! fences, the front matter, and header sections.

use lsp_types::{
    DocumentSymbol, FoldingRange, FoldingRangeKind, Location, SymbolInformation, SymbolKind, Uri,
};
use tmark::ir::{plain_text, walk_blocks, Block, Document, Inline, LineIndex, NodeRef, Span};

use crate::convert::range;

/// A symbol before it is shaped for the protocol.
struct Sym {
    name: String,
    detail: Option<String>,
    kind: SymbolKind,
    /// The whole construct (a header's section, a container's body).
    range: Span,
    /// What the editor highlights when the symbol is selected.
    selection: Span,
    children: Vec<Sym>,
}

/// Hierarchical outline of `doc`.
pub fn document_symbols(doc: &Document, index: &LineIndex) -> Vec<DocumentSymbol> {
    collect(&doc.blocks, body_end(doc))
        .into_iter()
        .map(|s| shape(s, index))
        .collect()
}

/// Flat outline for clients without hierarchical symbol support.
pub fn symbol_information(doc: &Document, index: &LineIndex, uri: &Uri) -> Vec<SymbolInformation> {
    let mut out = Vec::new();
    fn flatten(
        syms: Vec<Sym>,
        container: Option<&str>,
        index: &LineIndex,
        uri: &Uri,
        out: &mut Vec<SymbolInformation>,
    ) {
        for s in syms {
            #[allow(deprecated)]
            out.push(SymbolInformation {
                name: s.name.clone(),
                kind: s.kind,
                tags: None,
                deprecated: None,
                location: Location::new(uri.clone(), range(index, s.selection)),
                container_name: container.map(str::to_string),
            });
            flatten(s.children, Some(&s.name), index, uri, out);
        }
    }
    flatten(
        collect(&doc.blocks, body_end(doc)),
        None,
        index,
        uri,
        &mut out,
    );
    out
}

/// Where the last section ends: the end of the last block, so that trailing
/// blank lines stay out of the outline.
fn body_end(doc: &Document) -> u32 {
    doc.blocks
        .iter()
        .map(|b| b.meta().span.end)
        .max()
        .unwrap_or(0)
}

fn shape(s: Sym, index: &LineIndex) -> DocumentSymbol {
    #[allow(deprecated)]
    DocumentSymbol {
        name: s.name,
        detail: s.detail,
        kind: s.kind,
        tags: None,
        deprecated: None,
        range: range(index, s.range.join(s.selection)),
        selection_range: range(index, s.selection),
        children: (!s.children.is_empty())
            .then(|| s.children.into_iter().map(|c| shape(c, index)).collect()),
    }
}

fn label(text: &str, fallback: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        fallback.to_string()
    } else {
        text.to_string()
    }
}

/// Symbols of a block sequence. Headers nest by level and own everything
/// up to the next header of the same or a higher level; `end` bounds the
/// last section.
fn collect(blocks: &[Block], end: u32) -> Vec<Sym> {
    let mut root: Vec<Sym> = Vec::new();
    // Open headers, outermost first, with their level.
    let mut stack: Vec<(u8, Sym)> = Vec::new();

    fn close_to(level: u8, stack: &mut Vec<(u8, Sym)>, root: &mut Vec<Sym>, end: u32) {
        while stack.last().is_some_and(|(l, _)| *l >= level) {
            let (_, mut sym) = stack.pop().expect("checked");
            sym.range.end = sym.range.end.max(end);
            attach(sym, stack, root);
        }
    }
    fn attach(sym: Sym, stack: &mut [(u8, Sym)], root: &mut Vec<Sym>) {
        match stack.last_mut() {
            Some((_, parent)) => parent.children.push(sym),
            None => root.push(sym),
        }
    }

    for block in blocks {
        let span = block.meta().span;
        match block {
            Block::Header(h) => {
                close_to(h.level, &mut stack, &mut root, span.start);
                let mut sym = symbol(
                    label(&plain_text(&h.content), "(untitled)"),
                    h.attrs.id().map(|id| format!("#{id}")),
                    SymbolKind::STRING,
                    span,
                );
                // Grows as siblings are met; closed by `close_to`.
                sym.range = span;
                stack.push((h.level, sym));
            }
            Block::Caption(c) => {
                let word = c.kind.word();
                let sym = symbol(
                    label(&plain_text(&c.content), word),
                    Some(match c.attrs.id() {
                        Some(id) => format!("{word} #{id}"),
                        None => word.to_string(),
                    }),
                    SymbolKind::OBJECT,
                    span,
                );
                attach(sym, &mut stack, &mut root);
            }
            Block::Figure(f) => {
                let mut sym = symbol(
                    f.attrs
                        .id()
                        .map_or("figure".to_string(), |id| format!("figure #{id}")),
                    None,
                    SymbolKind::MODULE,
                    span,
                );
                sym.children = collect(&f.content, span.end);
                attach(sym, &mut stack, &mut root);
            }
            Block::Admonition(a) => {
                let title = a.title.as_deref().map(plain_text).unwrap_or_default();
                let mut sym = symbol(
                    label(&title, &a.kind),
                    Some(a.kind.clone()),
                    SymbolKind::NAMESPACE,
                    span,
                );
                sym.children = collect(&a.content, span.end);
                attach(sym, &mut stack, &mut root);
            }
            Block::Div(d) => {
                let mut sym = symbol(
                    d.attrs
                        .id()
                        .map_or(d.name.clone(), |id| format!("{} #{id}", d.name)),
                    None,
                    SymbolKind::MODULE,
                    span,
                );
                sym.children = collect(&d.content, span.end);
                attach(sym, &mut stack, &mut root);
            }
            Block::Include(i) => {
                let sym = symbol(
                    i.path.clone(),
                    Some("include".into()),
                    SymbolKind::FILE,
                    span,
                );
                attach(sym, &mut stack, &mut root);
            }
            Block::BlockQuote(q) => {
                for sym in collect(&q.content, span.end) {
                    attach(sym, &mut stack, &mut root);
                }
            }
            Block::BulletList(l) => {
                for item in &l.items {
                    for sym in collect(&item.content, span.end) {
                        attach(sym, &mut stack, &mut root);
                    }
                }
            }
            Block::OrderedList(l) => {
                for item in &l.items {
                    for sym in collect(&item.content, span.end) {
                        attach(sym, &mut stack, &mut root);
                    }
                }
            }
            Block::Para(p) => match para_special(p) {
                Some(ParaKind::Aside(content)) => {
                    let mut sym = symbol("aside".into(), None, SymbolKind::NAMESPACE, span);
                    sym.children = collect(content, span.end);
                    attach(sym, &mut stack, &mut root);
                }
                Some(ParaKind::GeneratedImage(lang)) => {
                    let sym = symbol(
                        format!("{lang} image"),
                        Some("generated".into()),
                        SymbolKind::OBJECT,
                        span,
                    );
                    attach(sym, &mut stack, &mut root);
                }
                None => {
                    for sym in counter_items(block) {
                        attach(sym, &mut stack, &mut root);
                    }
                }
            },
            _ => {
                for sym in counter_items(block) {
                    attach(sym, &mut stack, &mut root);
                }
            }
        }
    }
    close_to(0, &mut stack, &mut root, end);
    root
}

/// Paragraphs that stand for a block: a `::: aside` lowers to
/// `Para([Aside])`, a `python image` fence to `Para([Image])` with a
/// `generate` attribute (design 03 §Implementation notes).
enum ParaKind<'a> {
    Aside(&'a [Block]),
    GeneratedImage(&'a str),
}

fn para_special(p: &tmark::ir::Para) -> Option<ParaKind<'_>> {
    let [only] = p.content.as_slice() else {
        return None;
    };
    match only {
        Inline::Aside(a) => Some(ParaKind::Aside(&a.content)),
        Inline::Image(i) => i.attrs.get("generate").map(ParaKind::GeneratedImage),
        _ => None,
    }
}

fn symbol(name: String, detail: Option<String>, kind: SymbolKind, span: Span) -> Sym {
    Sym {
        name,
        detail,
        kind,
        range: span,
        selection: span,
        children: Vec::new(),
    }
}

/// `#(prefix:key)` items inside one leaf block.
fn counter_items(block: &Block) -> Vec<Sym> {
    let mut out = Vec::new();
    walk_blocks(std::slice::from_ref(block), &mut |node: NodeRef| {
        if let NodeRef::Inline(Inline::CounterItem(item)) = node {
            out.push(symbol(
                format!("{}:{}", item.prefix, item.key),
                Some("counter item".into()),
                SymbolKind::KEY,
                item.meta.span,
            ));
        }
    });
    out
}

/// Folding ranges: multi-line containers, fences, the front matter, and
/// header sections.
pub fn folding_ranges(doc: &Document, index: &LineIndex) -> Vec<FoldingRange> {
    let mut spans: Vec<(Span, Option<FoldingRangeKind>)> = Vec::new();
    if !doc.front_matter.raw.is_empty() {
        spans.push((doc.front_matter.meta.span, Some(FoldingRangeKind::Region)));
    }
    sections(&doc.blocks, body_end(doc), &mut spans);
    walk_blocks(&doc.blocks, &mut |node: NodeRef| {
        let NodeRef::Block(block) = node else { return };
        let kind = match block {
            Block::CodeBlock(_)
            | Block::Figure(_)
            | Block::Admonition(_)
            | Block::Div(_)
            | Block::BlockQuote(_)
            | Block::BulletList(_)
            | Block::OrderedList(_)
            | Block::DefinitionList(_)
            | Block::Table(_)
            | Block::MathBlock(_)
            | Block::RawBlock(_) => Some(FoldingRangeKind::Region),
            Block::Comment(_) => Some(FoldingRangeKind::Comment),
            Block::Para(p) => para_special(p).map(|_| FoldingRangeKind::Region),
            _ => None,
        };
        if let Some(kind) = kind {
            spans.push((block.meta().span, Some(kind)));
        }
    });
    let mut out: Vec<FoldingRange> = spans
        .into_iter()
        .filter_map(|(span, kind)| {
            let start = index.line_col(span.start).line;
            let last = index
                .line_col(span.end.saturating_sub(1).max(span.start))
                .line;
            (last > start).then_some(FoldingRange {
                start_line: start,
                start_character: None,
                end_line: last,
                end_character: None,
                kind,
                collapsed_text: None,
            })
        })
        .collect();
    out.sort_by_key(|f| (f.start_line, std::cmp::Reverse(f.end_line)));
    out.dedup_by_key(|f| (f.start_line, f.end_line));
    out
}

/// A header section runs from the header to the next header of the same or
/// a higher level, within its container.
fn sections(blocks: &[Block], end: u32, out: &mut Vec<(Span, Option<FoldingRangeKind>)>) {
    let mut open: Vec<(u8, u32)> = Vec::new();
    for block in blocks {
        let span = block.meta().span;
        match block {
            Block::Header(h) => {
                while open.last().is_some_and(|(l, _)| *l >= h.level) {
                    let (_, start) = open.pop().expect("checked");
                    out.push((
                        Span::new(span.file, start, span.start),
                        Some(FoldingRangeKind::Region),
                    ));
                }
                open.push((h.level, span.start));
            }
            Block::Figure(f) => sections(&f.content, span.end, out),
            Block::Para(p) => {
                if let Some(ParaKind::Aside(content)) = para_special(p) {
                    sections(content, span.end, out);
                }
            }
            Block::Admonition(a) => sections(&a.content, span.end, out),
            Block::Div(d) => sections(&d.content, span.end, out),
            Block::BlockQuote(q) => sections(&q.content, span.end, out),
            _ => {}
        }
    }
    let file = blocks
        .first()
        .map(|b| b.meta().span.file)
        .unwrap_or_default();
    for (_, start) in open.into_iter().rev() {
        out.push((Span::new(file, start, end), Some(FoldingRangeKind::Region)));
    }
}
