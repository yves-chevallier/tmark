"""resolve and lint over the conformance fixtures, with Python loaders."""

from __future__ import annotations

import pytest

import tmark


class MemoryLoader:
    """The Python twin of tmark_registry::MemoryLoader, recording requests."""

    def __init__(self, files: dict[str, str]):
        self.files = files
        self.requests: list[tuple[str, str]] = []

    def load(self, from_path: str, rel: str) -> str | None:
        self.requests.append((from_path, rel))
        return self.files.get(rel)


def test_resolve_counter_item(fixture):
    text = fixture("counter-item")
    doc = tmark.parse(text, file="counter-item.md")
    res = tmark.resolve(doc, MemoryLoader({}), {"path": "counter-item.md"}, text=text)
    assert res["counters"]["fw"]["name"] == "Finding"
    assert res["counters"]["fw"]["format"] == "FW-{n:02d}"
    assert res["counters"]["fw"]["numbers"] == {"boot-loop": 1}
    assert res["counters"]["fw"]["next"] == 2
    assert res["counters"]["fw"]["tmark_numbered"] is True
    assert res["next_start"] == {"fw": 2}
    assert "fig" in res["counters"] and "fig" not in res["next_start"]
    (label,) = res["labels"]
    assert label["id"] == "fw:boot-loop"
    assert label["prefix"] == "fw"
    assert label["host"] == "counter_item"
    assert label["number"] == 1
    assert label["formatted"] == "FW-01"
    assert res["diagnostics"] == []


def test_resolve_continues_a_series_from_start(fixture):
    text = fixture("counter-item")
    doc = tmark.parse(text)
    res = tmark.resolve(doc, None, {"start": {"fw": 7}})
    assert res["labels"][0]["formatted"] == "FW-07"
    assert res["next_start"] == {"fw": 8}


def test_resolve_references(fixture):
    text = fixture("reference-bare")
    doc = tmark.parse(text, file="reference-bare.md")
    res = tmark.resolve(doc, MemoryLoader({}), {"path": "reference-bare.md"}, text=text)
    keys = [(r["key"], r["resolution"]["kind"]) for r in res["refs"]]
    assert keys == [("sec:intro", "unresolved"), ("ein05", "unresolved")]
    assert [(d["code"], d["line"], d["col"]) for d in res["diagnostics"]] == [
        ("ref-unresolved", 1, 6),
        ("ref-unresolved", 1, 21),
    ]
    assert res["bibliography"] == []


def test_resolve_bracketed_and_doi(fixture):
    doc = tmark.parse(fixture("reference-bracketed"))
    res = tmark.resolve(doc)
    assert [r["key"] for r in res["refs"]] == ["ein05", "AI2027"]
    doc = tmark.parse(fixture("reference-doi"))
    res = tmark.resolve(doc)
    assert {r["resolution"]["kind"] for r in res["refs"]} == {"doi"}
    assert res["dois"] == ["10.1002/andp.19053221004"]
    assert res["diagnostics"] == []


def test_resolve_with_an_inline_bibliography():
    text = (
        "---\npress:\n  sources:\n    bibliography:\n"
        "      ein05: https://doi.org/10.1002/andp.19053221004\n"
        "      knuth: {type: book, title: The Art}\n---\n\nSee @ein05 and @knuth.\n"
    )
    res = tmark.resolve(tmark.parse(text))
    assert res["bibliography"] == ["ein05", "knuth"]
    assert res["entries"]["knuth"]["entry_type"] == "book"
    assert res["dois"] == ["https://doi.org/10.1002/andp.19053221004"]
    assert [r["resolution"] for r in res["refs"]] == [
        {"kind": "citation", "key": "ein05"},
        {"kind": "citation", "key": "knuth"},
    ]


def test_python_loader_receives_the_includes(fixture):
    text = fixture("include")
    doc = tmark.parse(text, file="docs/include.md")
    loader = MemoryLoader({"chapters/boot.md": "# Boot {#sec:boot}\n"})
    res = tmark.resolve(doc, loader, {"path": "docs/include.md"}, text=text)
    assert loader.requests == [
        ("docs/include.md", "chapters/boot.md"),
        ("docs/chapters", "chapters/setup.md"),
    ]
    assert res["included"] == [{"id": 1, "path": "docs/chapters/boot.md"}]
    assert [lbl["id"] for lbl in res["labels"]] == ["sec:boot"]
    assert res["labels"][0]["span"][0] == 1
    missing = [d for d in res["diagnostics"] if d["code"] == "include-missing"]
    assert len(missing) == 1
    assert missing[0]["line"] == 3 and missing[0]["path"] == "docs/include.md"
    assert "setup.md" in missing[0]["message"]


def test_fs_loader_is_the_default(tmp_path):
    (tmp_path / "part.md").write_text("Part {#sec:part}\n=====\n")
    (tmp_path / "main.md").write_text("{include}(part.md)\n\nSee @sec:part.\n")
    text = (tmp_path / "main.md").read_text()
    doc = tmark.parse(text)
    res = tmark.resolve(doc, None, {"path": str(tmp_path / "main.md")})
    assert res["refs"][0]["resolution"]["kind"] == "label"
    assert res["diagnostics"] == []
    assert tmark.lint(text, file=str(tmp_path / "main.md")) == []


def test_loader_exceptions_propagate():
    class Broken:
        def load(self, from_path, rel):
            raise RuntimeError("disk on fire")

    doc = tmark.parse("{include}(x.md)\n")
    with pytest.raises(RuntimeError, match="disk on fire"):
        tmark.resolve(doc, Broken())
    with pytest.raises(TypeError, match="load"):
        tmark.resolve(doc, object())


def test_lint_collects_every_stage(fixture_inputs):
    text = fixture_inputs("include")[0]
    diagnostics = tmark.lint(text, loader=MemoryLoader({}))
    stages = {d["code"]: d["stage"] for d in diagnostics}
    assert stages["deprecated"] == "parse"
    assert stages["include-missing"] == "resolve"
    fix = next(d for d in diagnostics if d["code"] == "deprecated")["fix"]
    assert fix["replacement"] == "{include}(chapters/boot.md)"
    # Levels re-level lint rules; parse and resolve diagnostics are facts.
    skip = "# A\n\n### B\n"
    assert [(d["code"], d["severity"]) for d in tmark.lint(skip)] == [("heading-skip", "hint")]
    assert tmark.lint(skip, options={"levels": {"heading-skip": "error"}})[0]["severity"] == "error"
    assert tmark.lint(skip, options={"levels": {"heading-skip": "off"}}) == []
    with pytest.raises(ValueError, match="unknown diagnostic code"):
        tmark.lint(text, options={"levels": {"nope": "off"}})
    with pytest.raises(TypeError, match="options"):
        tmark.lint(text, options={"colour": "red"})


def test_resolve_refuses_another_major(fixture):
    doc = tmark.parse(fixture("counter-item"))
    doc["tmark"] = "99.0.0"
    with pytest.raises(ValueError, match="99.0.0"):
        tmark.resolve(doc)
    del doc["tmark"]
    assert tmark.resolve(doc)["next_start"] == {"fw": 2}


def test_resolve_numbers_a_site_and_links_siblings(fixture):
    """Design 06 §Site-wide resolution: the MkDocs plugin's two passes."""
    b_text = fixture("counter-item") + "\n![x](a.png)\n\nFigure: Crash. {#fig:crash}\n"
    b = tmark.resolve(tmark.parse(b_text), None, {"path": "b.md", "numbering": "all", "start": {"fig": 3}})
    assert b["numbering"] == "all" and b["next_start"]["fig"] == 4
    assert b["counters"]["fig"]["tmark_numbered"] is True
    boot, crash = b["book"]
    assert crash == {
        "key": "fig:crash",
        "prefix": "fig",
        "number": "3",
        "kind": "figure",
        "title": "Crash.",
        "location": "b.md#fig:crash",
    }
    assert boot["number"] == "FW-01" and boot["location"] == "b.md#fw:boot-loop"
    a = tmark.resolve(tmark.parse("See @fig:crash and @fw:boot-loop.\n"), None, {"book": b["book"], "lang": "de"})
    assert [r["resolution"] for r in a["refs"]] == [
        {"kind": "sibling", "label": "3", "location": "b.md#fig:crash"},
        {"kind": "sibling", "label": "FW-01", "location": "b.md#fw:boot-loop"},
    ]
    assert a["lang"] == "de" and a["counters"]["fig"]["name"] == "Abbildung"
    assert a["diagnostics"] == []
    with pytest.raises(TypeError, match="options"):
        tmark.resolve(tmark.parse("x"), None, {"numbering": "chapter"})
