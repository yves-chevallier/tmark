# ADR 0003 — The IR is defined once, in Rust, and exported as a schema

**Status:** accepted.

**Context.** TeXSmith defines the IR in Python (`texsmith.ir.nodes`). A Rust
core needs the same tree. Two definitions drift.

**Decision.** `tmark-ir` is the definition. `serde` gives JSON; `schemars`
gives a JSON Schema committed under `crates/tmark-ir/schema/`. TeXSmith
generates its Python models from that schema. The PyO3 binding exchanges
documents as JSON-shaped Python objects, not as PyO3 classes mirroring every
node.

**Consequences.** One place to add a node. The Python side validates at the
edge and is otherwise untyped Rust data. If JSON marshalling ever shows in a
profile, switch the encoding, not the ownership.
