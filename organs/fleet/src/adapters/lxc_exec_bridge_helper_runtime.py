"""Runtime adapter for the root-owned LXC exec bridge helper."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import json_object_or_default, read_json_file_or_default
from organs.fleet.src.core.exec_bridge import LxcExecBridgePolicy, LxcExecRequest


def response(status: str, **extra: Any) -> dict[str, Any]:
    payload = {
        "schema": "cynic.lxc_exec_bridge.response.v1",
        "status": status,
    }
    payload.update(extra)
    return payload


def load_policy(
    path: Path | None,
    *,
    default_policy_paths: list[Path],
) -> LxcExecBridgePolicy:
    candidates = []
    if path is not None:
        candidates.append(path)
    env_path = os.environ.get("CYNIC_LXC_EXEC_BRIDGE_POLICY")
    if env_path:
        candidates.append(Path(env_path))
    candidates.extend(default_policy_paths)

    for candidate in candidates:
        if candidate.exists():
            return LxcExecBridgePolicy(json_object_or_default(candidate.read_text(encoding="utf-8"), {}))
    raise FileNotFoundError("no LXC exec bridge policy found")


def load_declared_services(path: Path | None) -> set[str]:
    if path is None or not path.exists():
        return set()
    payload = read_json_file_or_default(path, {})
    services = set()
    for node in payload.get("nodes", []):
        for service in node.get("services", []):
            name = service.get("name")
            if isinstance(name, str):
                services.add(name)
    return services


def run_bridge(
    request_payload: dict[str, Any],
    *,
    policy: LxcExecBridgePolicy,
    declared_services: set[str],
    dry_run: bool = False,
    runner=subprocess.run,
    geteuid=os.geteuid,
) -> dict[str, Any]:
    request = LxcExecRequest.from_json(request_payload)
    validation = policy.validate_request(request, declared_services)
    if not validation.ok:
        return response(
            "rejected",
            violations=list(validation.violations),
            node=request.node,
            vmid=request.vmid,
            purpose=request.purpose,
        )

    pct_argv = request.to_pct_argv()
    if dry_run:
        return response(
            "dry_run",
            command=pct_argv,
            node=request.node,
            vmid=request.vmid,
            purpose=request.purpose,
        )

    if geteuid() != 0:
        return response(
            "rejected",
            violations=["helper_must_run_as_root"],
            node=request.node,
            vmid=request.vmid,
            purpose=request.purpose,
        )

    completed = runner(
        pct_argv,
        check=False,
        text=True,
        capture_output=True,
        timeout=request.timeout_seconds,
    )
    return response(
        "completed",
        node=request.node,
        vmid=request.vmid,
        purpose=request.purpose,
        exit_code=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )
