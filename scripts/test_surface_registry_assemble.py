#!/usr/bin/env python3
"""Deterministic adversarial probes for surface-registry-assemble."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from importlib.machinery import SourceFileLoader
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
ASSEMBLER = ROOT / "scripts" / "surface-registry-assemble"
SCHEMA = "cynic.surface_registry.v1"


_ASSEMBLER_SPEC = importlib.util.spec_from_loader(
    "surface_registry_assemble", SourceFileLoader("surface_registry_assemble", str(ASSEMBLER))
)
assert _ASSEMBLER_SPEC is not None and _ASSEMBLER_SPEC.loader is not None
ASSEMBLER_MODULE = importlib.util.module_from_spec(_ASSEMBLER_SPEC)
_ASSEMBLER_SPEC.loader.exec_module(ASSEMBLER_MODULE)


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

    def test_nested_symlink_entry_is_rejected(self) -> None:
        temporary, manifest, output = self.fixture()
        with temporary:
            nested = Path(temporary.name) / "fragments" / "nested"
            nested.mkdir()
            (nested / "link.json").symlink_to(Path(temporary.name) / "fragments" / "main.json")
            self.assert_rejected(manifest, output, "symlink")

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
    def test_root_identity_mismatch_fails_closed_and_closes_root(self) -> None:
        temporary, manifest, _ = self.fixture()
        with temporary:
            root = manifest.parent.resolve()
            closed: list[int] = []
            with mock.patch.object(ASSEMBLER_MODULE.os, "open", return_value=101), mock.patch.object(
                ASSEMBLER_MODULE.os, "readlink", return_value="/tmp"
            ), mock.patch.object(ASSEMBLER_MODULE.os, "close", side_effect=closed.append):
                with self.assertRaises(SystemExit):
                    ASSEMBLER_MODULE.build_document(
                        json.loads(manifest.read_text(encoding="utf-8")), manifest.parent
                    )
            self.assertEqual(closed, [101])

    def test_fragments_open_failure_closes_root_and_duplicate_descriptor(self) -> None:
        temporary, manifest, _ = self.fixture()
        with temporary:
            root = manifest.parent.resolve()
            opened = iter([101])
            closed: list[int] = []

            def open_side_effect(*args: object, **kwargs: object) -> int:
                try:
                    return next(opened)
                except StopIteration:
                    raise PermissionError(13, "permission denied")

            with mock.patch.object(ASSEMBLER_MODULE.os, "open", side_effect=open_side_effect), mock.patch.object(
                ASSEMBLER_MODULE.os, "readlink", return_value=str(root)
            ), mock.patch.object(ASSEMBLER_MODULE.os, "dup", return_value=202), mock.patch.object(
                ASSEMBLER_MODULE.os, "close", side_effect=closed.append
            ):
                with self.assertRaises(SystemExit):
                    ASSEMBLER_MODULE.build_document(
                        json.loads(manifest.read_text(encoding="utf-8")), manifest.parent
                    )
            self.assertEqual(closed, [202, 101])


if __name__ == "__main__":
    unittest.main()
