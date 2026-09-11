"""Shared fixtures: the conformance corpus of the repository."""

from __future__ import annotations

import pathlib
import re

import pytest

ROOT = pathlib.Path(__file__).resolve().parents[3]
CONFORMANCE = ROOT / "spec" / "conformance"


def canonical(name: str) -> str:
    """The `## canonical` block of a conformance fixture."""
    text = (CONFORMANCE / f"{name}.md").read_text()
    match = re.search(r"## canonical\n\n(`{3,})md\n(.*?)\1\n", text, re.S)
    assert match, f"{name}: no canonical block"
    return match.group(2)


def inputs(name: str) -> list[str]:
    """Every `md` block of the `## input` section of a fixture."""
    text = (CONFORMANCE / f"{name}.md").read_text()
    section = re.search(r"## input\n\n(.*?)\n## ", text, re.S)
    assert section, f"{name}: no input section"
    return [m.group(2) for m in re.finditer(r"(`{3,})md\n(.*?)\1\n", section.group(1), re.S)]


@pytest.fixture
def fixture():
    return canonical


@pytest.fixture
def fixture_inputs():
    return inputs
