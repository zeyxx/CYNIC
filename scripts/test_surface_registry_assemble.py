#!/usr/bin/env python3
"""Deterministic adversarial probes for surface-registry-assemble."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ASSEMBLER = ROOT / "scripts" / "surface-registry-assemble"
SCHEMA = "cynic.surface_registry.v1"


class SurfaceRegistryAssemblerSecurityTests(unittest.TestCase):
    def run_assembler(self, manifest: Path, output: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(ASSEMBLER), "--manifest", str(manifest), "--output", str(output)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )

    def fixture(self) -> tuple[tempfile.TemporaryDirectory[str], Path, Path]:
        temporary = tempfile.TemporaryDirectory(prefix="surface-registry-probe-")
        root = Path(temporary.name)
        fragments = root / "fragments"
        fragments.mkdir()
        (fragments / "main.json").write_text(json.dumps({"id": "main", "value": 1}), encoding="utf-8")
        manifest = root / "manifest.json"
        manifest.write_text(
            json.dumps({"schema": SCHEMA, "surfaces": [{"id": "main", "path": "fragments/main.json"}]}),
            encoding="utf-8",
        )
        return temporary, manifest, root / "assembled.json"

    def assert_rejected(self, manifest: Path, output: Path, text: str) -> None:
        result = self.run_assembler(manifest, output)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(text, result.stderr)

    def test_positive_fixture_still_assembles(self) -> None:
        temporary, manifest, output = self.fixture()
        with temporary:
            result = self.run_assembler(manifest, output)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(output.read_text(encoding="utf-8"))["surfaces"][0]["id"], "main")

    def test_symlink_escape_is_rejected(self) -> None:
        temporary, manifest, output = self.fixture()
        with temporary:
            outside = Path(temporary.name) / "outside.json"
            outside.write_text(json.dumps({"id": "main"}), encoding="utf-8")
            (Path(temporary.name) / "fragments" / "link.json").symlink_to(outside)
            document = json.loads(manifest.read_text(encoding="utf-8"))
            document["surfaces"][0]["path"] = "fragments/link.json"
            manifest.write_text(json.dumps(document), encoding="utf-8")
            self.assert_rejected(manifest, output, "symlink")

    def test_outside_fragments_path_is_rejected(self) -> None:
        temporary, manifest, output = self.fixture()
        with temporary:
            outside = Path(temporary.name) / "outside.json"
            outside.write_text(json.dumps({"id": "main"}), encoding="utf-8")
            document = json.loads(manifest.read_text(encoding="utf-8"))
            document["surfaces"][0]["path"] = "outside.json"
            manifest.write_text(json.dumps(document), encoding="utf-8")
            self.assert_rejected(manifest, output, "beneath fragments directory")

    def test_nested_orphan_is_rejected(self) -> None:
        temporary, manifest, output = self.fixture()
        with temporary:
            nested = Path(temporary.name) / "fragments" / "nested"
            nested.mkdir()
            (nested / "orphan.json").write_text(json.dumps({"id": "orphan"}), encoding="utf-8")
            self.assert_rejected(manifest, output, "nested/orphan.json")

    def test_duplicate_retired_ids_are_rejected(self) -> None:
        temporary, manifest, output = self.fixture()
        with temporary:
            fragments = Path(temporary.name) / "fragments"
            for name in ("old-a.json", "old-b.json"):
                (fragments / name).write_text(json.dumps({"id": "old"}), encoding="utf-8")
            document = json.loads(manifest.read_text(encoding="utf-8"))
            document["retired"] = [
                {"id": "old", "reason": "legacy", "path": "fragments/old-a.json"},
                {"id": "old", "reason": "legacy", "path": "fragments/old-b.json"},
            ]
            manifest.write_text(json.dumps(document), encoding="utf-8")
            self.assert_rejected(manifest, output, "duplicate retired surface id")


if __name__ == "__main__":
    unittest.main()
