# 10 — Testing strategy

Tests are the specification made executable. Four layers, each with a single
job.

## 1. Conformance fixtures (`spec/conformance/`)

One Markdown file per construct, format in `spec/conformance/README.md`.
Each fixture states the input (one or more sugar spellings), the canonical
form, the IR (JSON), the diagnostics expected, and optionally the body each
backend must produce. The test runner (`crates/tmark/tests/conformance.rs`)
asserts, for every fixture:

- every input parses to the IR (modulo spans and ids);
- the canonical form prints from that IR and re-parses to it;
- the diagnostics match by code and span;
- the backend bodies match, when given, as `insta` snapshots.

The fixtures are the spec's roadmap item 2. A construct without a fixture is
not implemented; a spec change without a fixture change is not a spec
change.

## 2. CommonMark spec tests

The vendored `spec.json` of the CommonMark version upstream tracks. Run
through the HTML writer, compared as in the reference implementation
(normalised HTML). Exceptions are listed by example number with the X-class
deviation that explains each (`__x__` → small caps, `---` still an `<hr>` in
HTML so no exception there, `~x~`, `@word`, `#[`). Any other failure is a
bug.

## 3. Property tests (`proptest`)

- Round-trip: `parse(print(doc)) == doc` for generated documents (bounded
  depth, every node kind, attributes with awkward values, nested containers).
- Idempotence of `fmt`.
- Local edit: `edit(text, doc, id, doc[id])` is the identity on `text`.
- Parser totality: arbitrary bytes never panic and always yield a document
  whose spans are within bounds and non-overlapping among siblings.

## 4. Unit and snapshot tests

Per crate, close to the code: escaping tables, attribute parsing, item
grammar of bracketed references, `LineIndex` UTF-16 conversion, `Requires`
accumulation. Writers use `insta` snapshots on the fixture corpus, one
snapshot per backend per fixture.

## Fixtures for the editor

`editors/vscode/test/sample.md` and its scope assertions stay as the
grammar's test until the grammar is generated from `tmark-ir`, at which
point the sample becomes a conformance fixture too.

## CI

`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`,
`cargo run -p tmark-ir --example schema && git diff --exit-code` (generated
artifacts are committed and current), `npm test` in `editors/vscode`,
`maturin build` on the three platforms at release.

## Performance

A `criterion` benchmark on a 1 MB synthetic document (the fixture corpus
concatenated ×N) guards the parser target of `02-syntax.md`. Regressions
over 20 % fail CI.
