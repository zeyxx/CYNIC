#!/usr/bin/env python3
"""Root-owned LXC exec bridge helper.

Installed target path:
  /usr/local/libexec/cynic-lxc-exec-bridge

The helper reads a JSON request on stdin, validates it against the installed
policy, then invokes pct in argv form. It never accepts shell strings.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from organs.contracts.src.surface_contracts import json_object_or_default
from organs.fleet.src.adapters.lxc_exec_bridge_helper_runtime import (
    load_declared_services,
    load_policy as _load_policy,
    response,
    run_bridge,
)

try:
    from organs.fleet.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


ROOT = bootstrap_repo_path()
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from organs.fleet.src.core.exec_bridge import LxcExecBridgePolicy


DEFAULT_POLICY_PATHS = [
    Path("/etc/cynic/lxc-exec-bridge.policy.json"),
    ROOT / "organs/fleet/policies/lxc-exec-bridge.policy.json",
]


def load_policy(path: Path | None) -> LxcExecBridgePolicy:
    return _load_policy(path, default_policy_paths=DEFAULT_POLICY_PATHS)


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC LXC exec bridge helper")
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--registry", type=Path, default=ROOT / "infra/registry.json")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    try:
        request_payload = json_object_or_default(sys.stdin.read() or "{}", {})
        policy = load_policy(args.policy)
        declared_services = load_declared_services(args.registry)
        payload = run_bridge(
            request_payload,
            policy=policy,
            declared_services=declared_services,
            dry_run=args.dry_run,
        )
    except Exception as exc:  # noqa: BLE001 - helper must always emit JSON.
        payload = response("error", error=str(exc))

    print(json.dumps(payload, sort_keys=True))
    return 0 if payload.get("status") in {"completed", "dry_run"} else 1


if __name__ == "__main__":
    raise SystemExit(main())
