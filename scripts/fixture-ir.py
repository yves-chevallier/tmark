#!/usr/bin/env python3
"""Regenerate the `## ir` block of conformance fixtures from the parser.

    cargo build -p tmark-syntax --example dump
    python3 scripts/fixture-ir.py spec/conformance/*.md

The output is the parser's opinion, normalised like the conformance runner
(ids, spans, sugar fields and defaults dropped). REVIEW EVERY DIFF before
committing: a fixture records the intended IR, not whatever the parser
produced. Diagnostics of the canonical input are printed for information.
"""
import json
import pathlib
import re
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
DUMP = ROOT / "target/debug/examples/dump"
SUGAR_FIELDS = {"position", "bracketed"}


def normalise(value):
    if isinstance(value, dict):
        node = "span" in value
        out = {}
        for key, item in value.items():
            if key == "span" or (node and key == "id") or key in SUGAR_FIELDS:
                continue
            item = normalise(item)
            if item is None or (not isinstance(item, bool) and item in ([], {}, "")):
                continue
            out[key] = item
        return out
    if isinstance(value, list):
        return [normalise(item) for item in value]
    return value


def main(paths):
    if not DUMP.exists():
        sys.exit("build the dump example first: cargo build -p tmark-syntax --example dump")
    for path in map(pathlib.Path, paths):
        text = path.read_text()
        canonical = re.search(r"## canonical\n\n```md\n(.*?)```\n", text, re.S)
        ir = re.search(r"## ir\n\n```json\n(.*?)```\n", text, re.S)
        if not canonical or not ir:
            print(f"{path.name}: no canonical/ir section, skipped")
            continue
        with tempfile.NamedTemporaryFile("w", suffix=".md", delete=False) as tmp:
            tmp.write(canonical.group(1))
        result = subprocess.run([str(DUMP), tmp.name], capture_output=True, text=True, check=True)
        document = normalise(json.loads(result.stdout))
        document.pop("file", None)
        new = json.dumps(document, indent=2, ensure_ascii=False) + "\n"
        if new != ir.group(1):
            path.write_text(text[: ir.start(1)] + new + text[ir.end(1) :])
            print(f"{path.name}: ir block updated")
        else:
            print(f"{path.name}: unchanged")
        if result.stderr.strip():
            print(f"  diagnostics on the canonical input:\n  " + result.stderr.strip().replace("\n", "\n  "))


if __name__ == "__main__":
    main(sys.argv[1:] or sorted((ROOT / "spec/conformance").glob("*.md")))
