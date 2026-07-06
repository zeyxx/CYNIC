"""Backward-compatible compatibility wrapper.

Legacy entrypoint for the supervisor topology test suite.

The monolithic supervisor topology tests were split into concern-specific
modules for governance clarity and maintainability:

- test_validate_repo_topology_maturity_and_roots.py
- test_validate_repo_topology_organ_shape.py
- test_validate_repo_topology_surfaces.py
- test_validate_repo_topology_theme_documents.py
- test_validate_repo_topology_contracts.py
- test_validate_repo_topology_git_membrane.py
- test_validate_repo_topology_policy_legacy.py

Executing this module directly will run all topology submodules below; this file
does not add new assertions.
"""

from __future__ import annotations

import importlib.util
import pathlib
import unittest
import sys


try:
    from ._test_utils import resolve_project_root
except ImportError:
    pass


def _load_topology_subtests() -> unittest.TestSuite:
    try:
        repo_root = resolve_project_root(pathlib.Path(__file__), fallback_depth=3)
    except Exception:
        fallback = pathlib.Path(__file__).resolve()
        for candidate in [fallback, *fallback.parents]:
            if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
                repo_root = candidate
                break
        else:
            repo_root = fallback
    if str(repo_root) not in sys.path:
        sys.path.insert(0, str(repo_root))

    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    root = pathlib.Path(__file__).resolve().parent
    test_files = sorted(root.glob("test_validate_repo_topology_*.py"))
    for path in test_files:
        module_name = f"cynic_test_legacy_validate_repo_topology_{path.stem}"
        spec = importlib.util.spec_from_file_location(module_name, path)
        if spec is None or spec.loader is None:
            raise RuntimeError(f"Cannot load test module from {path}")
        module = importlib.util.module_from_spec(spec)
        sys.modules[module_name] = module
        spec.loader.exec_module(module)
        suite.addTests(loader.loadTestsFromModule(module))

    return suite


def main() -> None:
    result = unittest.TextTestRunner(verbosity=2).run(_load_topology_subtests())
    raise SystemExit(0 if result.wasSuccessful() else 1)


if __name__ == "__main__":
    main()
