"""Core scanning service for organ-radar."""

from __future__ import annotations

from dataclasses import asdict

from src.adapters.github_sensor import GitHubSensorAdapter
from src.adapters.http_sensor import HttpSensorAdapter
from src.domain.entities import Target, TargetType


DEFAULT_TARGETS = {
    "hermes-agent": Target(
        id="hermes-agent",
        target_type=TargetType.GITHUB_REPO,
        uri="https://github.com/nousresearch/hermes-agent",
        metadata={
            "description": "Hermes Agent CLI - autonomous cortex agent",
            "current_version": None,
            "branch": "main",
        },
    ),
    "cynic-kernel": Target(
        id="cynic-kernel",
        target_type=TargetType.GITHUB_REPO,
        uri="https://github.com/zeyxx/CYNIC",
        metadata={
            "description": "CYNIC Kernel - multi-agent coordination kernel",
            "current_version": None,
            "branch": "main",
        },
    ),
    "holdex-live": Target(
        id="partner_holdex_live",
        target_type=TargetType.API_ENDPOINT,
        uri="https://holdex.live",
        metadata={"description": "Holdex UI Frontend"},
    ),
}


def sensor_for(target: Target):
    """Route to the correct sensor adapter based on target type."""
    if target.target_type == TargetType.GITHUB_REPO:
        return GitHubSensorAdapter()
    if target.target_type == TargetType.API_ENDPOINT:
        return HttpSensorAdapter()
    raise ValueError(f"No sensor adapter for {target.target_type}")


def observation_to_result(observation) -> dict:
    """Normalize a sensor observation for CLI/kernel ingestion."""
    result = {
        "target_id": observation.target_id,
        "status": observation.status.value if hasattr(observation.status, "value") else str(observation.status),
        "timestamp": observation.timestamp.isoformat(),
        "latency_ms": round(observation.latency_ms, 2) if observation.latency_ms else None,
        "current_version": observation.current_version,
        "latest_version": observation.latest_version,
        "release_info": None,
        "pending_count": len(observation.pending_releases),
    }

    if observation.release_info:
        result["release_info"] = {
            "version": observation.release_info.version,
            "tag": observation.release_info.tag,
            "published_at": observation.release_info.published_at,
            "html_url": observation.release_info.html_url,
        }

    return result


async def scan_target(target: Target, verbose: bool = False) -> dict:
    """Scan a single target and return a structured result."""
    sensor = sensor_for(target)
    if verbose:
        print(f"[*] Scanning: {target.id} ({target.uri})")
    return observation_to_result(await sensor.perceive(target))


def serialize_observation(obs) -> str:
    """Serialize an observation for JSON output."""
    import json

    def _default(obj):
        if hasattr(obj, "isoformat"):
            return obj.isoformat()
        if hasattr(obj, "value"):
            return obj.value
        return str(obj)

    return json.dumps(asdict(obs), default=_default, indent=2)
