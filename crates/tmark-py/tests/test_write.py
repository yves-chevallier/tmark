"""write: bodies for the three backends against one resolution."""

from __future__ import annotations

import pytest

import tmark

TEXT = """---
press:
  declare:
    counters:
      fw: {name: Finding, format: "FW-{n:02d}"}
---

# Findings {#sec:findings}

#(fw:boot) The firmware reboots when the watchdog fires.

See @fw:boot, @sec:findings and {keys}[ctrl+s].

```python
print("hi")
```

## Later {#sec:later}

#(fw:brownout) A brown-out resets the clock. Compare @fw:boot.
"""


@pytest.fixture
def parsed():
    return tmark.parse(TEXT, file="findings.md")


@pytest.mark.parametrize("backend", ["latex", "typst", "html"])
def test_write_returns_a_body(parsed, backend):
    body = tmark.write(parsed, backend, {"source_map": True})
    assert set(body) == {"text", "map", "requires"}
    assert "FW-01" in body["text"] and "FW-02" in body["text"]
    assert "Findings" in body["text"]
    req = body["requires"]
    assert set(req) >= {"packages", "fragments", "shell_escape", "assets", "bibliography", "citations", "acronyms", "index", "counters"}
    assert req["counters"] == ["fw"]
    assert isinstance(req["shell_escape"], bool)
    assert body["map"], "source_map on fills the map"
    for start, end, node in body["map"]:
        assert 0 <= start <= end <= len(body["text"].encode())
        assert isinstance(node, int)
    assert tmark.write(parsed, backend)["map"] == []


def test_latex_requires_fragments_from_the_table(parsed):
    body = tmark.write(parsed, "latex")
    names = {f["name"] for f in tmark.fragments()}
    assert body["requires"]["fragments"], "a counter item needs a contract"
    assert set(body["requires"]["fragments"]) <= names
    assert "\\" in body["text"]


def test_resolved_handle_is_shared_across_slots(parsed):
    res = tmark.resolve(parsed, None, {"path": "findings.md"})
    handle = res["handle"]
    assert isinstance(handle, tmark.Resolved)
    assert repr(handle) == "<tmark.Resolved: 4 labels, 3 refs, 0 diagnostics>"
    assert handle.view()["next_start"] == {"fw": 3}
    # Two slot documents over halves of the block list, one resolution.
    blocks = parsed["blocks"]
    cut = next(i for i, b in enumerate(blocks) if b["type"] == "Header" and b["level"] == 2)
    first = dict(parsed, blocks=blocks[:cut])
    second = dict(parsed, blocks=blocks[cut:])
    for resolved in (res, handle):
        one = tmark.write(first, "html", None, None, resolved)["text"]
        two = tmark.write(second, "html", None, None, resolved)["text"]
        assert "FW-01" in one and "FW-02" not in one
        assert "FW-02" in two and "FW-01" in two  # the back reference keeps its number
    # Without the handle each slot resolves alone and numbering restarts.
    alone = tmark.write(second, "html")["text"]
    assert "FW-01" in alone and "FW-02" not in alone
    # `start` through resolve_options continues a series instead.
    cont = tmark.write(second, "html", resolve_options={"start": {"fw": 2}})["text"]
    assert "FW-02" in cont


def test_write_validates_options(parsed):
    body = tmark.write(parsed, "latex", {"media": "web", "headings": {"base_level": 0, "numbered": False}, "code": {"engine": "minted"}, "numbering": {"fw": "tmark"}})
    assert body["requires"]["shell_escape"] is True
    with pytest.raises(TypeError, match="unknown key `colour`"):
        tmark.write(parsed, "latex", {"colour": "red"})
    with pytest.raises(TypeError, match="unknown key `headings.depth`"):
        tmark.write(parsed, "latex", {"headings": {"depth": 2}})
    with pytest.raises(ValueError, match="options"):
        tmark.write(parsed, "latex", {"media": "paper"})
    with pytest.raises(ValueError, match="unknown backend"):
        tmark.write(parsed, "docx")
    with pytest.raises(TypeError, match="resolved"):
        tmark.write(parsed, "latex", resolved={"labels": []})


CITED = """---
press:
  sources:
    bibliography:
      ein05: {type: article, author: "Einstein, Albert", year: 1905, title: Zur Elektrodynamik}
---

Cite @ein05, @[ein05] and @[+ein05].
"""


def test_citations_option_beats_the_front_matter():
    # Spec §Cite (C51): a bare `@key` is the short citation unless the
    # document, or this option, says narrative; `@[key]` and `+key` read
    # as written either way.
    doc = tmark.parse(CITED)
    assert "Cite \\cite{ein05}, \\cite{ein05} and \\textcite{ein05}." in tmark.write(doc, "latex")["text"]
    narrative = tmark.write(doc, "latex", {"citations": {"narrative": True}})["text"]
    assert "Cite \\textcite{ein05}, \\cite{ein05} and \\textcite{ein05}." in narrative
    switched = tmark.parse(CITED.replace("press:\n", "press:\n  features: {citations.narrative: true}\n"))
    assert "\\textcite{ein05}, \\cite{ein05}" in tmark.write(switched, "latex")["text"]
    assert "\\cite{ein05}, \\cite{ein05}" in tmark.write(switched, "latex", {"citations": {"narrative": False}})["text"]
    assert 'form: "prose"' in tmark.write(doc, "typst", {"citations": {"narrative": True}})["text"]
    with pytest.raises(TypeError, match="unknown key `citations.style`"):
        tmark.write(doc, "latex", {"citations": {"style": "short"}})


def test_write_loads_includes_through_the_loader():
    class L:
        def __init__(self):
            self.seen = []

        def load(self, from_path, rel):
            self.seen.append(rel)
            return "Part {#sec:part}\n=====\n" if rel == "part.md" else None

    loader = L()
    doc = tmark.parse("{include}(part.md)\n\nSee @sec:part.\n")
    body = tmark.write(doc, "html", loader=loader)
    assert loader.seen == ["part.md"]
    assert "?" not in body["text"].split("See")[1][:20]
