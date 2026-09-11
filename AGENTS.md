# Working rules for implementers

This file is for whoever implements TMark from this repository, agent or
human. It is deliberately short. The design documents carry the detail.

## Start here

1. Read `README.md`, `design/00-overview.md`, `design/01-architecture.md`.
2. Read `spec/tmark.md` §Document model, §Lexical grammar, then the section of
   the node catalogue relevant to your task.
3. Read the design document and the ADRs of the part you touch.
4. Check `design/11-roadmap.md` for the milestone you are in and its
   definition of done. Do not start a later milestone's work early.
5. Read `design/13-handoff.md`: the previous agent's state, self-critique
   and review mandates. Run the reviews it asks for before building on
   the crates they concern.

## Principles, in priority order

1. **The spec is the source of truth.** If the implementation and
   `spec/tmark.md` disagree, either the code is wrong or the spec has a bug.
   A spec bug is fixed *in the spec*, with a line in `design/12-spec-challenges.md`,
   before the code follows. Never encode a silent deviation.
2. **Pure core.** No crate under `crates/` performs I/O, spawns a process,
   reads the clock, or touches the network, except `tmark-cli`, `tmark-lsp`
   and `tmark-py` at their edges, through the `Loader` trait
   (`design/06-registries.md`). A function that needs a file takes its
   content, or a `&dyn Loader`.
3. **One definition per fact (SSOT).** The IR is defined once in `tmark-ir`;
   the front-matter schema once, derived from Rust types; diagnostic rules
   once, in `tmark-lint`; the list of roles, node words, counter prefixes once,
   in `tmark-ir::registry`. Grammars, JSON schemas, Python stubs, editor
   snippets are *generated* from these, never hand-maintained twice.
4. **KISS and YAGNI.** Build the milestone's features, nothing speculative.
   Prefer a function to a trait, a trait to a framework, an enum to a plugin
   system. A generic mechanism is justified only by two concrete users that
   exist in this repository.
5. **SOLID where it pays.** Single responsibility per crate and per module;
   extension through data (registries, rule tables) rather than inheritance;
   the `Loader`, `Writer` and `Rule` traits are the only abstraction seams,
   and each has at least two implementations. No trait for one type.
6. **DRY across languages, not within reason.** Duplicating three lines is
   fine; duplicating a table of node names between Rust, Python and a grammar
   is not.

## Conventions

- Rust 2021, `cargo fmt`, `cargo clippy -- -D warnings`, no `unsafe`.
- Every node, diagnostic and configuration key carries the spec section it
  implements in a doc comment (`/// Spec §Roles`).
- Errors are values: parsing never fails; unrecognised input is literal text
  plus a diagnostic. Writers never panic on a valid IR.
- Tests are the conformance fixtures first (`spec/conformance/`), unit tests
  second, snapshots (`insta`) for writers and the `mkdocs` profile,
  property tests (`proptest`) for round-trip. A construct without a
  fixture is not implemented.
- Accept snapshots deliberately: after a parser, printer or writer change
  run `cargo insta review` (or read the diff `INSTA_UPDATE=always` makes)
  and accept each moved snapshot on its merits; never accept a run
  wholesale to make CI green. A new fixture needs one generation run and
  a reading of its new snapshots.
- Regenerate the fixture IR after a lowering change: `cargo build -p
  tmark-syntax --example dump && python3 scripts/fixture-ir.py`, then
  review the diff of the `ir` blocks before committing. The diagnostic
  and IR schemas and the registry tables are regenerated with the
  `schema` and `registries` examples of `tmark-ir`; CI fails when they
  drift.
- The Python surface is generated too: a changed `#[pyfunction]`
  signature or docstring in `crates/tmark-py/src/lib.rs` is followed by
  `python crates/tmark-py/scripts/gen_stubs.py` (a pytest checks the
  stub). Check `git status` after `maturin develop`: it drops a `.so`
  and `__pycache__` into the package, which stay untracked.
- Commits: `<scope>: <imperative summary>` with the crate as scope
  (`syntax: parse role heads`). One concern per commit.
- Do not add a dependency without a line in the crate's `Cargo.toml` comment
  saying why, and a mention in `design/01-architecture.md` if it crosses a
  crate boundary. The workspace MSRV is Rust 1.80 (`rust-version` in
  `Cargo.toml`): refuse a clippy suggestion that needs a newer std
  (`Option::is_none_or`) or raise the MSRV on purpose, in its own commit.
- A change to what crosses the TeXSmith boundary (a node field, a
  diagnostic code, a `FRAGMENTS` row, a `Requires` field, a `lower_web`
  wrapper, an option name of the bindings) is a two-repository change:
  say so in the commit and in `design/13-handoff.md` §The cross-repository
  contract.

## Definition of done for any task

- The behaviour is described in the spec or a design document.
- A conformance fixture or a test demonstrates it.
- `cargo test --workspace`, `cargo clippy`, `cargo fmt --check` pass.
- Generated artifacts (schema, grammar tables, stubs) are regenerated and
  committed.
- `design/11-roadmap.md` is updated if a milestone item is completed.

## What not to do

- Do not implement custom user-defined syntax. The spec excludes it (P4, P6).
- Do not build a lossless concrete syntax tree. The spec chooses spans plus
  local edits (§Round-trip and source spans, ADR 0004).
- Do not add I/O to the core to "simplify" a feature. Route it through
  `Loader` or move the feature to TeXSmith.
- Do not port TeXSmith's templates, fonts or build orchestration here.
