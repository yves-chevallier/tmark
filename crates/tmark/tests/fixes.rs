//! `tmark::fixes` + `tmark::apply_fixes`: what `tmark lint --fix` writes for
//! the deprecated citation sugars (spec §Cite, Appendix "Deprecation
//! schedule"; design 05 §Fixes).

use tmark::{apply_fixes, check, FileId, LintConfig, MemoryLoader, Profile, ResolveOptions};

fn fixed(text: &str) -> String {
    let (_, diagnostics) = check(
        text,
        FileId::default(),
        Profile::default(),
        &MemoryLoader::new(),
        &ResolveOptions::default(),
        &LintConfig::default(),
    );
    apply_fixes(text, FileId::default(), &diagnostics).0
}

/// A citation hugging the word before it (`sortie[^key]`, 21 of the 29
/// citations of a real review) gets a space: `@` never fires after a word
/// character (the X4 guard), so `sortie@key` would be literal text.
#[test]
fn a_hugging_citation_fix_keeps_the_sigil_alive() {
    assert_eq!(
        fixed("En sortie[^spru485a] et [^ein05], puis tutor.^[ein05,AI2027] fin.\n"),
        "En sortie @spru485a et @ein05, puis tutor. @[ein05; AI2027] fin.\n"
    );
    // The fixed text parses to references, not to prose.
    let (doc, _) = check(
        "En sortie @spru485a et @ein05.\n",
        FileId::default(),
        Profile::default(),
        &MemoryLoader::new(),
        &ResolveOptions::default(),
        &LintConfig::default(),
    );
    let tmark::ir::Block::Para(p) = &doc.blocks[0] else {
        panic!("{:?}", doc.blocks)
    };
    let refs = p
        .content
        .iter()
        .filter(|i| matches!(i, tmark::ir::Inline::Ref(_)))
        .count();
    assert_eq!(refs, 2);
}
