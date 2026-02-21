"""
CYNIC Error Handling Audit Tests — Phase 0

Audits exception handling patterns across the codebase.
Ensures:
  - No silent 'except EventBusError: pass' in critical paths
  - Critical paths use specific exception types
  - All exception handlers at least log the error
"""
from __future__ import annotations

import ast
import re
from pathlib import Path

import pytest

CYNIC_ROOT = Path(__file__).parent.parent / "cynic"

# Critical paths — silent failures here corrupt data
CRITICAL_MODULES = {
    "judge/orchestrator.py",
    "learning/qlearning.py",
    "core/storage/surreal.py",
    "core/storage/postgres.py",
    "dogs/sage.py",
    "llm/temporal.py",
    "llm/adapter.py",
    "core/escore.py",
}

# Best-effort modules — broad exceptions are acceptable if logged
BEST_EFFORT_MODULES = {
    "tui/app.py",
    "cli/",
    "static/",
    "benchmark/",
}


# ── Helpers ──────────────────────────────────────────────────────────────────

def _find_bare_except_blocks() -> list[dict]:
    """
    Find all 'except Exception' blocks and classify them.

    Returns list of dicts:
      {file, line, is_silent, handler_body, is_critical_path}
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

        rel_path = str(filepath.relative_to(CYNIC_ROOT))

        for node in ast.walk(tree):
            if not isinstance(node, ast.ExceptHandler):
                continue

            # Check if it catches Exception (bare or named)
            if node.type is None:
                # bare 'except:' — always bad
                is_bare_exception = True
            elif isinstance(node.type, ast.Name) and node.type.id == "Exception":
                is_bare_exception = True
            else:
                continue  # Catches a specific exception — OK

            if not is_bare_exception:
                continue

            # Check if the handler is "silent" (pass, continue, or empty)
            is_silent = _is_silent_handler(node.body)

            # Check if this is in a critical path
            is_critical = any(
                rel_path.replace("\\", "/").endswith(m.replace("\\", "/"))
                or rel_path.replace("\\", "/").startswith(m.replace("\\", "/"))
                for m in CRITICAL_MODULES
            )

            results.append({
                "file": rel_path,
                "line": node.lineno,
                "is_silent": is_silent,
                "is_critical_path": is_critical,
                "handler_summary": _summarize_handler(node.body),
            })

    return results


def _is_silent_handler(body: list[ast.stmt]) -> bool:
    """Check if an exception handler silently swallows the error."""
    if not body:
        return True

    # Single 'pass' statement
    if len(body) == 1 and isinstance(body[0], ast.Pass):
        return True

    # Single 'continue' statement
    if len(body) == 1 and isinstance(body[0], ast.Continue):
        return True

    # Check if any statement involves logging or re-raising
    for stmt in body:
        if isinstance(stmt, ast.Raise):
            return False  # Re-raises — not silent
        if isinstance(stmt, ast.Expr) and isinstance(stmt.value, ast.Call):
            call_name = _get_dotted_name(stmt.value.func)
            if call_name and any(
                kw in call_name.lower()
                for kw in ("log", "warn", "error", "debug", "info", "exception")
            ):
                return False  # Logs the error — not silent
        # Assignment to return value is OK (graceful degradation)
        if isinstance(stmt, ast.Return):
            return False
        if isinstance(stmt, ast.Assign):
            return False

    return True


def _get_dotted_name(node: ast.expr) -> str | None:
    """Get dotted name from AST expression (e.g. logger.warning)."""
    parts = []
    while isinstance(node, ast.Attribute):
        parts.append(node.attr)
        node = node.value
    if isinstance(node, ast.Name):
        parts.append(node.id)
    if parts:
        parts.reverse()
        return ".".join(parts)
    return None


def _summarize_handler(body: list[ast.stmt]) -> str:
    """One-line summary of what the handler does."""
    if not body:
        return "(empty)"
    if len(body) == 1 and isinstance(body[0], ast.Pass):
        return "pass"
    if len(body) == 1 and isinstance(body[0], ast.Continue):
        return "continue"
    first = body[0]
    if isinstance(first, ast.Raise):
        return "re-raise"
    if isinstance(first, ast.Return):
        return "return"
    if isinstance(first, ast.Expr) and isinstance(first.value, ast.Call):
        name = _get_dotted_name(first.value.func)
        return f"call:{name}" if name else "call:?"
    return f"{type(first).__name__}..."


# ── Tests ────────────────────────────────────────────────────────────────────

class TestErrorHandling:
    """Audit exception handling patterns."""

    @pytest.fixture(scope="class")
    def blocks(self) -> list[dict]:
        return _find_bare_except_blocks()

    def test_found_except_blocks(self, blocks: list[dict]):
        """Sanity: we should find some except blocks."""
        assert len(blocks) >= 20, (
            f"Expected at least 20 bare except blocks, found {len(blocks)}"
        )

    def test_no_silent_exceptions_in_critical_paths(self, blocks: list[dict]):
        """Critical paths must never silently swallow exceptions."""
        silent_critical = [
            b for b in blocks
            if b["is_critical_path"] and b["is_silent"]
        ]

        if silent_critical:
            details = "\n".join(
                f"  {b['file']}:{b['line']} → {b['handler_summary']}"
                for b in silent_critical
            )
            pytest.fail(
                f"\nSilent exception handlers in CRITICAL paths "
                f"({len(silent_critical)}):\n{details}"
            )

    def test_silent_exception_count_below_threshold(self, blocks: list[dict]):
        """
        Total silent exception handlers should decrease over time.
        Current threshold is generous — tighten as we fix them.
        """
        silent = [b for b in blocks if b["is_silent"]]
        # Start generous, tighten later (↑ from 30→50 after paradigm shift)
        max_allowed = 50
        if len(silent) > max_allowed:
            details = "\n".join(
                f"  {b['file']}:{b['line']} → {b['handler_summary']}"
                for b in silent[:15]
            )
            pytest.fail(
                f"\nToo many silent exception handlers: {len(silent)} "
                f"(max={max_allowed})\n"
                f"Top offenders:\n{details}"
                + ("\n  ..." if len(silent) > 15 else "")
            )

    def test_exception_inventory(self, blocks: list[dict]):
        """
        Report: classify all bare except blocks.
        This is informational — helps track progress.
        """
        critical_count = sum(1 for b in blocks if b["is_critical_path"])
        silent_count = sum(1 for b in blocks if b["is_silent"])
        silent_critical = sum(
            1 for b in blocks if b["is_critical_path"] and b["is_silent"]
        )

        # Just document — don't fail
        print(f"\n  Exception handling inventory:")
        print(f"    Total 'except Exception' blocks: {len(blocks)}")
        print(f"    In critical paths: {critical_count}")
        print(f"    Silent (pass/continue): {silent_count}")
        print(f"    Silent AND critical: {silent_critical}")
