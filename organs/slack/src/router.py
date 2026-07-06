"""
organ-slack — Router
Resolves a logical routing key (e.g. "stage_privacy_ethereum") to a concrete
Slack channel ID by reading the registry.json routing table and mapping each
value to an environment variable.

Design principle: zero hard-coded channel IDs. Every ID lives in .cynic-env.
"""

from __future__ import annotations

import os
import re
from pathlib import Path

from organs.contracts.src.surface_contracts import read_json_file_or_default


def _project_root() -> Path:
    """Resolve project root with env override and deterministic fallback."""
    root = os.environ.get("CYNIC_PROJECT_ROOT") or os.environ.get("CYNIC_REPO_ROOT")
    if root:
        return Path(root).expanduser().resolve()

    start = Path(__file__).resolve()
    for candidate in [start, *start.parents]:
        if (candidate / "infra").is_dir() and (candidate / "organs").is_dir() and (candidate / ".git").exists():
            return candidate

    return Path(__file__).resolve().parent


def _registry_path() -> Path:
    """Resolve the registry path used by the Slack adapter."""
    override = os.environ.get("CYNIC_REGISTRY_PATH")
    if override:
        return Path(override)
    return _project_root() / "infra" / "registry.json"


def _load_routing_table() -> dict[str, str]:
    """Return the slack_routing block from registry.json."""
    registry = read_json_file_or_default(_registry_path(), {})
    return registry.get("routing_rules", {}).get("slack_routing", {})


def resolve_channel(routing_key: str) -> str:
    """
    Resolve a routing key to a Slack channel ID.

    Resolution order:
    1. Look up routing_key in registry slack_routing table → get env var name.
    2. Read that env var from the environment.
    3. Fallback: SLACK_CHANNEL_KERNEL (always defined in .cynic-env).

    Raises RuntimeError if no channel can be resolved at all.
    """
    table = _load_routing_table()
    env_var = table.get(routing_key)

    if env_var:
        channel = os.environ.get(env_var, "")
        if channel:
            return channel

    # Fallback to kernel ops channel
    fallback = os.environ.get("SLACK_CHANNEL_KERNEL", "")
    if fallback:
        return fallback

    raise RuntimeError(
        f"organ-slack: cannot resolve channel for routing_key={routing_key!r}. "
        "Ensure SLACK_CHANNEL_KERNEL is set in .cynic-env."
    )


def resolve_coordination_channel(
    project: str,
    workstream: str | None = None,
    thread_id: str | None = None,
    fallback_key: str | None = None,
) -> str:
    """
    Resolve a coordination route with context-driven fallbacks.
    """
    route_candidates: list[str] = []
    if fallback_key:
        route_candidates.append(fallback_key)
    if workstream:
        route_candidates.extend(
            [
                f"{project}:{workstream}",
                f"{project}_{workstream}",
                f"{_normalize_route_key(project)}_{_normalize_route_key(workstream)}",
            ]
        )
    route_candidates.append(_normalize_route_key(project))
    if thread_id:
        route_candidates.append(_normalize_route_key(thread_id))

    seen = set()
    for route_key in route_candidates:
        if route_key in seen:
            continue
        seen.add(route_key)
        try:
            return resolve_channel(route_key)
        except RuntimeError:
            continue

    raise RuntimeError(
        f"organ-slack: cannot resolve coordination channel for project={project!r}, "
        f"workstream={workstream!r}, thread_id={thread_id!r}"
    )


def _normalize_route_key(value: str) -> str:
    return re.sub(r"[^A-Z0-9_]", "_", value.upper())


def list_routes() -> dict[str, str]:
    """Return routing_key → resolved_channel_id (or UNRESOLVED) for diagnostics."""
    table = _load_routing_table()
    result = {}
    for key, env_var in table.items():
        channel = os.environ.get(env_var, "")
        result[key] = channel if channel else f"UNRESOLVED ({env_var})"
    return result
