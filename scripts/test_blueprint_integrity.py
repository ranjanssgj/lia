#!/usr/bin/env python3
"""Regression checks for Blueprint truncation detection."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

CHECK = Path(__file__).with_name("check-blueprint-integrity.py")


class BlueprintIntegrityChecks(unittest.TestCase):
    def run_check(self, text: str) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as directory:
            candidate = Path(directory) / "blueprint.md"
            candidate.write_text(text, encoding="utf-8")
            return subprocess.run(
                [sys.executable, str(CHECK), str(candidate)],
                capture_output=True,
                text=True,
                check=False,
            )

    def test_rejects_truncation_marker(self) -> None:
        result = self.run_check("Warning: truncated output\n" + "# Heading\n" * 700 + "x" * 610_000)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("truncation markers present", result.stderr)

    def test_rejects_insufficient_size_and_heading_count(self) -> None:
        result = self.run_check("# Heading\n" * 10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("below 600000", result.stderr)
        self.assertIn("below 650", result.stderr)

    def test_accepts_document_over_all_thresholds(self) -> None:
        content = "# Heading\n" * 8_000 + "x" * 610_000
        result = self.run_check(content)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
