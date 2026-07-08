"""Structural regressions guardrails for supervisor."""

from pathlib import Path
import re
import unittest


try:
    from ._test_utils import resolve_project_root
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import resolve_project_root


ROOT = resolve_project_root(Path(__file__).resolve())
HARD_CODED_PARENT_RE = re.compile(r"\.parents\[[0-9]+\]")


class StructuralConventionTests(unittest.TestCase):
    def test_no_hard_coded_parent_indexes_in_python_sources(self) -> None:
        offenders: list[tuple[Path, str]] = []
        source_root = ROOT / "organs" / "supervisor"
        for path in source_root.rglob("*.py"):
            if "__pycache__" in path.parts:
                continue
            text = path.read_text(encoding="utf-8")
            for match in HARD_CODED_PARENT_RE.finditer(text):
                line = text[: match.start()].count("\n") + 1
                offenders.append((path.relative_to(ROOT), f"L{line}: {match.group(0)}"))

        self.assertFalse(
            offenders,
            "Hard-coded Path.parents[] indexes detected:\n"
            + "\n".join(f"{path} -> {line}" for path, line in offenders),
        )


if __name__ == "__main__":
    unittest.main()
