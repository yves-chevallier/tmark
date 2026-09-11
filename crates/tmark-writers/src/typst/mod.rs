//! The Typst writer. Design 07; construct catalogue in
//! `specs/migration/writers-and-passes.md` §2.

pub mod escape;

use tmark_ir::Document;
use tmark_registry::Resolved;

use crate::{Backend, Body, Writer, WriterOptions};

/// The Typst writer (skeleton; constructs land one at a time, milestone 4).
#[derive(Debug, Default, Clone, Copy)]
pub struct TypstWriter;

impl Writer for TypstWriter {
    fn backend(&self) -> Backend {
        Backend::Typst
    }

    fn write(&self, _doc: &Document, _res: &Resolved, opts: &WriterOptions) -> Body {
        let out = crate::common::Out::new(opts.source_map);
        let (text, map) = out.finish();
        Body {
            text,
            map,
            requires: Default::default(),
        }
    }
}
