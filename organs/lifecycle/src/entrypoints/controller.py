from __future__ import annotations

import asyncio
import json

from organs.lifecycle.src.core.controller import (
    FRESHNESS_STALE_SECONDS,
    LifecycleController,
    LifecycleControllerPaths,
    repo_root as _repo_root,
    run_controller_once,
)


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Lifecycle convergence controller")
    parser.add_argument("--health", action="store_true", help="emit non-mutating freshness health")
    parser.add_argument(
        "--reconcile",
        action="store_true",
        help="run health and converge if the SSOT is stale",
    )
    args = parser.parse_args()

    controller = LifecycleController()
    if args.health:
        result = controller.health()
    elif args.reconcile:
        result = asyncio.run(controller.reconcile())
    else:
        result = asyncio.run(controller.run_once())
    print(json.dumps(result, indent=2, sort_keys=True))
    if args.health and result.get("status") != "ok":
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
