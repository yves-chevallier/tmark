# ADR 0006 — `.md` stays the primary extension

**Status:** accepted.

**Context.** A dedicated `.tm` extension would make TMark files first-class
in editors but invisible to GitHub, MkDocs, Zensical and every Markdown tool,
contradicting P3 (graceful degradation).

**Decision.** `.md` is the primary extension. `.tm`, `.tmd` and `.tmark` are
accepted as explicit markers. Editors detect TMark in `.md` by a `press` key
in the front matter, a `tmark.toml` in the workspace, or a user association.

**Consequences.** The VS Code extension injects its grammar into Markdown and
keeps a `tmark` language id for the explicit extensions. Diagnostics are not
shown on `.md` files that are not detected as TMark.
