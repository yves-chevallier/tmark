//! Local edits splice one reprinted node into its span and touch nothing
//! else (design 04 §Local edits).
use tmark_fmt::{edit, NodeEdit, Replacement};
use tmark_ir::{walk, FileId, Inline, NodeRef, Str};
use tmark_syntax::parse;

#[test]
fn renaming_a_reference_key_touches_only_its_span() {
    let text = "Intro.\n\nSee @sec:old  and   keep   this spacing.\n";
    let doc = parse(text, FileId::default()).document;
    // Find the Ref node.
    let mut target = None;
    walk(&doc, &mut |node: NodeRef| {
        if let NodeRef::Inline(Inline::Ref(r)) = node {
            target = Some((r.meta.id, r.clone()));
        }
    });
    let (id, mut reference) = target.expect("a reference");
    reference.items[0].key = "sec:new".to_string();
    let edited = edit(
        text,
        &doc,
        NodeEdit {
            id,
            replacement: Replacement::Inline(Inline::Ref(reference)),
        },
    );
    assert_eq!(
        edited,
        "Intro.\n\nSee @sec:new  and   keep   this spacing.\n"
    );
}

#[test]
fn an_unknown_id_leaves_the_text_alone() {
    let text = "Nothing here.\n";
    let doc = parse(text, FileId::default()).document;
    let edited = edit(
        text,
        &doc,
        NodeEdit {
            id: tmark_ir::NodeId(9999),
            replacement: Replacement::Inline(Inline::Str(Str {
                meta: Default::default(),
                text: "x".into(),
            })),
        },
    );
    assert_eq!(edited, text);
}

mod common;

use common::fixtures;
use proptest::prelude::*;
use tmark_fmt::{edit_many, print_node, EditError};
use tmark_ir::{Block, Document, NodeId};

fn parse_doc(text: &str) -> Document {
    parse(text, FileId::default()).document
}

/// Every node of the document with its span, in pre-order.
fn spans(doc: &Document) -> Vec<(NodeId, u32, u32)> {
    let mut out = Vec::new();
    walk(doc, &mut |n: NodeRef| {
        let s = n.span();
        out.push((n.id(), s.start, s.end));
    });
    out
}

#[test]
fn overlapping_edits_are_refused_and_nothing_is_applied() {
    let text = "Para with *emphasis* here.\n";
    let doc = parse_doc(text);
    let all = spans(&doc);
    let para = doc.blocks[0].meta().id;
    let emph = all
        .iter()
        .find(|(id, s, e)| {
            *id != para && *s < *e && text[*s as usize..*e as usize] == *"*emphasis*"
        })
        .map(|(id, _, _)| *id)
        .expect("the emphasis node");
    let text_edit = |id| NodeEdit {
        id,
        replacement: Replacement::Text("X".into()),
    };
    assert_eq!(
        edit_many(text, &doc, vec![text_edit(para), text_edit(emph)]),
        Err(EditError::Overlap(para, emph))
    );
    assert_eq!(
        edit_many(text, &doc, vec![text_edit(emph), text_edit(emph)]),
        Err(EditError::Overlap(emph, emph))
    );
    assert_eq!(
        edit_many(text, &doc, vec![text_edit(NodeId(9999))]),
        Err(EditError::NotFound(NodeId(9999)))
    );
    assert_eq!(
        edit_many(text, &doc, vec![text_edit(emph)]).unwrap(),
        "Para with X here.\n"
    );
}

#[test]
fn batch_edits_apply_from_the_end() {
    let text = "One @aa and @bb and @cc.\n";
    let doc = parse_doc(text);
    let refs: Vec<NodeId> = spans(&doc)
        .into_iter()
        .filter(|(_, s, e)| text[*s as usize..*e as usize].starts_with('@'))
        .map(|(id, _, _)| id)
        .collect();
    assert_eq!(refs.len(), 3);
    let edits = refs
        .iter()
        .enumerate()
        .map(|(i, id)| NodeEdit {
            id: *id,
            replacement: Replacement::Text(format!("@{}", "long".repeat(i + 1))),
        })
        .collect();
    assert_eq!(
        edit_many(text, &doc, edits).unwrap(),
        "One @long and @longlong and @longlonglong.\n"
    );
}

/// The top-level blocks of every fixture, spliced with their own printed
/// form, give the text back: the printed form of a block in a canonical
/// text is its source, and a batch of disjoint splices touches nothing
/// else. (Two lists in a row alternate their marker when printed together,
/// so a lone list prints with the first marker: those blocks are skipped.)
#[test]
fn splicing_top_level_blocks_with_their_own_print_is_the_identity() {
    let mut failures = Vec::new();
    for (name, text) in fixtures() {
        let doc = parse_doc(&text);
        let edits: Vec<NodeEdit> = doc
            .blocks
            .iter()
            .filter(|b| !matches!(b, Block::BulletList(_) | Block::OrderedList(_)))
            .map(|b| NodeEdit {
                id: b.meta().id,
                replacement: Replacement::Block(b.clone()),
            })
            .collect();
        match edit_many(&text, &doc, edits) {
            Ok(edited) if edited == text => {}
            Ok(edited) => failures.push(format!(
                "{name}:\n{}",
                common::first_difference(&text, &edited)
            )),
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// The canonical texts the property tests draw from.
fn corpus() -> Vec<String> {
    fixtures().into_iter().map(|(_, text)| text).collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Any subset of disjoint nodes, each spliced with its own source
    /// slice, is the identity (the splicing is exact whatever the order
    /// and the count of edits).
    #[test]
    fn splicing_source_slices_is_the_identity(
        index in 0usize..64,
        mask in proptest::collection::vec(any::<bool>(), 0..128),
    ) {
        let corpus = corpus();
        let text = &corpus[index % corpus.len()];
        let doc = parse_doc(text);
        let all = spans(&doc);
        // Take the marked nodes, greedily skipping any that overlaps a
        // node already taken (children of taken nodes, in pre-order).
        let mut taken: Vec<(u32, u32)> = Vec::new();
        let mut edits = Vec::new();
        for (i, (id, start, end)) in all.iter().enumerate() {
            if !mask.get(i).copied().unwrap_or(false) {
                continue;
            }
            if taken.iter().any(|(s, e)| start < e && s < end) {
                continue;
            }
            taken.push((*start, *end));
            edits.push(NodeEdit {
                id: *id,
                replacement: Replacement::Text(text[*start as usize..*end as usize].to_string()),
            });
        }
        prop_assert_eq!(edit_many(text, &doc, edits).unwrap(), text.clone());
    }

    /// Any subset of top-level blocks, spliced with its own printed form,
    /// is the identity.
    #[test]
    fn splicing_printed_blocks_is_the_identity(
        index in 0usize..64,
        mask in proptest::collection::vec(any::<bool>(), 0..32),
    ) {
        let corpus = corpus();
        let text = &corpus[index % corpus.len()];
        let doc = parse_doc(text);
        let edits: Vec<NodeEdit> = doc
            .blocks
            .iter()
            .enumerate()
            .filter(|(i, b)| {
                mask.get(*i).copied().unwrap_or(false)
                    && !matches!(b, Block::BulletList(_) | Block::OrderedList(_))
            })
            .map(|(_, b)| NodeEdit {
                id: b.meta().id,
                replacement: Replacement::Text(print_node(NodeRef::Block(b))),
            })
            .collect();
        prop_assert_eq!(edit_many(text, &doc, edits).unwrap(), text.clone());
    }
}
