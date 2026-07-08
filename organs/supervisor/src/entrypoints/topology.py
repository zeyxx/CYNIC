"""Supervisor topology entrypoint.

This module is the canonical runnable surface for topology validation.
It delegates to the controller so legacy entry contracts remain stable while
preserving entrypoint-layer separation.
"""

from __future__ import annotations

import sys


try:
    from organs.supervisor.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.supervisor.src.controllers.validate_repo_topology import main as _controller_main


def main() -> int:
    return _controller_main()


if __name__ == "__main__":
    raise SystemExit(main())
