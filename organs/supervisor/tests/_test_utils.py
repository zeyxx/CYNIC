"""Test helpers for local supervisor tests."""

from __future__ import annotations

import os
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from types import ModuleType
import sys


def resolve_project_root(module_path: Path, fallback_depth: int = 3) -> Path:
    path = Path(module_path).resolve()
    raw_root = os.environ.get("CYNIC_PROJECT_ROOT", "").strip()
    if raw_root:
        env_root = Path(raw_root).expanduser().resolve()
        if (env_root / "organs").is_dir() and (env_root / "infra").is_dir():
            return env_root

    for candidate in [path, *path.parents]:
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate

    for candidate in [path, *path.parents]:
        if (candidate / ".git").is_dir():
            return candidate

    fallback_index = max(0, min(fallback_depth, len(path.parents)))
    return path.parents[fallback_index]


def load_module_from_path(module_name: str, module_path: Path) -> ModuleType:
    """Load a Python module from an explicit file path."""

    module_path = Path(module_path)
    repo_root = resolve_project_root(module_path)

    repo_root_str = str(repo_root)
    if repo_root_str not in sys.path:
        sys.path.insert(0, repo_root_str)

    spec = spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot load module from {module_path}")

    module = module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module
