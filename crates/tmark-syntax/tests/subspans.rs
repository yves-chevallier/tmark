//! The sub-spans the parser records point at the tokens they name
//! (design 03 §Identity and spans; IR review C1–C3).

use tmark_ir::{walk, Block, FileId, Inline, NodeRef};

const DOC: &str = "\
# Intro {#sec:intro .draft}

$$
x
$$ {#eq:a}

$$ y $$ {#eq:b}

See @sec:intro, @[see fig:x, p. 3; -ein05] and [@ein05; @sec:intro].
A #(fw:one) item and {counter}(fw:two).

::: figure {#fig:x}
![alt](a.png){#fig:inner width=10%}
:::

Table: A caption {#tbl:t}

| a |
| - |
| 1 |
";

#[test]
fn sub_spans_name_their_tokens() {
    let parsed = tmark_syntax::parse(DOC, FileId::default());
    let mut ids = Vec::new();
    let mut keys = Vec::new();
    let mut counters = Vec::new();
    walk(&parsed.document, &mut |node: NodeRef| {
        let attrs = match node {
            NodeRef::Block(Block::Header(h)) => Some(&h.attrs),
            NodeRef::Block(Block::MathBlock(m)) => Some(&m.attrs),
            NodeRef::Block(Block::Figure(f)) => Some(&f.attrs),
            NodeRef::Block(Block::Caption(c)) => Some(&c.attrs),
            NodeRef::Inline(Inline::Image(i)) => Some(&i.attrs),
            _ => None,
        };
        if let Some(attrs) = attrs {
            let span = attrs.id_span.expect("every id here is parsed from text").0;
            ids.push((
                attrs.id().unwrap().to_string(),
                DOC[span.start as usize..span.end as usize].to_string(),
            ));
        }
        match node {
            NodeRef::Inline(Inline::Ref(r)) => {
                for item in &r.items {
                    let span = item.key_span.0;
                    keys.push((
                        item.key.clone(),
                        DOC[span.start as usize..span.end as usize].to_string(),
                    ));
                }
            }
            NodeRef::Inline(Inline::CounterItem(c)) => {
                let span = c.key_span.0;
                counters.push((
                    c.key.clone(),
                    DOC[span.start as usize..span.end as usize].to_string(),
                ));
            }
            _ => {}
        }
    });
    assert_eq!(ids.len(), 6, "{ids:?}");
    for (id, slice) in &ids {
        assert_eq!(id, slice);
    }
    assert_eq!(keys.len(), 5, "{keys:?}");
    for (key, slice) in &keys {
        assert_eq!(key, slice);
    }
    assert_eq!(
        counters,
        [("one".into(), "one".into()), ("two".into(), "two".into())]
    );
}
