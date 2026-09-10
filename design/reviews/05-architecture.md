# Review 05 — Architecture (handoff mandate 5)

Reviewed at `d4ad58a` (`main`). Two commits landed while the review ran
(`b7ff413`, `tmark.toml` in the facade; `d4ad58a`, semantic tokens) and were
folded in; the first bears on the last question.
Method: read `AGENTS.md`, `01`, `06`, `07`, `08`, `13`, the ADRs; grepped every
library crate for I/O; drew the Cargo graph with `cargo tree`; regenerated
the schema and the grammar and diffed. Findings are ranked: rule violations,
then drift between documents and code, then recommendations. Each carries
evidence and an action.

## A. Rule violations

### A1. The `fs` feature gate is ineffective; `FsLoader` is compiled everywhere

`crates/tmark/Cargo.toml:9-10` declares `fs = ["tmark-registry/fs"]` and
depends on `tmark-registry` with `default-features = false`. But
`crates/tmark-lint/Cargo.toml:11` and `crates/tmark-writers/Cargo.toml:11`
depend on `tmark-registry` with its defaults, and `crates/tmark-registry/Cargo.toml:10`
makes `fs` a default. Feature unification therefore turns `fs` on in every
build of the facade:

```
$ cargo tree -p tmark --no-default-features -e features -i tmark-registry
tmark-registry feature "fs"
└── tmark-registry feature "default"
    ├── tmark-lint  ← tmark-lint feature "default" ← tmark
    └── tmark-writers ← tmark-writers feature "default" ← tmark
```

`crates/tmark-wasm/Cargo.toml:10` and `crates/tmark-py/Cargo.toml:10` depend on
`tmark` with defaults, so the WASM crate also enables `fs`
(`cargo tree -p tmark-wasm -e features -i tmark-registry` lists
`tmark-registry feature "fs"` twice). `06-registries.md` §Implementation notes
("off in WASM") is not true today. Nothing breaks yet because `std::fs`
compiles on `wasm32-unknown-unknown` and fails at run time, which is exactly
the silent kind of leak the gate exists to prevent.

Action:
- `crates/tmark-registry/Cargo.toml`: `default = []`.
- `crates/tmark-lint/Cargo.toml`, `crates/tmark-writers/Cargo.toml`:
  `tmark-registry = { path = "…", default-features = false }`.
- `crates/tmark-wasm/Cargo.toml`: `tmark = { path = "../tmark", default-features = false }`.
- Keep `tmark`'s `fs` default on (the CLI, LSP and PyO3 want it) and add to
  CI: `cargo tree -p tmark-wasm -e features | grep -q 'feature "fs"' && exit 1`.

### A2. I/O in the facade (`Config::discover`)

`crates/tmark/src/config.rs:90-110` (commit `b7ff413`) walks parent directories with `Path::is_file` and calls
`std::fs::read_to_string` from the facade, behind `#[cfg(feature = "fs")]`.
AGENTS.md principle 2 and ADR 0001 name four edge crates; `tmark` is not one
of them. The `toml` dependency is added to the workspace under the "Core"
comment (`Cargo.toml:30-31`, `crates/tmark/Cargo.toml:20-23`) without the mention in `01-architecture.md`
§Dependencies policy that AGENTS.md requires.

This is the same shape as the `FsLoader` deviation and can be accepted the
same way, but only once A1 makes `fs` a real opt-in; otherwise the facade
does unconditional I/O on WASM. `Config::parse(text, dir)` (config.rs:59-85)
is pure and belongs in the facade (two users: CLI and LSP).

Action: fix A1; keep `Config::parse` and `Config::discover`; add to
`01-architecture.md` §The facade a paragraph "Feature `fs`: `FsLoader` and
`Config::discover` are the only functions in the core that touch the file
system; both are absent without the feature", and add `toml` (facade only) to
§Dependencies policy. (`d4ad58a` already replaced the LSP's own directory
walk with `Config::discover`, `crates/tmark-lsp/src/lib.rs:248`, so the walk
is written once.)

### A3. SSOT: the caption-kind ↔ counter-prefix mapping is spelled three times

- `crates/tmark-lint/src/rules/caption_id.rs:26-28`: `CaptionKind::Table => "tbl"`, `Figure => "fig"`, `Listing => "lst"`.
- `crates/tmark-registry/src/collect.rs:35-39`: `Host::Table => Some("tbl")`, … `Host::Equation => Some("eq")`.
- `crates/tmark-lsp/src/outline.rs:142-144`: `CaptionKind::Table => "Table"`, … (the label words that `registry::PREFIXES` already carries, `crates/tmark-ir/src/registry.rs:217-219`).
- `crates/tmark-registry/src/collect.rs:217`: the heading-class list `["part", "chap", "sec", "app"]`, which is `PREFIXES` filtered on `Scope::Document` minus `note`, or better an explicit `heading: bool` field.
- `crates/tmark-lint/src/rules/hardcoded_number.rs:12`: `"Figure", "Table", "Listing", "Section", "Chapter", "Equation", "Appendix"` — the `label` column of `PREFIXES`.

Only `tmark-ir::registry` may define these (AGENTS.md principle 3). Action:
add `impl CaptionKind { pub fn prefix(self) -> &'static Prefix }` in
`tmark-ir` (or a `prefix` field on the `Prefix`-to-host relation), a
`Prefix::heading` flag, and have the four call sites read the registry.
`hardcoded_number` derives its word list from
`PREFIXES.iter().filter_map(|p| p.label)`.

### A4. SSOT: the grammar generator duplicates three registry tables

`editors/vscode/scripts/build_grammar.py:39-46` hard-codes `ROLES` (18
names), `DEPRECATED_ROLES` (4) and `NODE_WORDS` (`code|table-config|table|image|raw`).
These are `ROLES`, the `replaced_by` column and `NODE_WORDS` of
`crates/tmark-ir/src/registry.rs:82-115,135-158`. Admonition types and
counter prefixes are matched generically (`[\w-]+`, build_grammar.py:167,
469-488), so they are not duplicated. `FENCE_LANGUAGES` (:49-57) is
editor-only and stays.

The design already schedules the fix for M3 (`01-architecture.md`
§Repository layout, `08-lsp.md` §The VS Code extension). Recommendation on
the shape: the 718-line script is mostly regex recognisers of the spec, not
tables. Porting all of it to Rust to remove three lists is churn. The
smaller change that satisfies SSOT: a `tmark-ir` example (`registries`)
that writes `editors/vscode/scripts/registries.json` (roles with
`replaced_by`, node words, prefixes, admonitions), which `build_grammar.py`
reads instead of its constants; CI regenerates both and diffs. The Python
stays the generator of the regexes; the tables come from Rust.

### A5. Dependencies without a justification line

AGENTS.md: "Do not add a dependency without a line in the crate's
`Cargo.toml` comment saying why". Missing in:
- `crates/tmark-lint/Cargo.toml:10-11,14` (three deps, no comment).
- `crates/tmark-writers/Cargo.toml:10-12` — including `tmark-fmt`, an edge that no design document's table lists (see B3).
- `crates/tmark-registry/Cargo.toml:17-19` (`serde`, `serde_json`, `schemars`; they serve `inventory.rs`).
- `crates/tmark/Cargo.toml:14-19` (six workspace crates, none commented; `toml` and `serde` at :20-23 are).
- `crates/tmark-cli/Cargo.toml:17` (`serde_json`).
- `crates/tmark-py/Cargo.toml:10`, `crates/tmark-wasm/Cargo.toml:10`.

`tmark-lsp/Cargo.toml` and `tmark-ir/Cargo.toml` comply. Action: one line each.

### A6. Traits: one extra, one duplicated fact

Traits that cross crate boundaries are exactly `Loader`
(`crates/tmark-registry/src/loader.rs:7`, two impls at :61 and :73) and `Rule`
(`crates/tmark-lint/src/lib.rs:23`, five impls under `rules/`). `Writer` does
not exist yet (`crates/tmark-writers/src/lib.rs` is three lines). Compliant.

`crates/tmark-syntax/examples/dump.rs:30` defines `trait SeverityName` for one
type, in an example; harmless but it is the fourth copy of the severity
words: `crates/tmark-cli/src/main.rs:94-101` and `:238-246`,
`crates/tmark-lint/src/lib.rs:37-42` (the inverse, for `tmark.toml`),
`dump.rs:30-40`. Action: `Severity::as_str()` and `Severity::from_str()` in
`tmark-ir` next to `Code::id`/`Code::from_id`, used by the CLI, `LintConfig::set`,
`Config::parse` and the example.

## B. Drift between documents and code

### B1. `01-architecture.md` §The facade lists signatures the code does not have

| Document | Code |
| --- | --- |
| `resolve(doc, loader) -> Resolved` | `resolve(doc, loader, &ResolveOptions)` (`crates/tmark-registry/src/lib.rs:60`) |
| `lint(doc, res) -> Vec<Diagnostic>` | `lint(doc, res, text, &Config)` (`crates/tmark-lint/src/lib.rs:68`) |
| "exactly these entry points" | plus `parse_strict`, `check`, `schema`, `FsLoader`, `MemoryLoader`, `Config`, `Resolution` (`crates/tmark/src/lib.rs:10-21`) |
| `write(...)` | absent (M4; expected) |
| "all pure" | `FsLoader` re-export at `:16-17` and `Config::discover` are not |

Action: rewrite the block from the code and state the `fs` feature (A2).

### B2. `05-diagnostics.md` §Rules shows a trait the crate does not implement

Document: `default_severity(&self)` and `check(&self, doc, res, out)`. Code:
`Context { doc, resolved, text }` and no `default_severity` (the severity
comes from `Code::default_severity`, `crates/tmark-lint/src/lib.rs:53-58`).
The implementation notes say so; the main text still shows the old trait.
Action: replace the block.

### B3. `tmark-writers → tmark-fmt` is an undeclared edge

`crates/tmark-writers/Cargo.toml:12` depends on `tmark-fmt`. The diagram and
table in `01-architecture.md` give `tmark-writers` the dependencies
`tmark-ir, tmark-registry`; only the last paragraph of `07-writers.md`
implies the edge ("`tmark-writers::commonmark` re-exports `tmark-fmt`"). The
edge points downward, so it is legal; it is undocumented. See D2 for whether
it should exist at all.

### B4. `09-bindings.md` promises things the crates do differently

- `tmark lsp` as a CLI subcommand (§CLI) versus a separate `tmark-lsp` binary
  (`crates/tmark-lsp/Cargo.toml:9-11`, `editors/vscode/scripts/bundle-server.sh`).
  Two binaries are the better split (the CLI does not carry `lsp-server`);
  update the document.
- `tmark schema frontmatter|ir|inventory`: `tmark_ir::schema` knows
  `ir` and `frontmatter` only (`crates/tmark-ir/src/lib.rs:61-68`). The inventory
  schema that `06-registries.md` §Inventory format says TeXSmith generates
  its writer from does not exist: no `crates/tmark-registry/schema/`, nothing
  in `crates/tmark-ir/examples/schema.rs`, while
  `crates/tmark-ir/schema/README.md` says it "belongs to `tmark-registry`
  (milestone 2)". `Inventory` derives `JsonSchema`
  (`crates/tmark-registry/src/inventory.rs:14`), so the artifact is one
  function away. Action: move `schema(name)` into the facade (it is the
  only crate that sees both `tmark-ir` and `tmark-registry`), add
  `"inventory"`, write `crates/tmark-registry/schema/inventory.json` from
  the example, and extend the CI diff check of `10-testing.md`.
- "one file per subcommand": `crates/tmark-cli/src/main.rs` is one file of
  316 lines. Fine at this size; drop the sentence or split when `write` lands.

### B5. `08-lsp.md` versus `tmark-lsp`

- "Files not open are loaded through `FsLoader` … and cached by mtime": no
  cache; `crates/tmark-lsp/src/worker.rs:87` builds a fresh `FsLoader` per
  analysis. Acceptable at this stage; note it as an open item.
- "Diagnostics are published per file, including for included files":
  `crates/tmark-lsp/src/convert.rs:58` drops diagnostics whose `span.file`
  is not the open document. M3 item; the comment says so.
- The rest matches: synchronous parse on the main loop
  (`crates/tmark-lsp/src/lib.rs:261`), 150 ms debounce on the worker
  (`worker.rs:15,54`), snapshot reads, UTF-16 conversion through
  `LineIndex` (`convert.rs:14-33`).

### B6. Generated artifacts are current

`cargo run -q -p tmark-ir --example schema` and
`python3 editors/vscode/scripts/build_grammar.py` both rewrote their files
with no diff (`git status --porcelain` unchanged apart from the concurrent
agent's edits). Both artifacts carry a header naming their generator
(`crates/tmark-ir/schema/frontmatter.json`, `editors/vscode/syntaxes/tmark.tmLanguage.json:3-5`).
The missing artifact is the inventory schema (B4).

### B7. Purity outside library targets (accept and document)

Library code of every non-edge crate is free of `std::fs`, `std::io`,
`std::process`, `std::time`, `std::env`, `std::net`, `std::thread` except
`FsLoader` (`crates/tmark-registry/src/loader.rs:68-77`, gated) and the
`Config::discover` (A2). The vendored `tmark-markdown` has none.
Third-party dependencies of the core do no I/O: `biblatex` (string parser;
deps `paste`, `roman-numerals-rs`, `strum`, `unicode-normalization`,
`unscanny`), `serde_yaml_ng`, `schemars`, `unicode-id`, `toml`.

File-system use exists in examples and integration tests:
`crates/tmark-ir/examples/schema.rs:23,29`,
`crates/tmark-syntax/examples/bench.rs:9-25` (also `Instant`),
`crates/tmark-syntax/examples/dump.rs:9-10`,
`crates/tmark-fmt/tests/roundtrip.rs:44-53`,
`crates/tmark/tests/conformance.rs:62,133`. These are binaries reading
fixtures, not library code. Action: one sentence in `01-architecture.md`
§Dependencies policy: "the purity rule applies to library targets; examples
and integration tests may read the repository".

## C. Answers to the mandate's questions

### C1. Purity (question 1)

See A1, A2, B7. Nothing else leaks. The facade's `fs` default is the right
default for the three native edges once A1 makes `default-features = false`
mean what it says for WASM.

### C2. Dependency graph (question 2)

Actual graph (`cargo tree -e normal`, workspace crates only):

```
tmark-cli   tmark-lsp   tmark-py   tmark-wasm
     \          |          |          /
                 tmark
       /    |     |     |      \
tmark-writers  tmark-lint  tmark-fmt  tmark-syntax  tmark-ir
   |   |  \        |          |          |    \
   |   |   \       |          |          |     \
   |   |    tmark-registry ───┘          |      \
   |   |      |        \                 |       \
   |   |   tmark-syntax  tmark-ir        |        \
   |   └── tmark-fmt                     └── tmark-markdown
   └────── tmark-ir
```

Differences from `01-architecture.md`:
- `tmark-writers → tmark-fmt` (B3).
- `tmark → tmark-ir` and `tmark → tmark-syntax` directly (fine; the table
  says "all of the above").
- No upward or sideways edge. `tmark-fmt → tmark-syntax` is a dev-dependency
  only (`crates/tmark-fmt/Cargo.toml:14`), as the table says. No
  dev-dependency hides a real one: `tmark-lint`'s dev-dep on `tmark-syntax`
  (`Cargo.toml:14`) is for tests that parse fixtures; the library reaches
  the parser through `tmark-registry` anyway.
- Third-party: `crossbeam-channel`, `lsp-server`, `lsp-types`, `clap` at the
  edges only; `biblatex`, `serde_yaml_ng`, `schemars`, `serde`, `serde_json`,
  `unicode-id` in the core, all in the policy list; `toml` in the facade (A2).

Justification comments: see A5.

### C3. The facade (question 3)

Bindings do not reach below the facade. Every import in the edges goes
through `tmark` or `tmark::ir` (`crates/tmark-cli/src/main.rs:12-13`,
`crates/tmark-lsp/src/lib.rs:36-37`, `worker.rs:12`, `convert.rs:12`,
`outline.rs:10-12`, `semantic.rs:10-11`). `tmark::ir` is the facade's re-export of `tmark-ir`
(`crates/tmark/src/lib.rs:12`), which `01-architecture.md` allows. No
violation.

`tmark::check` (`crates/tmark/src/lib.rs:26-39`) belongs in the facade in
principle: composing the three stages is the facade's job. In practice it
has one user. The LSP cannot use it, for two reasons that are structural:
it parses on the main thread and resolves on the worker
(`lib.rs:261`, `worker.rs:87-89`), and `check` throws `Resolved` away, which
the LSP keeps for navigation and semantic tokens (`lib.rs:109-114`). So the
worker re-composes `resolve + lint` by hand (`worker.rs:87-89`), the exact duplication `check`
was meant to avoid.

Action: split the composition where the LSP splits it.
`tmark::analyse(doc, text, loader, options, lint) -> (Resolved, Vec<Diagnostic>)`
is the resolve+lint half; `check = parse + analyse`. Both edges use it.

Two more compositions are duplicated by the edges and should move into the
facade:
- profile → parser selection: `crates/tmark-cli/src/main.rs:298-302` and
  `crates/tmark-lsp/src/lib.rs:260-264` both write
  `if profile == Strict { parse_strict } else { parse }`. `Profile` lives in
  `tmark-fmt` and `tmark-syntax` cannot see it, so the facade is the one
  place for `parse_with(text, file, profile)`. (Moving `Profile` to
  `tmark-ir` would be cleaner still — it is a spec-level notion
  (§Conformance) that both the parser's `lower::Options { strict }` and the
  printer read — but that is a larger change; `parse_with` in the facade is
  enough for M3.)
- path relativisation: `crates/tmark-cli/src/main.rs:201-211` (`pathdiff`),
  `crates/tmark/src/config.rs:130-145` (`relative_to`) and
  `crates/tmark-registry/src/loader.rs:13-42` (`join`/`normalise`). The root
  cause is `ResolveOptions.bibliography` being "relative to the document"
  (`crates/tmark-registry/src/lib.rs:35-36`) while every caller holds paths
  relative to the working directory or to `tmark.toml`. Let
  `ResolveOptions.bibliography` carry paths as given and load them with
  `loader.load(Path::new(""), rel)`; `pathdiff` and `relative_to` disappear.

One SSOT nit in the LSP: `has_press_key` (`crates/tmark-lsp/src/lib.rs:482`)
re-scans the front matter delimiters that the parser already handles.
Parse first (it happens on open anyway, `lib.rs:261`), then detect from
`Document.front_matter` through a small accessor in `tmark-ir`.

### C4. `tmark-writers` versus `tmark-fmt` (question 4)

`tmark-fmt` is the CommonMark/TMark printer with a `Profile` enum whose
`Mkdocs` variant already exists as a placeholder
(`crates/tmark-fmt/src/lib.rs:21-33`). `07-writers.md` puts a `commonmark`
module in `tmark-writers` that "re-exports `tmark-fmt`", `WriterOptions`
carries a `profile` "for CommonMark", and `11-roadmap.md` §M4 files the
"`Mkdocs` profile" under `tmark-writers`. That is one feature with two
homes and a dependency edge (B3) to serve it.

The MkDocs output is IR → text with different spellings; that is what a
printer profile is (`04-printer.md` §Profiles). It needs no `Resolved` and
no `Requires`. A `Writer` implementation for it would take `&Resolved` and
`&WriterOptions` and ignore both, and return a `Body` whose `SourceMap` has
no consumer (`07-writers.md` §Source maps lists LaTeX, Typst and HTML
consumers only). That is a trait implementation for uniformity, which
AGENTS.md principle 4 forbids.

Recommendation:
- The CommonMark/MkDocs profile lives in `tmark-fmt`, as `Profile::Mkdocs`.
  Amend `11-roadmap.md` §M4 ("`Mkdocs` profile" → in `tmark-fmt`) and
  `07-writers.md` (drop the `commonmark` module and the `profile` option).
- `tmark-writers` at M4 contains: the `Writer` trait, `Body`, `Requires`,
  `SourceMap`, `common.rs` (escaping, attribute and label formatting from
  the counter `ref` templates), and `html`, `latex`, `typst`. `Backend` has
  three variants. `tmark::write` dispatches on them; `tmark::format` is the
  fourth output. Python's `tmark.write(doc, "commonmark")` in
  `09-bindings.md` becomes `tmark.format`.
- Remove `tmark-fmt` from `crates/tmark-writers/Cargo.toml`. The graph then
  matches the diagram without editing it.

If a CommonMark source map ever gets a consumer, `tmark-fmt::Out`
(`crates/tmark-fmt/src/out.rs`) is where to record spans; the writer wrapper
still would not be needed.

### C5. Crate count (question 5)

Twelve crates. Line counts are library sources excluding tests.

| Crate | Lines | Own third-party deps | Consumers | Earns its place by | Verdict |
| --- | --- | --- | --- | --- | --- |
| `tmark-markdown` | 71 800 (vendored) | `unicode-id` | `tmark-syntax` | edition 2018, own lint allows, upstream test suite, ADR 0002 boundary; dominant compile cost | keep |
| `tmark-ir` | 3 600 | `serde`, `serde_json`, `schemars`, `serde_yaml_ng` | everything | the SSOT; changes here rebuild all, which is correct | keep |
| `tmark-syntax` | 2 500 | `serde_yaml_ng` | registry, facade | keeps `tmark-markdown` out of `tmark-fmt`'s and `tmark-ir`'s build | keep |
| `tmark-fmt` | 1 600 | none | writers (B3), facade | ir-only dependency set; testable without the parser as a library | keep |
| `tmark-registry` | 1 100 | `biblatex`, `schemars` | lint, writers, facade | distinct dependency (`biblatex`), the `Loader` seam | keep |
| `tmark-lint` | 400 | none | facade | not by dependencies (same as registry's) nor by compile cost; by AGENTS.md principle 3 naming it as the home of rules, and by keeping `tmark-registry` single-purpose | keep; merging into `tmark-registry` saves one `Cargo.toml` and would make the rule catalogue a module of the resolver — no concrete benefit |
| `tmark-writers` | 3 | none yet | facade | will hold the largest non-vendored code and the `Writer` seam; M4 | keep |
| `tmark` | 250 | `toml` | four edges | the crates.io entry point and the composition layer | keep |
| `tmark-cli` | 316 | `clap` | users | distinct deps and binary | keep |
| `tmark-lsp` | 1 300 | `lsp-server`, `lsp-types`, `crossbeam-channel` | editor | distinct deps and binary; `typst` at M4 | keep |
| `tmark-py` | 3 | `pyo3` at M5 | TeXSmith | will need Python headers to build; keeping it in `members` will make `cargo build --workspace` fail on machines without them once `pyo3` lands | keep for now; at M5 put it outside `default-members` and build it with `maturin` only |
| `tmark-wasm` | 3 | `wasm-bindgen` at M5 | web extension | same reasoning; `cdylib` + `rlib` today costs nothing | keep; A1 must make its `fs` off |

No merge has a concrete benefit today. The two candidates with the weakest
case (`tmark-lint` into `tmark-registry`, and the two three-line bindings)
cost nothing to keep and would cost a rename of every `tmark_lint::` path,
the AGENTS.md sentence, `05-diagnostics.md` and the review mandates to
merge. Not recommended.

### C6. Traits (question 6)

See A6: `Loader` (2 impls) and `Rule` (5) are the only cross-crate traits;
`Writer` is pending. One single-type trait in an example (`dump.rs:30`).

### C7. SSOT (question 7)

Roles, node words, prefixes, admonitions, features and deprecations are
defined once in `crates/tmark-ir/src/registry.rs` and consumed from there
by `tmark-syntax` (`lower/mod.rs:67,108,112`, `lower/inline.rs:449`,
`lower/head.rs:216`, `lower/block.rs:426`), `tmark-fmt` (`block.rs:132`)
and `tmark-registry` (`counters.rs:107,126`, `collect.rs:226`). The vendored
tokenizer holds no name table: `tmark-syntax` passes the prefix and
admonition lists in (`lower/mod.rs:108-115`), which is the right direction.

Exceptions: the caption/prefix/label mapping (A3) and the grammar script
(A4). Two mirror-image `match` blocks map role names to `Inline` variants
(`crates/tmark-syntax/src/lower/inline.rs:571-591`) and back
(`crates/tmark-fmt/src/inline.rs:79-84`); `Role.node` in the registry
carries the variant name as a string, so they cannot be derived without a
macro. Accept: a `match` per direction is what AGENTS.md principle 6 allows,
and a fixture per role pins them.

### C8. Generated artifacts (question 8)

Current (B6). Missing generator for the inventory schema (B4).

## D. Recommendation for M3

### D1. What `tmark-lsp` may depend on

Workspace crates: `tmark` only. This holds today (`cargo tree -p tmark-lsp
--depth 1`: `crossbeam-channel`, `lsp-server`, `lsp-types`, `serde`,
`serde_json`, `tmark`). Third-party: `lsp-server`, `lsp-types`,
`crossbeam-channel` (owned by `lsp-server` anyway), `serde`, `serde_json`;
`typst` and `typst-ide` at M4 (ADR 0005). Not `tokio`, not `tower-lsp`
(ADR 0007), not `regex`, not any `tmark-*` crate directly. When a feature
needs something the facade lacks (sub-spans of reference keys, an
`analyse` function, `parse_with(profile)`, `Config`), the fix is in the
facade or in `tmark-ir`, never an import of `tmark-registry` or
`tmark-fmt` from the server. Add this list to `08-lsp.md` §Shape so the
next feature does not argue it.

### D2. Does `tmark.toml` belong in the facade?

Yes, with one condition. The type and `Config::parse(text, dir)` are pure,
have two users now (CLI, LSP) and a third at M5 (`tmark-py`, so TeXSmith
reads the same file), and depend on `Profile`, `LintConfig`, `Code` and
`ResolveOptions` from four crates — exactly what the facade is for. No new
crate. `Config::discover` reads the file system; accept it as the second
`fs`-gated deviation and write it down in `01-architecture.md` next to
`FsLoader`, after A1 makes the gate real. Both edges already call it
(`crates/tmark-cli/src/main.rs:106`, `crates/tmark-lsp/src/lib.rs:248`). Make `Config::parse_profile`/`tmark::parse_with` the one place that
turns a profile into a parser.

### D3. Order of the fixes

Before M3 work touches the crates: A1 (feature gate), A2's documentation,
A5 (comments), B1 and B2 (documents). During the LSP work: C3's `analyse`
and `parse_with`, A3 (the caption mapping, because completion and outline
both want the label word), A6 (severity words, because `tmark.toml` parses
them). At M3's grammar step: A4 (tables exported from `tmark-ir`). At M4:
C4 (no CommonMark writer) and B4 (inventory schema, which TeXSmith needs
before it writes inventories).
