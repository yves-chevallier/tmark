# tmark

Python binding of the TMark core: `parse`, `format`, `lint`, `fixes`,
`resolve`, `edit`, `schema`, the diagnostic catalogue and the closed
registries, all JSON-shaped. Design: `design/09-bindings.md` of the
repository. Development install from the workspace:

    uv pip install maturin
    maturin develop -m crates/tmark-py/Cargo.toml
