//! Sub-figures (spec §Image, Figure): the images of a `::: figure`
//! container take no number of the `fig` series; their labels resolve to
//! the container's number with a letter, and the container advances the
//! series once. The conformance fixtures cannot express `ResolveOptions`,
//! so the numbering lives here.
use std::path::PathBuf;

use tmark_ir::FileId;
use tmark_registry::{
    resolve, Host, MemoryLoader, Numbering, Resolution, ResolveOptions, Resolved,
};

/// A plain figure, then a container of two labelled images captioned
/// after it, then references to all three.
const DOC: &str = "![Trace](trace.svg){#fig:trace}\n\nFigure: A trace.\n\n\
::: figure {cols=2}\n![Left](l.svg){#fig:left}\n![Right](r.svg){#fig:right}\n:::\n\n\
Figure: Two views. {#fig:views}\n\nSee @fig:trace, @fig:views, @fig:left.\n";

fn run(text: &str, numbering: Numbering) -> Resolved {
    let doc = tmark_syntax::parse(text, FileId::default()).document;
    resolve(
        &doc,
        &MemoryLoader::new(),
        &ResolveOptions {
            path: PathBuf::from("page.md"),
            numbering,
            ..ResolveOptions::default()
        },
    )
}

/// The formatted number of every label, in document order.
fn numbers(resolved: &Resolved) -> Vec<(String, Option<String>)> {
    resolved
        .view()
        .labels
        .into_iter()
        .map(|l| (l.label.id, l.formatted))
        .collect()
}

fn reference(resolved: &Resolved, key: &str) -> Option<String> {
    resolved
        .refs
        .iter()
        .find(|r| r.key == key)
        .and_then(|r| match &r.resolution {
            Resolution::Label { number, .. } => number.clone(),
            _ => None,
        })
}

#[test]
fn a_container_advances_the_series_once_and_letters_its_images() {
    let resolved = run(DOC, Numbering::All);
    assert_eq!(
        numbers(&resolved),
        [
            ("fig:trace".to_string(), Some("1".to_string())),
            ("fig:left".to_string(), Some("2a".to_string())),
            ("fig:right".to_string(), Some("2b".to_string())),
            ("fig:views".to_string(), Some("2".to_string())),
        ]
    );
    // The hosts say why: the images are sub-figures, the container is the
    // figure.
    let hosts: Vec<Host> = resolved.labels.in_order.iter().map(|l| l.host).collect();
    assert_eq!(
        hosts,
        [Host::Figure, Host::Subfigure, Host::Subfigure, Host::Figure]
    );
    // Two figures on the page, not four: the next document starts at 3.
    assert_eq!(resolved.next_start().get("fig"), Some(&3));
    assert_eq!(reference(&resolved, "fig:trace").as_deref(), Some("1"));
    assert_eq!(reference(&resolved, "fig:views").as_deref(), Some("2"));
    assert_eq!(reference(&resolved, "fig:left").as_deref(), Some("2a"));
    assert!(
        resolved.diagnostics.is_empty(),
        "{:?}",
        resolved.diagnostics
    );
}

#[test]
fn the_letters_continue_the_containers_own_number_across_a_chain() {
    let doc = tmark_syntax::parse(DOC, FileId::default()).document;
    let resolved = resolve(
        &doc,
        &MemoryLoader::new(),
        &ResolveOptions {
            path: PathBuf::from("page.md"),
            numbering: Numbering::All,
            start: [("fig".to_string(), 7)].into_iter().collect(),
            ..ResolveOptions::default()
        },
    );
    assert_eq!(reference(&resolved, "fig:views").as_deref(), Some("8"));
    assert_eq!(reference(&resolved, "fig:left").as_deref(), Some("8a"));
    assert_eq!(resolved.next_start().get("fig"), Some(&9));
}

#[test]
fn the_backend_numbers_nothing_but_still_knows_the_sub_figures() {
    let resolved = run(DOC, Numbering::Backend);
    // No number here: LaTeX's `subcaption` and Typst number them.
    assert!(numbers(&resolved).iter().all(|(_, n)| n.is_none()));
    let left = resolved.labels.get("fig:left").unwrap();
    assert_eq!(left.host, Host::Subfigure);
    let subfigure = left.subfigure.as_ref().unwrap();
    assert_eq!(subfigure.parent.as_deref(), Some("fig:views"));
    assert_eq!(subfigure.letter.as_deref(), Some("a"));
    assert!(!resolved.next_start().contains_key("fig"));
}

#[test]
fn a_lone_image_is_the_figure_and_shares_its_number() {
    let text = "::: figure\n![Left](l.svg){#fig:left}\n:::\n\n\
                Figure: One view. {#fig:views}\n\nSee @fig:left.\n";
    let resolved = run(text, Numbering::All);
    assert_eq!(
        numbers(&resolved),
        [
            ("fig:left".to_string(), Some("1".to_string())),
            ("fig:views".to_string(), Some("1".to_string())),
        ]
    );
    assert_eq!(resolved.next_start().get("fig"), Some(&2));
    assert_eq!(reference(&resolved, "fig:left").as_deref(), Some("1"));
}

#[test]
fn an_unlabelled_container_leaves_its_images_unnumbered() {
    let text = "::: figure\n![Left](l.svg){#fig:left}\n![Right](r.svg){#fig:right}\n:::\n\n\
                See @fig:left.\n";
    let resolved = run(text, Numbering::All);
    assert_eq!(
        numbers(&resolved),
        [
            ("fig:left".to_string(), None),
            ("fig:right".to_string(), None),
        ]
    );
    // Nothing was counted, so the chain does not move.
    assert_eq!(resolved.next_start().get("fig"), Some(&1));
}

#[test]
fn a_container_that_is_not_a_grid_of_images_keeps_its_own_labels() {
    // A container holding prose is a plain float; the image in it is no
    // sub-figure and numbers like any other.
    let text = "::: figure\nA caveat about ![Left](l.svg){#fig:left}.\n:::\n\n\
                Figure: Prose. {#fig:views}\n";
    let resolved = run(text, Numbering::All);
    assert_eq!(
        numbers(&resolved),
        [
            ("fig:left".to_string(), Some("1".to_string())),
            ("fig:views".to_string(), Some("2".to_string())),
        ]
    );
}

#[test]
fn a_container_labelled_by_its_own_attribute_letters_the_same_way() {
    let text = "::: figure {#fig:views cols=2}\n![Left](l.svg){#fig:left}\n\
                ![Right](r.svg){#fig:right}\n\nFigure: Two views.\n:::\n\nSee @fig:right.\n";
    let resolved = run(text, Numbering::All);
    assert_eq!(reference(&resolved, "fig:right").as_deref(), Some("1b"));
    assert_eq!(resolved.next_start().get("fig"), Some(&2));
}
