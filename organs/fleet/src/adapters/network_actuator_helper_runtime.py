"""Runtime adapter for the root-owned network actuator helper."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import json_object_or_default
from organs.fleet.src.core.network_actuator import NetworkActuatorPolicy, NetworkActuatorRequest


def response(status: str, **extra: Any) -> dict[str, Any]:
    payload = {
        "schema": "cynic.network_actuator.response.v1",
        "status": status,
    }
    payload.update(extra)
    return payload


def load_policy(
    path: Path | None,
    *,
    default_policy_paths: list[Path],
) -> NetworkActuatorPolicy:
    candidates = []
    if path is not None:
        candidates.append(path)
    env_path = os.environ.get("CYNIC_NETWORK_ACTUATOR_POLICY")
    if env_path:
        candidates.append(Path(env_path))
    candidates.extend(default_policy_paths)

    for candidate in candidates:
        if candidate.exists():
            return NetworkActuatorPolicy(json_object_or_default(candidate.read_text(encoding="utf-8"), {}))
    raise FileNotFoundError("no network actuator policy found")


def run_request(
    request_payload: dict[str, Any],
    *,
    policy: NetworkActuatorPolicy,
    dry_run: bool = False,
    runner=subprocess.run,
    geteuid=os.geteuid,
) -> dict[str, Any]:
    request = NetworkActuatorRequest.from_json(request_payload)
    validation = policy.validate_request(request, dry_run=dry_run)
    if not validation.ok:
        return response(
            "rejected",
            violations=list(validation.violations),
            profile=request.profile,
        )

    commands = [list(command) for command in request.commands]
    if dry_run:
        return response(
            "dry_run",
            profile=request.profile,
            commands=commands,
            apply=request.apply,
        )

    if geteuid() != 0:
        return response(
            "rejected",
            violations=["helper_must_run_as_root"],
            profile=request.profile,
        )

    results = []
    for command in commands:
        completed = runner(
            command,
            check=False,
            text=True,
            capture_output=True,
            timeout=request.timeout_seconds,
        )
        results.append(
            {
                "command": command,
                "exit_code": completed.returncode,
                "stdout": completed.stdout,
                "stderr": completed.stderr,
            }
        )
        if completed.returncode != 0:
            return response(
                "completed_with_failures",
                profile=request.profile,
                results=results,
            )

    return response(
        "completed",
        profile=request.profile,
        results=results,
    )
