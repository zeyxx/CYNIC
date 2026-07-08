"""Tests for the shared cynic-python project root helper."""

import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from project_root import resolve_repo_root


class ProjectRootTests(unittest.TestCase):
    def test_env_root_wins_when_markers_are_ambiguous(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            explicit_root = root / "explicit"
            (explicit_root / "organs").mkdir(parents=True)
            (explicit_root / "infra").mkdir()

            ambiguous_root = root / "ambiguous"
            (ambiguous_root / "organs").mkdir(parents=True)
            (ambiguous_root / "infra").mkdir()
            start = ambiguous_root / "services" / "cynic-python" / "tests" / "foo.py"
            start.parent.mkdir(parents=True)
            start.write_text("# noop\n", encoding="utf-8")

            with patch.dict(os.environ, {"CYNIC_PROJECT_ROOT": str(explicit_root)}):
                self.assertEqual(explicit_root, resolve_repo_root(start))

    def test_marker_search_without_git(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "organs").mkdir()
            (root / "infra").mkdir()
            start = root / "services" / "cynic-python" / "tests" / "foo.py"
            start.parent.mkdir(parents=True)
            start.write_text("# noop\n", encoding="utf-8")

            self.assertEqual(root, resolve_repo_root(start))


if __name__ == "__main__":
    unittest.main()
