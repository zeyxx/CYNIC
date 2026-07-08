"""Runtime adapter for the root-owned host exec helper."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import json_object_or_default, read_json_file_or_default
from organs.fleet.src.core.host_exec import HostExecPolicy, HostExecRequest


def response(status: str, **extra: Any) -> dict[str, Any]:
    payload = {
        "schema": "cynic.host_exec.response.v1",
        "status": status,
    }
    payload.update(extra)
    return payload


def load_policy(
    path: Path | None,
    *,
    default_policy_paths: list[Path],
) -> HostExecPolicy:
    candidates = []
    if path is not None:
        candidates.append(path)
    env_path = os.environ.get("CYNIC_HOST_EXEC_POLICY")
    if env_path:
        candidates.append(Path(env_path))
    candidates.extend(default_policy_paths)

    for candidate in candidates:
        if candidate.exists():
            return HostExecPolicy(json_object_or_default(candidate.read_text(encoding="utf-8"), {}))
    raise FileNotFoundError("no host exec policy found")


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


def run_request(
    request_payload: dict[str, Any],
    *,
    policy: HostExecPolicy,
    declared_services: set[str],
    dry_run: bool = False,
    runner=subprocess.run,
    geteuid=os.geteuid,
) -> dict[str, Any]:
    request = HostExecRequest.from_json(request_payload)
    validation = policy.validate_request(request, declared_services)
    if not validation.ok:
        return response(
            "rejected",
            violations=list(validation.violations),
            node=request.node,
            role=request.role,
            purpose=request.purpose,
        )

    command = list(request.argv)
    if dry_run:
        return response(
            "dry_run",
            node=request.node,
            role=request.role,
            purpose=request.purpose,
            command=command,
        )

    if geteuid() != 0:
        return response(
            "rejected",
            violations=["helper_must_run_as_root"],
            node=request.node,
            role=request.role,
            purpose=request.purpose,
        )

    completed = runner(
        command,
        check=False,
        text=True,
        capture_output=True,
        timeout=request.timeout_seconds,
    )
    return response(
        "completed",
        node=request.node,
        role=request.role,
        purpose=request.purpose,
        exit_code=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
        command=command,
    )
