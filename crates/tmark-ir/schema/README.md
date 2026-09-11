Generated JSON schemas (`cargo run -p tmark-ir --example schema`): `ir.json`
(the `Document` tree), `frontmatter.json` (the typed `Keys`) and
`diagnostic.json` (one `Diagnostic`). Committed and checked in CI; regenerate
after any change to the types. The cross-document inventory schema
(`inventory.json`) belongs to `tmark-registry` (milestone 2); the resolution
schema (`resolved`) is served by the facade (`tmark::schema`) and the bindings.
