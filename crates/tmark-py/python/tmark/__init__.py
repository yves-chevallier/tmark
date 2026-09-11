"""TMark for Python: the facade of the Rust core, JSON-shaped.

Every function of :mod:`tmark._tmark` is re-exported here; documents cross
the boundary as plain ``dict``/``list`` values in the shape of
``schema("ir")`` (ADR 0003). ``design/09-bindings.md`` is the reference.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from tmark._tmark import (
    Resolved,
    __version__,
    codes,
    edit,
    edit_many,
    fixes,
    format,
    fragments,
    lint,
    lower_web,
    parse,
    registries,
    resolve,
    schema,
    schema_hash,
    version,
    write,
)


@runtime_checkable
class Loader(Protocol):
    """What ``lint``, ``fixes``, ``resolve``, ``write`` and ``lower_web`` load files through.

    ``from_path`` is the file the request comes from (the document, or an
    ``{include base=...}`` directory); ``rel`` is the path as written. Return
    the text, or ``None`` when there is no such file: the core reports
    ``include-missing`` and friends. Resolve ``rel`` as
    ``tmark_registry::loader::join`` does: an absolute ``rel`` as is,
    otherwise against the directory of ``from_path`` (``from_path`` itself
    when it has no extension), normalised textually.
    """

    def load(self, from_path: str, rel: str) -> str | None: ...


__all__ = [
    "Loader",
    "Resolved",
    "__version__",
    "codes",
    "edit",
    "edit_many",
    "fixes",
    "format",
    "fragments",
    "lint",
    "lower_web",
    "parse",
    "registries",
    "resolve",
    "schema",
    "schema_hash",
    "version",
    "write",
]
