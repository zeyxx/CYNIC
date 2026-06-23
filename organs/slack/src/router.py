"""
organ-slack — Router
Resolves a logical routing key (e.g. "stage_privacy_ethereum") to a concrete
Slack channel ID by reading the registry.json routing table and mapping each
value to an environment variable.

Design principle: zero hard-coded channel IDs. Every ID lives in .cynic-env.
"""

from __future__ import annotations

import json
import os
from pathlib import Path


REGISTRY_PATH = Path(__file__).resolve().parents[3] / "infra" / "registry.json"


def _load_routing_table() -> dict[str, str]:
    """Return the slack_routing block from registry.json."""
    with REGISTRY_PATH.open() as f:
        registry = json.load(f)
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


def list_routes() -> dict[str, str]:
    """Return routing_key → resolved_channel_id (or UNRESOLVED) for diagnostics."""
    table = _load_routing_table()
    result = {}
    for key, env_var in table.items():
        channel = os.environ.get(env_var, "")
        result[key] = channel if channel else f"UNRESOLVED ({env_var})"
    return result
