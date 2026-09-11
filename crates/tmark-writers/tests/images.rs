//! Icons and generated diagrams (writers-and-passes.md §2 "Figures";
//! fragment-contracts.md §1 row `icon`): an `Image{.icon}` is inline in
//! every backend, never a figure; a converted diagram (`generate=<lang>`)
//! is wrapped in `\adjustbox` for LaTeX.

use tmark_registry::{resolve, MemoryLoader, ResolveOptions};
use tmark_writers::{write, Backend, Media, WriterOptions};

fn body(md: &str, backend: Backend) -> tmark_writers::Body {
    let parsed = tmark_syntax::parse(md, tmark_ir::FileId::default());
    let resolved = resolve(
        &parsed.document,
        &MemoryLoader::new(),
        &ResolveOptions::default(),
    );
    let opts = WriterOptions {
        media: match backend {
            Backend::Html => Media::Web,
            _ => Media::Print,
        },
        ..WriterOptions::default()
    };
    write(&parsed.document, &resolved, backend, &opts)
}

#[test]
fn icon_is_inline() {
    let md = "Smile ![](assets/1f600.svg){.icon} now.\n\n![](assets/1f600.svg){.icon}\n";
    let latex = body(md, Backend::Latex);
    assert_eq!(
        latex.text,
        "Smile \\tsicon{assets/1f600.svg} now.\n\n\\tsicon{assets/1f600.svg}\n"
    );
    assert!(latex.requires.fragments.contains("ts-typesetting"));
    assert!(!latex.requires.packages.contains("float"));
    assert_eq!(latex.requires.assets.len(), 2);
    let typst = body(md, Backend::Typst);
    assert_eq!(
        typst.text,
        "Smile #box(image(\"assets/1f600.svg\"), height: 1em) now.\n\n#box(image(\"assets/1f600.svg\"), height: 1em)\n"
    );
    let html = body(md, Backend::Html);
    assert_eq!(
        html.text,
        "<p>Smile <img src=\"assets/1f600.svg\" alt=\"\" class=\"icon\" /> now.</p>\n<p><img src=\"assets/1f600.svg\" alt=\"\" class=\"icon\" /></p>\n"
    );
}

#[test]
fn generated_diagram_is_adjusted() {
    let md =
        "![Flow](assets/flow.pdf){generate=mermaid width=80%}\n\nFigure: The flow. {#fig:flow}\n";
    let latex = body(md, Backend::Latex);
    assert!(latex.text.contains(
        "\\adjustbox{max width=\\textwidth}{\\includegraphics[width=0.8\\linewidth]{assets/flow.pdf}}"
    ));
    assert!(latex
        .text
        .contains("\\caption[Flow]{The flow.}\\label{fig:flow}"));
    assert!(latex.requires.packages.contains("adjustbox"));
    // Not converted yet: nothing to include.
    let latex = body("![Flow](){generate=mermaid}\n", Backend::Latex);
    assert!(!latex.text.contains("includegraphics"));
    assert!(!latex.requires.packages.contains("adjustbox"));
}
