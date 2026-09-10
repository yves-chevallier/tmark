#!/usr/bin/env python3
"""Regenerate the `## ir` block of conformance fixtures from the parser.

    cargo build -p tmark-syntax --example dump
    python3 scripts/fixture-ir.py spec/conformance/*.md

The output is the parser's opinion, normalised by `tmark_ir::structural_json`
(ids, spans, sugar fields and defaults dropped; the one definition, shared
with the conformance runner). REVIEW EVERY DIFF before
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
def main(paths):
    if not DUMP.exists():
        sys.exit("build the dump example first: cargo build -p tmark-syntax --example dump")
    for path in map(pathlib.Path, paths):
        text = path.read_text()
        canonical = re.search(r"## canonical\n\n(`{3,})md\n(.*?)\1\n", text, re.S)
        ir = re.search(r"## ir\n\n```json\n(.*?)```\n", text, re.S)
        if not canonical or not ir:
            print(f"{path.name}: no canonical/ir section, skipped")
            continue
        with tempfile.NamedTemporaryFile("w", suffix=".md", delete=False) as tmp:
            tmp.write(canonical.group(2))
        result = subprocess.run(
            [str(DUMP), "--structural", tmp.name], capture_output=True, text=True, check=True
        )
        document = json.loads(result.stdout)
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
