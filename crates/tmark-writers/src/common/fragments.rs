//! Names of the fragment contracts the writers emit (fragment-contracts.md
//! §2). TODO(registry): replace these constants by `tmark_ir::registry::
//! FRAGMENTS` once the `wt/registry` branch is merged; the names are the
//! `press.fragments` spellings and must stay identical.

/// `\tslead`, `\tsmark`, `\tsdivider`, `\tsepigraph`, `\tsaside`,
/// `\tsprogress`, `\tsicon`, `tsdiv`.
pub const TYPESETTING: &str = "ts-typesetting";
/// `tscallout`.
pub const CALLOUTS: &str = "ts-callouts";
/// `tscode`, `\tscodeinline`.
pub const CODE: &str = "ts-code";
/// `\tskeys`.
pub const KEYSTROKES: &str = "ts-keystrokes";
/// `tstasklist`, `\tsdone`, `\tstodo`, `\tspartial`.
pub const TODOLIST: &str = "ts-todolist";
/// `\tsgls`, `\tsacr`.
pub const GLOSSARY: &str = "ts-glossary";
/// `\tsindex`.
pub const INDEX: &str = "ts-index";
/// `\parencite`, `\textcite` fallbacks without biblatex.
pub const BIBLIOGRAPHY: &str = "ts-bibliography";
/// `\tsscript`, `\tsemoji`.
pub const FONTS: &str = "ts-fonts";
/// Typst: the template turns `math.equation(numbering)` on.
pub const EQUATIONS: &str = "ts-equations";
