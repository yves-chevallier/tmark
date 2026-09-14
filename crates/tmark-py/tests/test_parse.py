"""parse, format, edit, fixes: the syntactic half of the API."""

from __future__ import annotations

import pytest

import tmark


def test_parse_returns_versioned_json(fixture):
    doc = tmark.parse(fixture("reference-bare"), file="reference-bare.md")
    assert list(doc)[0] == "tmark"
    assert doc["tmark"] == tmark.version() == tmark.__version__
    assert doc["file"] == 0
    assert doc["blocks"][0]["type"] == "Para"
    refs = [n for n in doc["blocks"][0]["content"] if n["type"] == "Ref"]
    assert [r["items"][0]["key"] for r in refs] == ["sec:intro", "ein05"]
    assert doc["diagnostics"] == []


def test_parse_file_id_and_spans():
    doc = tmark.parse("Hello *world*\n", file_id=3)
    assert doc["file"] == 3
    emph = doc["blocks"][0]["content"][1]
    assert emph["type"] == "Emph"
    assert emph["span"] == [3, 6, 13]


def test_parse_diagnostics_carry_line_and_col(fixture_inputs):
    text = fixture_inputs("include")[0]
    doc = tmark.parse(text, file="include.md")
    codes = [(d["code"], d["line"], d["col"], d["path"], d["stage"]) for d in doc["diagnostics"]]
    assert codes == [("deprecated", 1, 1, "include.md", "parse")]
    d = doc["diagnostics"][0]
    assert d["severity"] == "warning"
    assert d["span"][0] == 0
    assert "message" in d


def test_parse_profiles():
    assert tmark.parse("x", profile="strict")["blocks"]
    assert tmark.parse("x", profile="mkdocs")["blocks"]
    with pytest.raises(ValueError, match="unknown profile"):
        tmark.parse("x", profile="pandoc")


@pytest.mark.parametrize("name", ["counter-item", "reference-bracketed", "include"])
def test_format_is_idempotent_and_canonical(name, fixture, fixture_inputs):
    canonical = fixture(name)
    assert tmark.format(canonical) == canonical
    for text in fixture_inputs(name):
        assert tmark.format(text) == canonical
    assert tmark.format(canonical, profile="strict") == canonical


def test_edit_splices_one_node():
    text = "Hello *world*, bye.\n"
    doc = tmark.parse(text)
    emph = doc["blocks"][0]["content"][1]
    out = tmark.edit(text, doc, emph["id"], {"type": "Strong", "content": [{"type": "Str", "text": "world"}]})
    assert out == "Hello **world**, bye.\n"
    with pytest.raises(ValueError, match="not in the document"):
        tmark.edit(text, doc, 999, {"type": "Str", "text": "x"})
    with pytest.raises(TypeError, match="replacement"):
        tmark.edit(text, doc, emph["id"], {"type": "Nope"})


def test_edit_many_splices_disjoint_nodes():
    text = "Hello *world*, bye *all*.\n"
    doc = tmark.parse(text)
    content = doc["blocks"][0]["content"]
    emphs = [n for n in content if n["type"] == "Emph"]
    strong = lambda word: {"type": "Strong", "content": [{"type": "Str", "text": word}]}
    out = tmark.edit_many(
        text,
        doc,
        [
            {"node_id": emphs[0]["id"], "replacement": strong("world")},
            {"node_id": emphs[1]["id"], "replacement": strong("all")},
        ],
    )
    assert out == "Hello **world**, bye **all**.\n"
    assert tmark.edit_many(text, doc, []) == text
    para = doc["blocks"][0]["id"]
    with pytest.raises(ValueError, match="overlap"):
        tmark.edit_many(
            text,
            doc,
            [
                {"node_id": para, "replacement": {"type": "Para", "content": []}},
                {"node_id": emphs[0]["id"], "replacement": strong("x")},
            ],
        )
    with pytest.raises(ValueError, match="not in the document"):
        tmark.edit_many(text, doc, [{"node_id": 999, "replacement": strong("x")}])
    with pytest.raises(TypeError, match="node_id"):
        tmark.edit_many(text, doc, [{"replacement": strong("x")}])


def test_fixes_is_what_lint_fix_writes(fixture_inputs, fixture):
    text = fixture_inputs("include")[0]
    fixed = tmark.fixes(text, loader=NoFiles())
    assert fixed == fixture("include")
    assert tmark.fixes("plain\n") == "plain\n"
    # A citation hugging the word before it gets the space `@` needs
    # (the X4 guard); the sugar was the short form, so the fix is `@key`.
    assert tmark.fixes("En sortie[^spru485a] et [^ein05].\n") == "En sortie @spru485a et @ein05.\n"


class NoFiles:
    def load(self, from_path: str, rel: str) -> str | None:
        return None
