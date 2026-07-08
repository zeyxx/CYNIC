from __future__ import annotations

import asyncio
import json
import sys
from pathlib import Path
from typing import Protocol

try:
    from organs.lifecycle.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


if __package__ in {None, ""}:
    project_root = bootstrap_repo_path()
    if str(project_root) not in sys.path:
        sys.path.insert(0, str(project_root))

from organs.lifecycle.src.entrypoints.controller import LifecycleController


class _ReconcileController(Protocol):
    async def reconcile(self) -> dict: ...


async def run_watchdog_once(controller: _ReconcileController | None = None) -> dict:
    controller = controller or LifecycleController()
    return await controller.reconcile()


def main() -> int:
    result = asyncio.run(run_watchdog_once())
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
