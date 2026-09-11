"""schema, schema_hash, codes, registries, fragments, write, stubs."""

from __future__ import annotations

import pathlib
import subprocess
import sys

import pytest

import tmark

HERE = pathlib.Path(__file__).resolve().parent


def test_schemas_exist():
    ir = tmark.schema("ir")
    assert ir["title"] == "Document"
    assert "Block" in ir["definitions"]
    assert tmark.schema("frontmatter")["title"] == "Keys"
    assert tmark.schema("diagnostic")["title"] == "Diagnostic"
    resolved = tmark.schema("resolved")
    assert resolved["title"] == "ResolvedView"
    assert set(resolved["required"]) >= {"counters", "next_start", "labels", "refs", "diagnostics"}
    with pytest.raises(ValueError, match="unknown schema"):
        tmark.schema("nope")


def test_schema_hash_is_stable():
    first = tmark.schema_hash()
    assert len(first) == 16 and int(first, 16) >= 0
    assert first == tmark.schema_hash()
    # The same hash from a fresh interpreter: it depends on the schema alone.
    other = subprocess.run(
        [sys.executable, "-c", "import tmark; print(tmark.schema_hash())"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    assert other == first


def test_codes_catalogue():
    codes = tmark.codes()
    ids = [c["id"] for c in codes]
    assert ids[0] == "attr-no-host" and "ref-unresolved" in ids and "heading-skip" in ids
    assert len(ids) == len(set(ids))
    by_id = {c["id"]: c for c in codes}
    assert by_id["ref-unresolved"] == {
        "id": "ref-unresolved",
        "severity": "warning",
        "stage": "resolve",
        "doc": "Spec §Ref: a key found in no registry",
    }
    assert all(c["doc"] for c in codes)
    assert {c["stage"] for c in codes} == {"parse", "resolve", "lint"}


def test_registries_are_the_closed_tables():
    reg = tmark.registries()
    roles = {r["name"]: r for r in reg["roles"]}
    assert roles["code"]["principal"] == "lang"
    assert roles["margin"]["replaced_by"] == "aside"
    assert {w["word"] for w in reg["node_words"]} == {"code", "table", "table-config", "image", "raw"}
    assert reg["lang_default_node_words"] == [{"lang": "mermaid", "word": "image"}]
    prefixes = {p["name"]: p for p in reg["prefixes"]}
    assert prefixes["fig"]["label"] == "Figure" and prefixes["sec"]["heading"] is True
    assert {a["name"] for a in reg["admonitions"]} >= {"note", "theorem", "proof"}
    assert {f["name"] for f in reg["features"]} >= {"paragraph.lead", "compat.pymdownx"}
    assert any(d["id"] == "margin-role" for d in reg["deprecations"])
    labels = {k["name"]: k["label"] for k in reg["key_labels"]}
    assert labels["ctrl"] == "Ctrl" and labels["shift"].endswith("Shift")


def test_fragments_are_the_contract_table():
    fragments = tmark.fragments()
    by_name = {f["name"]: f for f in fragments}
    assert "ts-typesetting" in by_name
    row = by_name["ts-typesetting"]
    assert set(row) == {"name", "provides", "packages", "shell_escape", "description"}
    assert "\\tslead" in row["provides"] and "xcolor" in row["packages"]
    assert isinstance(row["shell_escape"], bool) and row["description"]
    assert len({f["name"] for f in fragments}) == len(fragments)


def test_write_is_milestone_4():
    with pytest.raises(NotImplementedError, match="milestone 4"):
        tmark.write(tmark.parse("x"), "latex", {})


def test_loader_protocol():
    class L:
        def load(self, from_path: str, rel: str) -> str | None:
            return None

    assert isinstance(L(), tmark.Loader)
    assert not isinstance(object(), tmark.Loader)


def test_stub_is_up_to_date():
    result = subprocess.run(
        [sys.executable, str(HERE.parent / "scripts" / "gen_stubs.py"), "--check"],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
