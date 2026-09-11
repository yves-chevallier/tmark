//! PyO3 native module `tmark._tmark`, wrapped by the Python package `tmark`.
//!
//! Design: `design/09-bindings.md` §Python. Every function is a thin
//! translation of the facade: text and options in, JSON-shaped values out
//! through `pythonize` (ADR 0003). No language logic lives here; the one
//! thing this crate owns is the `Loader` seam, where a Python object with
//! `load(from_path, rel) -> str | None` becomes a Rust `Loader`.
//!
//! The docstring of every function starts with its Python signature; the
//! stub `python/tmark/_tmark.pyi` is generated from these lines
//! (`scripts/gen_stubs.py`), so they are the one definition of the API.

#![forbid(unsafe_code)]
// The `#[pyfunction]` expansion of pyo3 0.22 converts a `PyErr` into a
// `PyErr`; clippy 1.98 flags it on every function returning `PyResult`.
#![allow(clippy::useless_conversion)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pythonize::{depythonize, pythonize};
use serde::Deserialize;
use serde_json::{json, Value};
use tmark::ir::registry::{
    ADMONITIONS, DEPRECATIONS, FEATURES, FRAGMENTS, KEY_LABELS, LANG_DEFAULT_NODE_WORDS,
    NODE_WORDS, PREFIXES, ROLES,
};
use tmark::ir::{Block, Code, Inline, LineIndex, NodeId, NodeRef};
use tmark::{
    Backend, BookLabel, Diagnostic, Document, FileId, FsLoader, LintConfig, Loader, NodeEdit,
    Profile, Replacement, ResolveNumbering, ResolveOptions, Resolved, WebOptions, WriterOptions,
};

// ---------------------------------------------------------------------------
// Boundary helpers
// ---------------------------------------------------------------------------

fn profile_of(name: &str) -> PyResult<Profile> {
    match name {
        "default" | "canonical" => Ok(Profile::Canonical),
        "strict" => Ok(Profile::Strict),
        "mkdocs" => Ok(Profile::Mkdocs),
        other => Err(PyValueError::new_err(format!(
            "unknown profile `{other}` (default, canonical, strict, mkdocs)"
        ))),
    }
}

fn to_py<'py>(py: Python<'py>, value: &Value) -> PyResult<Bound<'py, PyAny>> {
    pythonize(py, value).map_err(|e| PyValueError::new_err(e.to_string()))
}

fn from_py<T: for<'de> Deserialize<'de>>(obj: &Bound<'_, PyAny>, what: &str) -> PyResult<T> {
    depythonize(obj).map_err(|e| PyTypeError::new_err(format!("{what}: {e}")))
}

/// A document dict back into the IR, refusing another major version
/// (design 09 §Versioning).
fn document_of(obj: &Bound<'_, PyAny>) -> PyResult<Document> {
    let value: Value = from_py(obj, "doc")?;
    tmark::from_json(value).map_err(PyValueError::new_err)
}

/// The JSON of the diagnostics plus `stage` and, for the main file,
/// `path`, `line` and `col` (1-based, byte column: decision X10,
/// `tmark-cli`'s convention). Other files and an unknown text get `None`.
fn diagnostics_json(
    diagnostics: &[Diagnostic],
    file: FileId,
    path: &str,
    index: Option<&LineIndex>,
) -> Value {
    Value::Array(
        diagnostics
            .iter()
            .map(|d| {
                let mut value = serde_json::to_value(d).expect("a diagnostic serialises");
                let main = d.span.file == file;
                let (line, col) = match index {
                    Some(index) if main => {
                        let at = index.line_col(d.span.start);
                        (json!(at.line + 1), json!(at.col + 1))
                    }
                    _ => (Value::Null, Value::Null),
                };
                if let Value::Object(map) = &mut value {
                    map.insert("stage".into(), json!(d.code.stage()));
                    map.insert("path".into(), if main { json!(path) } else { Value::Null });
                    map.insert("line".into(), line);
                    map.insert("col".into(), col);
                }
                value
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// The `options` dict of `lint`, `fixes` and `resolve`: the fields of
/// `ResolveOptions`, the lint levels and the profile.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Options {
    /// Path of the document, relative to which includes and sources load.
    /// Defaults to `file` when that is a real path.
    path: Option<String>,
    /// `.bib` files, relative to the document's directory.
    bibliography: Vec<String>,
    /// First value of each tmark-numbered series (the previous
    /// document's `next_start`).
    start: BTreeMap<String, u32>,
    /// `backend` (user series only) or `all` (every series, for the web).
    numbering: ResolveNumbering,
    /// Language of the label words (`fr`, `de-CH`); the front matter's
    /// `lang`, then English, when absent.
    lang: Option<String>,
    /// Labels of the other documents of the book (`book` entries of their
    /// `resolve` results): what a key defined nowhere here may resolve to.
    book: Vec<BookLabel>,
    /// `default`, `canonical`, `strict` or `mkdocs`.
    profile: Option<String>,
    /// Lint levels by code: `off`, `hint`, `info`, `warning`, `error`.
    levels: BTreeMap<String, String>,
}

impl Options {
    fn parse(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Options> {
        match obj {
            Some(obj) if !obj.is_none() => from_py(obj, "options"),
            _ => Ok(Options::default()),
        }
    }

    fn resolve(&self, file: &str) -> ResolveOptions {
        let path = match &self.path {
            Some(path) => PathBuf::from(path),
            None if file == "<memory>" => PathBuf::new(),
            None => PathBuf::from(file),
        };
        ResolveOptions {
            path,
            bibliography: self.bibliography.iter().map(PathBuf::from).collect(),
            start: self.start.clone(),
            numbering: self.numbering,
            lang: self.lang.clone(),
            book: self.book.clone(),
        }
    }

    fn profile(&self) -> PyResult<Profile> {
        profile_of(self.profile.as_deref().unwrap_or("default"))
    }

    fn lint(&self) -> PyResult<LintConfig> {
        let mut config = LintConfig::default();
        for (code, level) in &self.levels {
            let Some(code) = Code::from_id(code) else {
                return Err(PyValueError::new_err(format!(
                    "unknown diagnostic code `{code}`"
                )));
            };
            config.set(code, level).map_err(PyValueError::new_err)?;
        }
        Ok(config)
    }
}

// ---------------------------------------------------------------------------
// The Loader seam
// ---------------------------------------------------------------------------

/// A Python object with `load(from_path: str, rel: str) -> str | None`.
/// The GIL is taken for the duration of each call only; the first Python
/// exception is kept and raised once the Rust stage returns.
struct PyLoader {
    object: Py<PyAny>,
    error: Mutex<Option<PyErr>>,
}

impl PyLoader {
    fn take_error(&self) -> PyResult<()> {
        match self.error.lock().map(|mut e| e.take()) {
            Ok(Some(error)) => Err(error),
            _ => Ok(()),
        }
    }
}

impl Loader for PyLoader {
    fn load(&self, from: &Path, rel: &str) -> Option<String> {
        if self.error.lock().map_or(true, |e| e.is_some()) {
            return None;
        }
        Python::with_gil(|py| {
            let from = from.to_string_lossy();
            let result = self
                .object
                .bind(py)
                .call_method1("load", (from.as_ref(), rel))
                .and_then(|value| {
                    if value.is_none() {
                        Ok(None)
                    } else {
                        value.extract::<String>().map(Some)
                    }
                });
            match result {
                Ok(text) => text,
                Err(error) => {
                    if let Ok(mut slot) = self.error.lock() {
                        *slot = Some(error);
                    }
                    None
                }
            }
        })
    }
}

enum AnyLoader {
    Fs(FsLoader),
    Py(PyLoader),
}

impl AnyLoader {
    /// `None` is the file system (the `fs` feature is on for this crate).
    fn of(obj: Option<&Bound<'_, PyAny>>) -> PyResult<AnyLoader> {
        match obj {
            Some(obj) if !obj.is_none() => {
                if !obj.hasattr("load")? {
                    return Err(PyTypeError::new_err(
                        "loader must have a `load(from_path, rel) -> str | None` method",
                    ));
                }
                Ok(AnyLoader::Py(PyLoader {
                    object: obj.clone().unbind(),
                    error: Mutex::new(None),
                }))
            }
            _ => Ok(AnyLoader::Fs(FsLoader)),
        }
    }

    fn as_dyn(&self) -> &dyn Loader {
        match self {
            AnyLoader::Fs(fs) => fs,
            AnyLoader::Py(py) => py,
        }
    }

    fn take_error(&self) -> PyResult<()> {
        match self {
            AnyLoader::Fs(_) => Ok(()),
            AnyLoader::Py(py) => py.take_error(),
        }
    }
}

/// The opaque handle `resolve` returns next to its JSON view, so that
/// `write` renders every slot of a document against one resolution
/// (numbering never restarts per slot). `Resolved` is not rebuilt from
/// its view: the view is for reading, the handle for writing.
#[pyclass(name = "Resolved", module = "tmark._tmark", frozen)]
struct PyResolved(Resolved);

#[pymethods]
impl PyResolved {
    fn __repr__(&self) -> String {
        format!(
            "<tmark.Resolved: {} labels, {} refs, {} diagnostics>",
            self.0.labels.in_order.len(),
            self.0.refs.len(),
            self.0.diagnostics.len()
        )
    }

    /// view() -> dict[str, Any]
    ///
    /// The JSON view of this resolution (`schema("resolved")`, without
    /// the `line`/`col` of `resolve`).
    fn view<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        to_py(
            py,
            &serde_json::to_value(self.0.view()).expect("the view serialises"),
        )
    }
}

/// Parse, resolve, lint, with fixes attached; the shared body of `lint`
/// and `fixes`.
fn check(
    py: Python<'_>,
    text: &str,
    file: &str,
    loader: Option<&Bound<'_, PyAny>>,
    options: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<Diagnostic>> {
    let options = Options::parse(options)?;
    let profile = options.profile()?;
    let lint = options.lint()?;
    let resolve = options.resolve(file);
    let loader = AnyLoader::of(loader)?;
    let (_, diagnostics) = py.allow_threads(|| {
        tmark::check(
            text,
            FileId::default(),
            profile,
            loader.as_dyn(),
            &resolve,
            &lint,
        )
    });
    loader.take_error()?;
    Ok(diagnostics)
}

// ---------------------------------------------------------------------------
// The module
// ---------------------------------------------------------------------------

/// parse(text: str, file: str = "<memory>", file_id: int = 0, profile: str = "default") -> dict[str, Any]
///
/// Parse a TMark text. Returns the document as JSON (design 03 §Serialization)
/// with `"tmark": "<version>"` as its first key and the parse `"diagnostics"`
/// as its last. `file` names the text in the diagnostics; `file_id` is the
/// id every span carries; `profile` is `default` (`canonical`), `strict`
/// or `mkdocs`. Parsing never fails.
#[pyfunction]
#[pyo3(signature = (text, file = "<memory>", file_id = 0, profile = "default"))]
fn parse<'py>(
    py: Python<'py>,
    text: &str,
    file: &str,
    file_id: u32,
    profile: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let profile = profile_of(profile)?;
    let id = FileId(file_id);
    let parsed = py.allow_threads(|| tmark::parse_with(text, id, profile));
    let index = LineIndex::new(text);
    let mut value = tmark::to_json(&parsed.document);
    if let Value::Object(map) = &mut value {
        map.insert(
            "diagnostics".into(),
            diagnostics_json(&parsed.diagnostics, id, file, Some(&index)),
        );
    }
    to_py(py, &value)
}

/// format(text: str, profile: str = "canonical") -> str
///
/// The canonical spelling of a text (design 04): `parse(format(text))`
/// is `parse(text)` and `format` is idempotent. The front matter is copied
/// byte for byte and prose is never re-wrapped.
#[pyfunction]
#[pyo3(signature = (text, profile = "canonical"))]
fn format(py: Python<'_>, text: &str, profile: &str) -> PyResult<String> {
    let profile = profile_of(profile)?;
    Ok(py.allow_threads(|| {
        let parsed = tmark::parse_with(text, FileId::default(), profile);
        tmark::format(&parsed.document, profile)
    }))
}

/// lint(text: str, file: str = "<memory>", loader: Loader | None = None, options: dict[str, Any] | None = None) -> list[dict[str, Any]]
///
/// Every diagnostic of a text: parse, resolve and lint, in that order,
/// with fixes attached (`tmark check`). Includes and sources load through
/// `loader` (the file system when `None`) relative to `options["path"]`
/// (`file` when it is a real path). `options` accepts `path`,
/// `bibliography` (list of `.bib` paths), `start` (prefix -> first value),
/// `numbering` (`backend` or `all`), `lang`, `book` (sibling labels),
/// `profile` and `levels` (code -> `off`/`hint`/`info`/`warning`/`error`).
/// Each diagnostic is the JSON of `tmark_ir::Diagnostic` (`code` as its
/// kebab-case id) plus `stage`, `path`, `line` and `col` (1-based, byte
/// column; `None` for another file).
#[pyfunction]
#[pyo3(signature = (text, file = "<memory>", loader = None, options = None))]
fn lint<'py>(
    py: Python<'py>,
    text: &str,
    file: &str,
    loader: Option<&Bound<'py, PyAny>>,
    options: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let diagnostics = check(py, text, file, loader, options)?;
    let index = LineIndex::new(text);
    to_py(
        py,
        &diagnostics_json(&diagnostics, FileId::default(), file, Some(&index)),
    )
}

/// fixes(text: str, file: str = "<memory>", loader: Loader | None = None, options: dict[str, Any] | None = None) -> str
///
/// The text with every safe fix applied: what `tmark lint --fix` writes
/// (deprecated spellings rewritten in their canonical form, design 05
/// §Fixes). `loader` and `options` are those of `lint`. The text comes back
/// unchanged when nothing has a fix.
#[pyfunction]
#[pyo3(signature = (text, file = "<memory>", loader = None, options = None))]
fn fixes(
    py: Python<'_>,
    text: &str,
    file: &str,
    loader: Option<&Bound<'_, PyAny>>,
    options: Option<&Bound<'_, PyAny>>,
) -> PyResult<String> {
    let diagnostics = check(py, text, file, loader, options)?;
    let (fixed, _) = tmark::apply_fixes(text, FileId::default(), &diagnostics);
    Ok(fixed)
}

/// resolve(doc: dict[str, Any], loader: Loader | None = None, options: dict[str, Any] | None = None, text: str | None = None) -> dict[str, Any]
///
/// Build the registries of a parsed document and resolve its references
/// (design 06). `doc` is a `parse` result (its `"tmark"` version must
/// match); `loader` and `options` are those of `lint` (`path`,
/// `bibliography`, `start`, `numbering`, `lang`, `book`). The result is
/// `schema("resolved")`: `numbering` and `lang` as used, `counters` (every
/// series with its `numbers` and `next`), `next_start` (prefix -> the
/// first free value, for the next document of a build), `labels` (in
/// document order, with `formatted` numbers), `book` (this document's
/// labels for its siblings, located at `path#key`), `refs` (one per
/// reference, `resolution.kind` in `label`, `sibling`, `citation`,
/// `glossary`, `doi`, `external`, `ambiguous`, `unresolved`),
/// `bibliography` (keys), `entries`, `dois` (pending), `glossary`, `index`,
/// `crossrefs`, `included` (files loaded through includes), `diagnostics`
/// and `handle`, an opaque `tmark.Resolved` that `write` takes (pass this
/// whole dict, or the handle, as its `resolved`). Pass `text` to get
/// `line` and `col` on the diagnostics of the main file.
#[pyfunction]
#[pyo3(signature = (doc, loader = None, options = None, text = None))]
fn resolve<'py>(
    py: Python<'py>,
    doc: &Bound<'py, PyAny>,
    loader: Option<&Bound<'py, PyAny>>,
    options: Option<&Bound<'py, PyAny>>,
    text: Option<&str>,
) -> PyResult<Bound<'py, PyAny>> {
    let document = document_of(doc)?;
    let options = Options::parse(options)?;
    let resolve = options.resolve("<memory>");
    let loader = AnyLoader::of(loader)?;
    let resolved = py.allow_threads(|| tmark::resolve(&document, loader.as_dyn(), &resolve));
    loader.take_error()?;
    let mut value = serde_json::to_value(resolved.view()).expect("the view serialises");
    let index = text.map(LineIndex::new);
    let path = options.path.as_deref().unwrap_or("<memory>");
    if let Value::Object(map) = &mut value {
        map.insert(
            "diagnostics".into(),
            diagnostics_json(&resolved.diagnostics, document.file, path, index.as_ref()),
        );
    }
    let out = to_py(py, &value)?;
    out.set_item("handle", Py::new(py, PyResolved(resolved))?)?;
    Ok(out)
}

/// edit(text: str, doc: dict[str, Any], node_id: int, replacement: dict[str, Any]) -> str
///
/// Replace one node of `text` (the source `doc` was parsed from) by the
/// canonical spelling of `replacement`, a Block or Inline node as JSON,
/// leaving every other byte untouched (design 04 §Local edits, ADR 0004).
/// Raises `ValueError` when `node_id` is not in the document.
#[pyfunction]
fn edit(
    text: &str,
    doc: &Bound<'_, PyAny>,
    node_id: u32,
    replacement: &Bound<'_, PyAny>,
) -> PyResult<String> {
    let document = document_of(doc)?;
    let edit = node_edit(&document, node_id, replacement)?;
    Ok(tmark::edit(text, &document, edit))
}

/// A `NodeEdit` from a node id and a Block or Inline as JSON; the node's
/// kind decides which (`Comment` exists in both).
fn node_edit(
    document: &Document,
    node_id: u32,
    replacement: &Bound<'_, PyAny>,
) -> PyResult<NodeEdit> {
    let id = NodeId(node_id);
    let Some(node) = tmark::ir::find(document, id) else {
        return Err(PyValueError::new_err(format!(
            "node {node_id} is not in the document"
        )));
    };
    let replacement = match node {
        NodeRef::Block(_) => Replacement::Block(from_py::<Block>(replacement, "replacement")?),
        NodeRef::Inline(_) => Replacement::Inline(from_py::<Inline>(replacement, "replacement")?),
    };
    Ok(NodeEdit { id, replacement })
}

/// edit_many(text: str, doc: dict[str, Any], edits: list[dict[str, Any]]) -> str
///
/// Apply several local edits in one pass: each item is
/// `{"node_id": int, "replacement": dict}` with the replacement shape of
/// `edit`. Spans must be disjoint; an unknown node, a span outside the
/// text or two overlapping edits raise `ValueError` and nothing is applied
/// (design 04 §Local edits).
#[pyfunction]
fn edit_many(text: &str, doc: &Bound<'_, PyAny>, edits: &Bound<'_, PyAny>) -> PyResult<String> {
    let document = document_of(doc)?;
    let mut node_edits = Vec::new();
    for item in edits.iter()? {
        let item = item?;
        let node_id: u32 = item
            .get_item("node_id")
            .and_then(|v| v.extract())
            .map_err(|e| PyTypeError::new_err(format!("edits: node_id: {e}")))?;
        let replacement = item
            .get_item("replacement")
            .map_err(|e| PyTypeError::new_err(format!("edits: replacement: {e}")))?;
        node_edits.push(node_edit(&document, node_id, &replacement)?);
    }
    tmark::edit_many(text, &document, node_edits).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// write(doc: dict[str, Any], backend: str, options: dict[str, Any] | None = None, loader: Loader | None = None, resolved: Resolved | dict[str, Any] | None = None, resolve_options: dict[str, Any] | None = None) -> dict[str, Any]
///
/// Render a document for a backend (`html`, `latex`, `typst`): a `Body`
/// as `{"text", "map", "requires"}` (design 07). `map` is
/// `[[start, end, node_id], ...]` over the output bytes, filled when
/// `options["source_map"]` is on; `requires` lists `packages`,
/// `fragments`, `shell_escape`, `assets`, `bibliography`, `citations`,
/// `acronyms`, `index` and `counters`. `options` maps one to one onto
/// `WriterOptions`, every key optional: `media` (`print` | `web`), `lang`,
/// `code` {`engine` (`pygments` | `minted` | `listings` | `verbatim`),
/// `inline_plain`, `inline_breaks`}, `latex` {`legacy_accents`},
/// `headings` {`base_level`, `numbered`}, `refs` {`textual_print`,
/// `textual_web`}, `numbering` (prefix -> `backend` | `tmark`), `typst`
/// {`math` (`mitex` | `native`)}, `source_map`. An unknown key is a
/// `TypeError`, a bad value a `ValueError`. `resolved` is the dict
/// `resolve` returned, or its `handle`: pass the same one for every slot
/// of a document so numbering never restarts; `None` resolves now, through
/// `loader` with `resolve_options` (the `options` of `resolve`).
#[pyfunction]
#[pyo3(signature = (doc, backend, options = None, loader = None, resolved = None, resolve_options = None))]
fn write<'py>(
    py: Python<'py>,
    doc: &Bound<'py, PyAny>,
    backend: &str,
    options: Option<&Bound<'py, PyAny>>,
    loader: Option<&Bound<'py, PyAny>>,
    resolved: Option<&Bound<'py, PyAny>>,
    resolve_options: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let document = document_of(doc)?;
    let Some(backend) = Backend::parse(backend) else {
        return Err(PyValueError::new_err(format!(
            "unknown backend `{backend}` (html, latex, typst)"
        )));
    };
    let options = writer_options(options)?;
    let handle = resolved_handle(resolved)?;
    let own;
    let resolved: &Resolved = match &handle {
        Some(handle) => &handle.get().0,
        None => {
            let resolve = Options::parse(resolve_options)?.resolve("<memory>");
            let loader = AnyLoader::of(loader)?;
            own = py.allow_threads(|| tmark::resolve(&document, loader.as_dyn(), &resolve));
            loader.take_error()?;
            &own
        }
    };
    let body = py.allow_threads(|| tmark::write(&document, resolved, backend, &options));
    to_py(py, &serde_json::to_value(body).expect("a body serialises"))
}

/// lower_web(text: str, doc: dict[str, Any], resolved: Resolved | dict[str, Any] | None = None, loader: Loader | None = None, options: dict[str, Any] | None = None) -> dict[str, Any]
///
/// Lower a page for a MkDocs site (design 07 §Web lowering, TeXSmith
/// `web-profile.md`): `text` is the source `doc` was parsed from; every
/// TMark construct of the per-construct table is spliced into what
/// Material renders (`<span class="ts-counter">FW-01</span>`,
/// `[FW-01](#fw:x)`, `<figure markdown="span">`, `!!! note`, sibling
/// links) and every other byte is kept, mkdocstrings directives
/// included. `resolved` is the `resolve` result or its `handle`, made
/// with `numbering: "all"` and the site's `book`; `None` resolves now
/// through `loader` with every series numbered. `loader` also serves the
/// text of included files (the file system when `None`). `options`:
/// `sections` (`title` | `number`, what `@sec:x` shows), `citations`
/// (`inline` | `passthrough`), `lang`, `css_prefix` (`ts-`); an unknown
/// key or value is a `TypeError`. Returns `{"text", "diagnostics",
/// "bibliography"}`: the lowered page (the `References` list appended
/// when citations were lowered inline), the lowering's own diagnostics
/// (with `line`/`col`), and the `References` list alone or `None`.
#[pyfunction]
#[pyo3(signature = (text, doc, resolved = None, loader = None, options = None))]
fn lower_web<'py>(
    py: Python<'py>,
    text: &str,
    doc: &Bound<'py, PyAny>,
    resolved: Option<&Bound<'py, PyAny>>,
    loader: Option<&Bound<'py, PyAny>>,
    options: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let document = document_of(doc)?;
    let options: WebOptions = match options.filter(|o| !o.is_none()) {
        Some(obj) => from_py(obj, "options")?,
        None => WebOptions::default(),
    };
    let loader = AnyLoader::of(loader)?;
    let handle = resolved_handle(resolved)?;
    let own;
    let resolved: &Resolved = match &handle {
        Some(handle) => &handle.get().0,
        None => {
            let resolve = ResolveOptions {
                numbering: ResolveNumbering::All,
                ..ResolveOptions::default()
            };
            own = py.allow_threads(|| tmark::resolve(&document, loader.as_dyn(), &resolve));
            loader.take_error()?;
            &own
        }
    };
    let lowered =
        py.allow_threads(|| tmark::lower_web(text, &document, resolved, loader.as_dyn(), &options));
    loader.take_error()?;
    let index = LineIndex::new(text);
    let path = resolved.path.to_string_lossy();
    let path = if path.is_empty() { "<memory>" } else { &path };
    let value = json!({
        "text": lowered.text,
        "diagnostics": diagnostics_json(&lowered.diagnostics, document.file, path, Some(&index)),
        "bibliography": lowered.bibliography,
    });
    to_py(py, &value)
}

/// The `resolved` argument of `write` and `lower_web`: a `tmark.Resolved`,
/// or the dict `resolve` returned (its `handle`), or nothing.
fn resolved_handle(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Py<PyResolved>>> {
    let Some(obj) = obj.filter(|o| !o.is_none()) else {
        return Ok(None);
    };
    if let Ok(handle) = obj.extract::<Py<PyResolved>>() {
        return Ok(Some(handle));
    }
    obj.get_item("handle")
        .and_then(|h| h.extract::<Py<PyResolved>>())
        .map(Some)
        .map_err(|_| {
            PyTypeError::new_err("resolved must be a tmark.Resolved or the dict resolve() returned")
        })
}

/// `WriterOptions` from a dict: every key must exist in the default
/// options at the same path (`TypeError` otherwise; `numbering` keys are
/// free), then serde decodes the values (`ValueError`).
fn writer_options(obj: Option<&Bound<'_, PyAny>>) -> PyResult<WriterOptions> {
    let Some(obj) = obj.filter(|o| !o.is_none()) else {
        return Ok(WriterOptions::default());
    };
    let value: Value = from_py(obj, "options")?;
    let known = serde_json::to_value(WriterOptions::default()).expect("options serialise");
    check_keys(&value, &known, &mut Vec::new())?;
    serde_json::from_value(value).map_err(|e| PyValueError::new_err(format!("options: {e}")))
}

fn check_keys(value: &Value, known: &Value, path: &mut Vec<String>) -> PyResult<()> {
    let (Value::Object(given), Value::Object(known)) = (value, known) else {
        return Ok(());
    };
    for (key, inner) in given {
        let Some(expected) = known.get(key) else {
            let at = path.iter().map(|p| format!("{p}.")).collect::<String>();
            return Err(PyTypeError::new_err(format!(
                "options: unknown key `{at}{key}`"
            )));
        };
        if key != "numbering" {
            path.push(key.clone());
            check_keys(inner, expected, path)?;
            path.pop();
        }
    }
    Ok(())
}

/// schema(name: str) -> dict[str, Any]
///
/// A JSON schema by name: `ir` (a document), `frontmatter` (the typed
/// keys), `diagnostic` (one diagnostic), `resolved` (a `resolve` result).
/// Generated from the Rust types (ADR 0003); TeXSmith generates its models
/// from `ir` and checks `schema_hash()` against the one it recorded.
#[pyfunction]
fn schema<'py>(py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    match tmark::schema(name) {
        Some(schema) => to_py(py, &schema),
        None => Err(PyValueError::new_err(format!(
            "unknown schema `{name}` (ir, frontmatter, diagnostic, resolved)"
        ))),
    }
}

/// schema_hash() -> str
///
/// A stable hash of `schema("ir")` (16 hexadecimal digits, the same on
/// every platform): equal for two builds whose IR shapes are identical.
#[pyfunction]
fn schema_hash() -> String {
    tmark::schema_hash()
}

/// codes() -> list[dict[str, Any]]
///
/// The diagnostic catalogue in order: `{"id", "severity", "stage", "doc"}`
/// per code, `severity` being the default one and `stage` `parse`,
/// `resolve` or `lint` (design 05 §Who emits what).
#[pyfunction]
fn codes(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    let value: Vec<Value> = Code::ALL
        .iter()
        .map(|code| {
            json!({
                "id": code.id(),
                "severity": code.default_severity(),
                "stage": code.stage(),
                "doc": code.doc(),
            })
        })
        .collect();
    to_py(py, &Value::Array(value))
}

/// fragments() -> list[dict[str, Any]]
///
/// The fragment-contract table (design 07): one row per contract a
/// writer's `requires.fragments` can name, `{"name", "provides",
/// "packages", "shell_escape", "description"}`.
#[pyfunction]
fn fragments(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    let value: Vec<Value> = FRAGMENTS
        .iter()
        .map(|f| {
            json!({
                "name": f.name,
                "provides": f.provides,
                "packages": f.packages,
                "shell_escape": f.shell_escape,
                "description": f.description,
            })
        })
        .collect();
    to_py(py, &Value::Array(value))
}

/// registries() -> dict[str, Any]
///
/// The closed registries of the IR as tables (design 03 §Closed
/// registries): `roles`, `node_words`, `lang_default_node_words`,
/// `prefixes`, `admonitions`, `features`, `deprecations`, `key_labels`
/// (keystroke name -> label). One definition, in `tmark_ir::registry`;
/// generate from these, never copy them.
#[pyfunction]
fn registries(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    let value = json!({
        "roles": ROLES,
        "node_words": NODE_WORDS,
        "lang_default_node_words": LANG_DEFAULT_NODE_WORDS
            .iter()
            .map(|(lang, word)| json!({"lang": lang, "word": word}))
            .collect::<Vec<_>>(),
        "prefixes": PREFIXES,
        "admonitions": ADMONITIONS,
        "features": FEATURES,
        "deprecations": DEPRECATIONS,
        "key_labels": KEY_LABELS
            .iter()
            .map(|k| json!({"name": k.name, "label": k.label}))
            .collect::<Vec<_>>(),
    });
    to_py(py, &value)
}

/// version() -> str
///
/// The version of the native module, the one the IR JSON root carries.
#[pyfunction]
fn version() -> &'static str {
    tmark::VERSION
}

#[pymodule]
fn _tmark(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", tmark::VERSION)?;
    m.add_class::<PyResolved>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(format, m)?)?;
    m.add_function(wrap_pyfunction!(lint, m)?)?;
    m.add_function(wrap_pyfunction!(fixes, m)?)?;
    m.add_function(wrap_pyfunction!(resolve, m)?)?;
    m.add_function(wrap_pyfunction!(edit, m)?)?;
    m.add_function(wrap_pyfunction!(edit_many, m)?)?;
    m.add_function(wrap_pyfunction!(write, m)?)?;
    m.add_function(wrap_pyfunction!(lower_web, m)?)?;
    m.add_function(wrap_pyfunction!(schema, m)?)?;
    m.add_function(wrap_pyfunction!(schema_hash, m)?)?;
    m.add_function(wrap_pyfunction!(codes, m)?)?;
    m.add_function(wrap_pyfunction!(fragments, m)?)?;
    m.add_function(wrap_pyfunction!(registries, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
