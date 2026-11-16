#!/usr/bin/env python3
"""Fail CI when the project blueprint is truncated or replaced by tool output."""
from pathlib import Path
import re
import sys

path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[1] / "LIA-Project-Blueprint.md"
try:
    raw = path.read_bytes()
    text = raw.decode("utf-8")
except (OSError, UnicodeDecodeError) as error:
    print(f"Blueprint integrity failure: {error}", file=sys.stderr)
    raise SystemExit(1)

markers = ("Warning: truncated output", "tokens truncated", "Total output lines:")
found = [marker for marker in markers if marker in text]
lines = text.splitlines()
heading_count = sum(bool(re.match(r"^#{1,6}\s+", line)) for line in lines)
failures = []
if found:
    failures.append(f"truncation markers present: {', '.join(found)}")
if len(raw) < 600_000:
    failures.append(f"size {len(raw)} bytes is below 600000")
if len(lines) < 8_000:
    failures.append(f"line count {len(lines)} is below 8000")
if heading_count < 650:
    failures.append(f"heading count {heading_count} is below 650")
if failures:
    print("Blueprint integrity failure: " + "; ".join(failures), file=sys.stderr)
    raise SystemExit(1)
print(f"Blueprint integrity OK: {len(raw)} bytes, {len(lines)} lines, {heading_count} headings")
