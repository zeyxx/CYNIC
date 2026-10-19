"""Architecture boundary tests — the hexagonal guards.

The domain must stay pure: no I/O, no drivers. Adapters may use them. This is the
same spirit as the Rust ``architecture_boundary.rs`` per-crate guard.
"""

from __future__ import annotations

import ast
from pathlib import Path

SRC = Path(__file__).resolve().parents[1] / "src" / "cynic_data"

FORBIDDEN_IN_DOMAIN = {
    "sqlite3", "psycopg2", "requests", "urllib", "http", "socket",
    "surreal", "surrealdb", "duckdb", "pyarrow", "subprocess", "os",
    # filesystem/stream I/O must live in adapters, never in the domain core
    "pathlib", "io", "shutil", "tempfile", "glob", "pickle",
}


def _domain_modules() -> list[Path]:
    return [
        p
        for p in SRC.rglob("domain/*.py")
        if p.name != "__init__.py"
    ]


def test_domain_modules_exist():
    mods = _domain_modules()
    assert mods, "no domain modules found — the layout is wrong"


def test_domain_has_no_io_imports():
    offenders: list[str] = []
    for mod in _domain_modules():
        tree = ast.parse(mod.read_text(encoding="utf-8"), filename=str(mod))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    root = alias.name.split(".")[0]
                    if root in FORBIDDEN_IN_DOMAIN:
                        offenders.append(f"{mod.name}: import {alias.name}")
            elif isinstance(node, ast.ImportFrom):
                if node.module:
                    root = node.module.split(".")[0]
                    if root in FORBIDDEN_IN_DOMAIN:
                        offenders.append(f"{mod.name}: from {node.module}")
    assert not offenders, "domain modules must be pure:\n" + "\n".join(offenders)


def test_kernel_has_no_io_imports():
    kernel = SRC / "kernel"
    offenders: list[str] = []
    for mod in kernel.rglob("*.py"):
        if mod.name == "__init__.py":
            continue
        tree = ast.parse(mod.read_text(encoding="utf-8"), filename=str(mod))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    if alias.name.split(".")[0] in FORBIDDEN_IN_DOMAIN:
                        offenders.append(f"{mod.name}: import {alias.name}")
            elif isinstance(node, ast.ImportFrom):
                if node.module and node.module.split(".")[0] in FORBIDDEN_IN_DOMAIN:
                    offenders.append(f"{mod.name}: from {node.module}")
    assert not offenders, "kernel must be pure:\n" + "\n".join(offenders)
