"""Shared helpers for organism stack topology reporting."""

from __future__ import annotations

import ast
import re
from pathlib import Path


UTILITY_DUPLICATE_ALLOWLIST = {
    "main",
    "check",
    "run",
    "handler",
    "setup",
}


def read_text_without_docstrings(path: Path) -> str:
    text = read_text(path)
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return text
    lines = text.splitlines()
    for node in ast.walk(tree):
        if not isinstance(node, (ast.Module, ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            continue
        body = getattr(node, "body", [])
        if not body:
            continue
        first = body[0]
        if not (
            isinstance(first, ast.Expr)
            and isinstance(first.value, ast.Constant)
            and isinstance(first.value.value, str)
            and hasattr(first, "lineno")
            and hasattr(first, "end_lineno")
        ):
            continue
        for index in range(first.lineno - 1, first.end_lineno):
            if 0 <= index < len(lines):
                lines[index] = ""
    return "\n".join(lines)


def duplicates(values: list[str]) -> list[str]:
    seen: set[str] = set()
    duplicate_values: set[str] = set()
    for value in values:
        if value in seen:
            duplicate_values.add(value)
        seen.add(value)
    return sorted(duplicate_values)


def count_files(path: Path) -> int:
    if not path.is_dir():
        return 0
    return sum(1 for item in path.rglob("*") if item.is_file())


def utility_function_duplicates(root: Path) -> dict[str, list[str]]:
    organs_root = root / "organs"
    if not organs_root.is_dir():
        return {}
    definitions: dict[str, list[str]] = {}
    function_pattern = re.compile(
        r"^def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\([^)]*\).*?\n(?P<body>(?:[ \t]+.*\n|[ \t]*\n)*)",
        flags=re.MULTILINE,
    )
    for path in organs_root.glob("*/src/**/*.py"):
        if is_ignored_duplicate_scan_path(path):
            continue
        for match in function_pattern.finditer(read_text(path)):
            name = match.group(1)
            if name.startswith("_") or name in UTILITY_DUPLICATE_ALLOWLIST or not is_utility_function_name(name):
                continue
            if delegates_to_shared_contract_port(match.group("body")) or delegates_to_imported_alias(match.group("body"), name):
                continue
            definitions.setdefault(name, []).append(rel(root, path))
    return {
        name: sorted(dict.fromkeys(paths))
        for name, paths in sorted(definitions.items())
        if len(set(paths)) > 1
    }


def is_ignored_duplicate_scan_path(path: Path) -> bool:
    parts = set(path.parts)
    return (
        "tests" in parts
        or path.name == "__init__.py"
        or path.name == "_bootstrap.py"
        or path.name.startswith("test_")
        or path.match("*/src/core/project_root.py")
        or path.match("*/src/project_root.py")
    )


def is_utility_function_name(name: str) -> bool:
    return name.startswith(
        (
            "load_",
            "read_",
            "write_",
            "resolve_",
            "parse_",
            "ensure_",
            "fetch_",
            "post_",
            "get_",
            "selected_",
            "wait_",
        )
    ) or name in {"pid_alive"}


def delegates_to_shared_contract_port(body: str) -> bool:
    return any(
        marker in body
        for marker in (
            "read_json_file_or_default(",
            "read_json_file_or_raise(",
            "read_toml_file_with_loader(",
            "read_yaml_file_with_loader(",
            "ensure_parent_dirs(",
            "ensure_dirs(",
            "load_env_file(",
            "load_required_env_from_default_file(",
            "select_mcp_binary(",
            "resolve_repo_root(",
            "post_observe_payload(",
            "post_kernel_observe(",
        )
    )


def delegates_to_imported_alias(body: str, name: str) -> bool:
    return (
        f"_{name}(" in body
        or f"runtime.{name}(" in body
        or re.search(rf"\b[a-zA-Z_][a-zA-Z0-9_]*\.{re.escape(name)}\(", body) is not None
    )


def yaml_scalar(text: str, key: str) -> str | None:
    match = re.search(rf"^{re.escape(key)}:\s*(.+?)\s*$", text, flags=re.MULTILINE)
    if not match:
        return None
    value = match.group(1).strip().strip("'\"")
    return value or None


def yaml_multiline_value(text: str, key: str) -> str | None:
    scalar = yaml_scalar(text, key)
    if scalar and scalar != "|":
        return scalar
    match = re.search(rf"^{re.escape(key)}:\s*\|\s*\n((?:\s+.+\n?)*)", text, flags=re.MULTILINE)
    if not match:
        return scalar
    lines = [line.strip() for line in match.group(1).splitlines() if line.strip()]
    return " ".join(lines)[:240] if lines else None


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def rel(root: Path, path: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def tail(text: str, limit: int = 240) -> str:
    clean = " ".join(text.split())
    return clean[-limit:] if len(clean) > limit else clean
