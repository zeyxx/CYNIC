from __future__ import annotations

import os
import socket
from typing import Any, Iterable, Mapping, Sequence


def _normalize_host(value: Any) -> str:
    text = str(value).strip()
    if not text:
        return ""
    return text


def _collect_unique(values: Iterable[Any]) -> list[str]:
    normalized = (_normalize_host(item) for item in values)
    return sorted({value for value in normalized if value})


def runtime_host_candidates() -> list[str]:
    return _collect_unique(
        (os.uname().nodename, socket.gethostname(), socket.getfqdn())
    )


def _build_runner_allowed_hosts(build_target: str, build_node: Mapping[str, Any] | None = None) -> list[str]:
    aliases: list[Any] = []
    if build_node:
        aliases.append(build_node.get("name", ""))
        aliases.extend(build_node.get("aliases", []) or [])
    aliases.append(build_target)
    return _collect_unique(aliases)


def build_runner_host_guard(
    expected_build_target: str,
    build_node: Mapping[str, Any] | None = None,
    *,
    observed_hosts: Sequence[str] | None = None,
    registry_build_target: str = "",
    intent: str | None = None,
    task_domain: str | None = None,
) -> dict[str, Any] | None:
    allowed_hosts = _build_runner_allowed_hosts(expected_build_target, build_node)
    if not allowed_hosts:
        return None

    observed = _collect_unique(observed_hosts or runtime_host_candidates())
    if any(host in allowed_hosts for host in observed):
        return None

    return {
        "status": "blocked",
        "error_kind": "lifecycle_execution_host_mismatch",
        "error": (
            "organ-forge refused lifecycle intent execution because this daemon is not "
            "running on the registry-selected build runner"
        ),
        "intent": intent,
        "task_domain": task_domain,
        "expected_build_target": expected_build_target,
        "registry_build_target": registry_build_target,
        "allowed_execution_hosts": allowed_hosts,
        "observed_execution_hosts": observed,
    }
