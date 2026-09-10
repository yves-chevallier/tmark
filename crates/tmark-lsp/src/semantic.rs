//! Semantic tokens: the resolution state the TextMate grammar cannot know.
//!
//! Design: `08-lsp.md` §Features: "node kinds and *resolution state*:
//! unknown role, unresolved reference, deprecated spelling. The TextMate
//! grammar stays for instant colour; semantic tokens overlay what needs a
//! registry." The extension maps every type below to a TextMate scope
//! (`semanticTokenScopes` in `editors/vscode/package.json`).

use lsp_types::{SemanticToken, SemanticTokenType, SemanticTokensLegend};
use tmark::ir::{walk, Code, Diagnostic, Document, Inline, LineCol, LineIndex, NodeRef, Span};
use tmark::{Resolution, Resolved};

/// Token types, in legend order. The index is the wire encoding.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Kind {
    /// A `@key` that resolves to a label, term or external inventory.
    Reference,
    /// A `@key` found in no registry, or in two.
    UnresolvedReference,
    /// A `@key` that resolves to a bibliography entry.
    Citation,
    /// A definition: `#(fw:key)`, `#[term]`.
    Label,
    /// A brace group that looks like a role but names none (design C4).
    UnknownRole,
    /// A deprecated spelling (Appendix "Deprecation schedule").
    Deprecated,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Reference,
        Kind::UnresolvedReference,
        Kind::Citation,
        Kind::Label,
        Kind::UnknownRole,
        Kind::Deprecated,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Reference => "reference",
            Kind::UnresolvedReference => "unresolvedReference",
            Kind::Citation => "citation",
            Kind::Label => "label",
            Kind::UnknownRole => "unknownRole",
            Kind::Deprecated => "deprecated",
        }
    }
}

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: Kind::ALL
            .iter()
            .map(|k| SemanticTokenType::new(k.name()))
            .collect(),
        token_modifiers: Vec::new(),
    }
}

/// Every token of the document, encoded as the protocol wants (deltas,
/// UTF-16 columns, one token per line). `resolved` is the analysis of this
/// very text when the worker has caught up; without it every reference is
/// reported as resolved rather than flickering.
pub fn tokens(
    doc: &Document,
    index: &LineIndex,
    resolved: Option<&Resolved>,
    diagnostics: &[Diagnostic],
) -> Vec<SemanticToken> {
    let mut found: Vec<(Span, Kind)> = Vec::new();
    walk(doc, &mut |node: NodeRef| {
        let NodeRef::Inline(inline) = node else {
            return;
        };
        match inline {
            Inline::Ref(r) => found.push((r.meta.span, reference_kind(r.meta.id, resolved))),
            Inline::CounterItem(n) => found.push((n.meta.span, Kind::Label)),
            Inline::IndexEntry(n) => found.push((n.meta.span, Kind::Label)),
            _ => {}
        }
    });
    for d in diagnostics {
        if d.span.file != doc.file {
            continue;
        }
        match d.code {
            Code::RoleUnknown => found.push((d.span, Kind::UnknownRole)),
            Code::Deprecated => found.push((d.span, Kind::Deprecated)),
            _ => {}
        }
    }
    // Diagnostics win over node kinds when they overlap (a deprecated
    // reference spelling shows as deprecated); then earliest first.
    found.sort_by_key(|(span, kind)| (span.start, kind_priority(*kind), span.end));
    encode(index, &found)
}

fn kind_priority(kind: Kind) -> u8 {
    match kind {
        Kind::Deprecated | Kind::UnknownRole => 0,
        _ => 1,
    }
}

fn reference_kind(node: tmark::ir::NodeId, resolved: Option<&Resolved>) -> Kind {
    let Some(resolved) = resolved else {
        return Kind::Reference;
    };
    let mut items = resolved.refs.iter().filter(|r| r.node == node).peekable();
    if items.peek().is_none() {
        return Kind::Reference;
    }
    let mut all_citations = true;
    for item in items {
        match item.resolution {
            Resolution::Unresolved | Resolution::Ambiguous => return Kind::UnresolvedReference,
            Resolution::Citation { .. } => {}
            _ => all_citations = false,
        }
    }
    if all_citations {
        Kind::Citation
    } else {
        Kind::Reference
    }
}

/// Non-overlapping tokens, split at line ends, as deltas.
fn encode(index: &LineIndex, found: &[(Span, Kind)]) -> Vec<SemanticToken> {
    let mut out = Vec::new();
    let mut last_end = 0u32;
    let mut prev_line = 0u32;
    let mut prev_col = 0u32;
    for &(span, kind) in found {
        if span.start < last_end || span.is_empty() {
            continue;
        }
        last_end = span.end;
        let first = index.line_col(span.start).line;
        let last = index.line_col(span.end.saturating_sub(1)).line;
        for line in first..=last {
            let line_start = index.line_start(line).unwrap_or(index.len());
            let line_end = index.line_start(line + 1).unwrap_or(index.len());
            let start = span.start.max(line_start);
            let end = span.end.min(line_end);
            if end <= start {
                continue;
            }
            let from = index.to_utf16(LineCol {
                line,
                col: start - line_start,
            });
            // Not `line_col(end)`: an end at the line break belongs to
            // this line, not to the start of the next.
            let to = index.to_utf16(LineCol {
                line,
                col: end - line_start,
            });
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 {
                from.col - prev_col
            } else {
                from.col
            };
            out.push(SemanticToken {
                delta_line,
                delta_start,
                length: to.col.saturating_sub(from.col),
                token_type: kind as u32,
                token_modifiers_bitset: 0,
            });
            prev_line = line;
            prev_col = from.col;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmark::FileId;

    #[test]
    fn encodes_deltas_per_line() {
        let text = "ab @x\ncd\n@y é #(fw:k)\n";
        let index = LineIndex::new(text);
        let found = [
            (Span::new(FileId(0), 3, 5), Kind::Reference),
            (Span::new(FileId(0), 9, 11), Kind::UnresolvedReference),
            (Span::new(FileId(0), 15, 22), Kind::Label),
        ];
        let tokens = encode(&index, &found);
        let flat: Vec<(u32, u32, u32, u32)> = tokens
            .iter()
            .map(|t| (t.delta_line, t.delta_start, t.length, t.token_type))
            .collect();
        assert_eq!(flat, [(0, 3, 2, 0), (2, 0, 2, 1), (0, 5, 7, 3)]);
    }

    #[test]
    fn overlaps_and_multiline() {
        let text = "@a\nb\n";
        let index = LineIndex::new(text);
        let found = [
            (Span::new(FileId(0), 0, 4), Kind::Deprecated),
            (Span::new(FileId(0), 0, 2), Kind::Reference),
        ];
        let tokens = encode(&index, &found);
        assert_eq!(tokens.len(), 2, "split at the line end, overlap dropped");
        assert_eq!(tokens[0].length, 3);
        assert_eq!(tokens[1].delta_line, 1);
        assert_eq!(tokens[1].length, 1);
    }
}
