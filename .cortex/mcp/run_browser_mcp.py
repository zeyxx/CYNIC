#!/usr/bin/env python3
"""Launch the browser MCP server without a shell wrapper."""

from __future__ import annotations

import os
import sys
from pathlib import Path

PROJECT_ROOT_CANDIDATE = Path(__file__).resolve().parents[2]
if str(PROJECT_ROOT_CANDIDATE) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT_CANDIDATE))

from organs.contracts.src.adapters.env import load_env_file
from organs.contracts.src.project_root import resolve_repo_root


def main() -> int:
    load_env_file(Path.home() / ".cynic-env")
    root = resolve_repo_root(PROJECT_ROOT_CANDIDATE)
    os.environ["CYNIC_PROJECT_ROOT"] = str(root)
    pythonpath = os.environ.get("PYTHONPATH", "")
    pythonpath_parts = [part for part in pythonpath.split(os.pathsep) if part]
    if str(root) not in pythonpath_parts:
        os.environ["PYTHONPATH"] = os.pathsep.join([str(root), *pythonpath_parts])
    os.environ.setdefault("ORGAN_BROWSER_STATE_DIR", str(root / "organs/browser/state"))
    os.chdir(root)
    server = root / "organs/browser/src/entrypoints/mcp_server.py"
    organ_venv = Path(os.environ.get("ORGAN_BROWSER_VENV", str(root / ".cynic/organs/browser/.venv")))
    organ_python = organ_venv / "bin/python"
    python = organ_python if organ_python.exists() and os.access(organ_python, os.X_OK) else Path(sys.executable)
    os.execv(str(python), [str(python), str(server)])
    return 127


if __name__ == "__main__":
    raise SystemExit(main())
