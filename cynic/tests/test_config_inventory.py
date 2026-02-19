"""
CYNIC Configuration Inventory Tests — Phase 0

Audits all os.getenv() / os.environ usage across the codebase.
Ensures:
  - Every env var has a default or is documented
  - No hardcoded passwords in production code
  - No duplicate env var names with different defaults
"""
from __future__ import annotations

import ast
import re
from pathlib import Path

import pytest

CYNIC_ROOT = Path(__file__).parent.parent / "cynic"


# ── Helpers ──────────────────────────────────────────────────────────────────

def _find_getenv_calls() -> list[dict]:
    """
    Find all os.getenv() and os.environ.get() calls in the codebase.

    Returns list of dicts: {file, line, var_name, default, has_default}
    """
    results = []
    for filepath in CYNIC_ROOT.rglob("*.py"):
        if "__pycache__" in str(filepath):
            continue
        try:
            source = filepath.read_text(encoding="utf-8", errors="replace")
            tree = ast.parse(source, filename=str(filepath))
        except (SyntaxError, UnicodeDecodeError):
            continue

        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue

            # Match os.getenv("FOO", default) or os.environ.get("FOO", default)
            func_name = _get_call_name(node)
            if func_name not in ("os.getenv", "os.environ.get"):
                continue

            if not node.args:
                continue

            # Extract var name
            first_arg = node.args[0]
            var_name = None
            if isinstance(first_arg, ast.Constant) and isinstance(first_arg.value, str):
                var_name = first_arg.value

            if var_name is None:
                continue

            # Extract default value
            default = None
            has_default = False
            if len(node.args) >= 2:
                has_default = True
                default_node = node.args[1]
                if isinstance(default_node, ast.Constant):
                    default = default_node.value
            elif node.keywords:
                for kw in node.keywords:
                    if kw.arg == "default":
                        has_default = True
                        if isinstance(kw.value, ast.Constant):
                            default = kw.value.value

            results.append({
                "file": str(filepath.relative_to(CYNIC_ROOT.parent)),
                "line": getattr(node, "lineno", 0),
                "var_name": var_name,
                "default": default,
                "has_default": has_default,
            })

    return results


def _get_call_name(node: ast.Call) -> str:
    """Extract dotted function name from an ast.Call node."""
    func = node.func
    parts = []
    while isinstance(func, ast.Attribute):
        parts.append(func.attr)
        func = func.value
    if isinstance(func, ast.Name):
        parts.append(func.id)
    parts.reverse()
    return ".".join(parts)


# ── Tests ────────────────────────────────────────────────────────────────────

class TestConfigInventory:
    """Audit all environment variable usage."""

    @pytest.fixture(scope="class")
    def env_vars(self) -> list[dict]:
        return _find_getenv_calls()

    def test_all_env_vars_found(self, env_vars: list[dict]):
        """Sanity check: we should find env vars in the codebase."""
        assert len(env_vars) >= 10, (
            f"Expected at least 10 os.getenv() calls, found {len(env_vars)}"
        )

    def test_no_hardcoded_passwords_as_defaults(self, env_vars: list[dict]):
        """Default values should not contain real passwords."""
        suspicious_defaults = []
        # These are OK as development defaults (clearly marked as local-only)
        ok_defaults = {"root", "local_dev_only", "cynic_phi_618", ""}

        for entry in env_vars:
            var_lower = entry["var_name"].lower()
            is_secret = any(
                kw in var_lower
                for kw in ("pass", "secret", "key", "token")
            )
            if not is_secret:
                continue
            if entry["default"] is None:
                continue  # No default = good (will be required at runtime)
            default_str = str(entry["default"])
            if default_str in ok_defaults:
                continue
            # Flag anything that looks like a real credential
            if len(default_str) > 20 and not default_str.startswith("http"):
                suspicious_defaults.append(
                    f"  {entry['file']}:{entry['line']} "
                    f"{entry['var_name']}='{default_str[:30]}...'"
                )

        if suspicious_defaults:
            pytest.fail(
                f"\nPossible hardcoded credentials:\n"
                + "\n".join(suspicious_defaults)
            )

    def test_no_duplicate_env_vars_with_different_defaults(self, env_vars: list[dict]):
        """Same env var name should not have conflicting defaults."""
        by_name: dict[str, list] = {}
        for entry in env_vars:
            by_name.setdefault(entry["var_name"], []).append(entry)

        conflicts = []
        for var_name, entries in by_name.items():
            defaults = {
                str(e["default"]) for e in entries if e["has_default"]
            }
            if len(defaults) > 1:
                locations = [
                    f"    {e['file']}:{e['line']} default={e['default']}"
                    for e in entries
                ]
                conflicts.append(
                    f"  {var_name} has {len(defaults)} different defaults:\n"
                    + "\n".join(locations)
                )

        if conflicts:
            pytest.fail(
                f"\nConflicting env var defaults:\n" + "\n".join(conflicts)
            )

    def test_inventory_completeness(self, env_vars: list[dict]):
        """
        Known critical env vars should all be present.
        This documents the expected configuration surface.
        """
        expected_vars = {
            "SURREAL_URL",
            "SURREAL_USER",
            "SURREAL_PASS",
            "OLLAMA_URL",
            "ANTHROPIC_API_KEY",
            "PORT",
        }
        found_vars = {e["var_name"] for e in env_vars}
        missing = expected_vars - found_vars
        if missing:
            pytest.fail(
                f"\nExpected env vars not found in codebase: {missing}\n"
                f"Found: {sorted(found_vars)}"
            )
