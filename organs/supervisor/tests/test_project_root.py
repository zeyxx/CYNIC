"""Tests for shared supervisor project root resolution helpers."""

import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


def _resolve_repo_root(path: Path) -> Path:
    start = path if path.is_dir() else path.parent
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return start


MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "core" / "project_root.py"
REPO_ROOT = _resolve_repo_root(Path(__file__).resolve())
FORGE_PROJECT_ROOT_PATH = REPO_ROOT / "organs" / "forge" / "src" / "core" / "project_root.py"
TWITTER_PROJECT_ROOT_PATH = REPO_ROOT / "organs" / "twitter" / "src" / "core" / "project_root.py"
_project_root = load_module_from_path("supervisor_project_root", MODULE_PATH)
_forge_project_root = load_module_from_path("forge_project_root", FORGE_PROJECT_ROOT_PATH)
_twitter_project_root = load_module_from_path("twitter_project_root", TWITTER_PROJECT_ROOT_PATH)


class ProjectRootHelperTests(unittest.TestCase):
    def test_env_root_wins_when_markers_are_ambiguous(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            explicit_root = root / "explicit"
            (explicit_root / "organs").mkdir(parents=True)
            (explicit_root / "infra").mkdir()

            ambiguous_root = root / "ambiguous"
            (ambiguous_root / "organs").mkdir(parents=True)
            (ambiguous_root / "infra").mkdir()
            start = ambiguous_root / "organs" / "supervisor" / "src" / "foo.py"
            start.parent.mkdir(parents=True)
            start.write_text("# noop\n", encoding="utf-8")

            with patch.dict(os.environ, {"CYNIC_PROJECT_ROOT": str(explicit_root)}):
                result = _project_root.resolve_project_root(start)
                self.assertEqual(explicit_root, result)

    def test_fallback_marker_search_without_git(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "organs").mkdir()
            (root / "infra").mkdir()
            start = root / "organs" / "supervisor" / "src" / "foo.py"
            start.parent.mkdir(parents=True)
            start.write_text("# noop\n", encoding="utf-8")

            result = _project_root.resolve_project_root(start)
            self.assertEqual(root, result)

    def test_git_directory_takes_precedence(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "organs").mkdir()
            (root / "infra").mkdir()
            (root / ".git").mkdir()
            start = root / "organs" / "supervisor" / "src" / "foo.py"
            start.parent.mkdir(parents=True)
            start.write_text("# noop\n", encoding="utf-8")

            result = _project_root.resolve_project_root(start)
            self.assertEqual(root, result)

    def test_shared_helpers_follow_the_same_root_contract(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "organs").mkdir()
            (root / "infra").mkdir()
            (root / ".git").mkdir()
            start = root / "organs" / "supervisor" / "src" / "foo.py"
            start.parent.mkdir(parents=True)
            start.write_text("# noop\n", encoding="utf-8")

            self.assertEqual(root, _forge_project_root.resolve_project_root(start))
            self.assertEqual(root, _twitter_project_root.resolve_project_root(start))

    def test_shared_helpers_respect_explicit_environment_root(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            explicit_root = root / "explicit"
            (explicit_root / "organs").mkdir(parents=True)
            (explicit_root / "infra").mkdir()

            ambiguous_root = root / "ambiguous"
            (ambiguous_root / "organs").mkdir(parents=True)
            (ambiguous_root / "infra").mkdir()
            start = ambiguous_root / "organs" / "supervisor" / "src" / "foo.py"
            start.parent.mkdir(parents=True)
            start.write_text("# noop\n", encoding="utf-8")

            with patch.dict(os.environ, {"CYNIC_PROJECT_ROOT": str(explicit_root)}):
                self.assertEqual(explicit_root, _forge_project_root.resolve_project_root(start))
                self.assertEqual(explicit_root, _twitter_project_root.resolve_project_root(start))
                self.assertEqual(explicit_root, _project_root.resolve_project_root(start))


if __name__ == "__main__":
    unittest.main()
