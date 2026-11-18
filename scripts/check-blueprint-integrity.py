#!/usr/bin/env python3
"""Fail CI when the project blueprint is truncated or replaced by tool output."""
from collections import Counter
from pathlib import Path
import re
import sys

arguments = sys.argv[1:]
baseline_path = None
if "--baseline" in arguments:
    index = arguments.index("--baseline")
    if index + 1 >= len(arguments):
        print("Blueprint integrity failure: --baseline requires a path", file=sys.stderr)
        raise SystemExit(2)
    baseline_path = Path(arguments[index + 1])
    del arguments[index : index + 2]
if len(arguments) > 1:
    print("usage: check-blueprint-integrity.py [blueprint] [--baseline path]", file=sys.stderr)
    raise SystemExit(2)
path = Path(arguments[0]) if arguments else Path(__file__).resolve().parents[1] / "LIA-Project-Blueprint.md"
try:
    raw = path.read_bytes()
    text = raw.decode("utf-8")
except (OSError, UnicodeDecodeError) as error:
    print(f"Blueprint integrity failure: {error}", file=sys.stderr)
    raise SystemExit(1)

markers = ("Warning: truncated output", "tokens truncated", "Total output lines:")
found = [marker for marker in markers if marker in text]
lines = text.splitlines()
heading_pattern = re.compile(r"^ {0,3}(#{1,6})(?:[ \t]+|$)(.*?)\s*$")


def heading_inventory(source: str) -> Counter[str]:
    inventory: Counter[str] = Counter()
    for line in source.splitlines():
        match = heading_pattern.match(line)
        if match:
            title = re.sub(r"[ \t]+#+[ \t]*$", "", match.group(2)).strip()
            title = " ".join(title.split())
            inventory[f"{'#' * len(match.group(1))} {title}".rstrip()] += 1
    return inventory


headings = heading_inventory(text)
heading_count = sum(headings.values())
failures = []
if found:
    failures.append(f"truncation markers present: {', '.join(found)}")
if len(raw) < 600_000:
    failures.append(f"size {len(raw)} bytes is below 600000")
if len(lines) < 8_000:
    failures.append(f"line count {len(lines)} is below 8000")
if heading_count < 650:
    failures.append(f"heading count {heading_count} is below 650")
if baseline_path is not None:
    try:
        baseline_text = baseline_path.read_bytes().decode("utf-8")
    except (OSError, UnicodeDecodeError) as error:
        print(f"Blueprint integrity failure: cannot read baseline: {error}", file=sys.stderr)
        raise SystemExit(1)
    missing = heading_inventory(baseline_text) - headings
    if missing:
        examples = ", ".join(
            f"{heading} ({count})" for heading, count in sorted(missing.items())[:10]
        )
        failures.append(
            f"{sum(missing.values())} baseline heading occurrence(s) missing: {examples}"
        )
if failures:
    print("Blueprint integrity failure: " + "; ".join(failures), file=sys.stderr)
    raise SystemExit(1)
print(f"Blueprint integrity OK: {len(raw)} bytes, {len(lines)} lines, {heading_count} headings")
