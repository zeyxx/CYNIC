"""
Linter: Detect orphaned async functions.

This tool prevents technical debt by catching async functions that:
1. Are defined but never called (orphaned)
2. Are called with create_task() instead of await (fire-and-forget)
3. Don't raise errors on failure (silent failures)

Run: python -m cynic.tools.lint_persistence [--fix]
"""
import ast
import re
import sys
from pathlib import Path
from typing import List, Tuple


class AsyncLinter(ast.NodeVisitor):
    """AST visitor that finds async function usage patterns."""

    def __init__(self):
        self.async_functions: dict[str, Tuple[str, int]] = {}  # name -> (file, line)
        self.async_calls: set[str] = set()  # functions that are called
        self.fire_and_forget_patterns: List[Tuple[str, int]] = []  # (location, line)
        self.silent_failures: List[Tuple[str, int]] = []  # (location, line)
        self.current_file = ""

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef):
        """Track async function definitions."""
        self.async_functions[node.name] = (self.current_file, node.lineno)
        self.generic_visit(node)

    def visit_Call(self, node: ast.Call):
        """Track function calls."""
        # Direct calls like await foo()
        if isinstance(node.func, ast.Name):
            self.async_calls.add(node.func.id)

        # create_task calls (fire-and-forget pattern)
        if isinstance(node.func, ast.Attribute):
            if node.func.attr == "create_task":
                if len(node.args) > 0 and isinstance(node.args[0], ast.Call):
                    inner_call = node.args[0]
                    if isinstance(inner_call.func, ast.Name):
                        func_name = inner_call.func.id
                        if func_name in self.async_functions:
                            self.fire_and_forget_patterns.append((f"{self.current_file}:{node.lineno}", node.lineno))

        self.generic_visit(node)

    def visit_Try(self, node: ast.Try):
        """Detect except clauses that silently catch exceptions."""
        for handler in node.handlers:
            if len(handler.body) > 0:
                # Check if handler just logs or passes
                last_stmt = handler.body[-1]
                if isinstance(last_stmt, ast.Pass):
                    self.silent_failures.append((f"{self.current_file}:{node.lineno}", node.lineno))
                elif isinstance(last_stmt, ast.Expr):
                    # Check if it's just a logging call without re-raise
                    is_just_logging = isinstance(last_stmt.value, ast.Call)
                    has_raise = any(isinstance(s, ast.Raise) for s in handler.body)
                    if is_just_logging and not has_raise:
                        self.silent_failures.append((f"{self.current_file}:{node.lineno}", node.lineno))

        self.generic_visit(node)


def lint_file(filepath: Path) -> Tuple[List[str], AsyncLinter]:
    """Lint a single file."""
    linter = AsyncLinter()
    linter.current_file = str(filepath)

    try:
        with open(filepath) as f:
            tree = ast.parse(f.read())
        linter.visit(tree)
    except SyntaxError as e:
        return [f"SyntaxError in {filepath}: {e}"], linter

    return [], linter


def find_orphaned_async_functions(root_path: Path) -> dict[str, List[str]]:
    """Find all orphaned async functions."""
    issues = {
        "orphaned": [],
        "fire_and_forget": [],
        "silent_failures": [],
    }

    all_async_functions = {}
    all_async_calls = set()
    all_linters = []

    # First pass: collect all definitions and calls
    for py_file in root_path.rglob("*.py"):
        # Skip test files and __pycache__
        if "__pycache__" in str(py_file) or str(py_file).endswith("_test.py"):
            continue

        errors, linter = lint_file(py_file)
        if errors:
            for error in errors:
                issues["orphaned"].append(error)
            continue

        all_async_functions.update(linter.async_functions)
        all_async_calls.update(linter.async_calls)
        all_linters.append(linter)

    # Second pass: find orphaned functions
    for func_name, (filepath, lineno) in all_async_functions.items():
        # Exclude private functions and common patterns
        if func_name.startswith("_on_") or func_name.startswith("_"):
            continue  # Event handlers are OK to be "orphaned" — called via event bus

        if func_name not in all_async_calls:
            issues["orphaned"].append(f"{filepath}:{lineno} - {func_name}() is never called")

    # Collect fire-and-forget patterns
    for linter in all_linters:
        for location, _ in linter.fire_and_forget_patterns:
            issues["fire_and_forget"].append(f"{location} - fire-and-forget create_task() detected")

    # Collect silent failures
    for linter in all_linters:
        for location, _ in linter.silent_failures:
            issues["silent_failures"].append(f"{location} - exception caught but not re-raised")

    return issues


def main():
    """Main entry point."""
    root_path = Path.cwd() / "cynic"

    if not root_path.exists():
        print(f"Error: {root_path} not found")
        sys.exit(1)

    print("🔍 Scanning for async linting issues...")
    print()

    issues = find_orphaned_async_functions(root_path)

    had_issues = False
    for issue_type, locations in issues.items():
        if locations:
            had_issues = True
            print(f"❌ {issue_type.upper().replace('_', ' ')}: ({len(locations)} found)")
            for location in locations:
                print(f"   {location}")
            print()

    if not had_issues:
        print("✅ No async linting issues found!")
        sys.exit(0)
    else:
        print(f"Found {sum(len(v) for v in issues.values())} issues")
        sys.exit(1)


if __name__ == "__main__":
    main()
