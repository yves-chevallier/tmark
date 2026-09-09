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
