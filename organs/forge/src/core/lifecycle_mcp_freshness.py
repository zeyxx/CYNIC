"""Pure MCP freshness planning for organ-forge lifecycle."""

from __future__ import annotations

from datetime import datetime, timezone

from .lifecycle_runtime_artifacts import build_runtime_artifact_manifest


MCP_FRESHNESS_PLAN_SCHEMA = "cynic.lifecycle.mcp_freshness_plan.v1"


def build_mcp_freshness_plan(
    registry: dict,
    freshness_report: dict,
    *,
    pending_duplicate: bool = False,
    now: datetime | None = None,
) -> dict:
    lifecycle = registry.get("lifecycle", {}).get("kernel_release", {})
    controller = lifecycle.get("mcp_freshness_controller", {})
    if controller.get("schema") != "cynic.mcp_freshness_controller.v1":
        raise RuntimeError("missing lifecycle.kernel_release.mcp_freshness_controller")

    status = str(freshness_report.get("status", "unknown"))
    stale_statuses = set(controller.get("stale_statuses", []))
    observed_at = (now or datetime.now(timezone.utc)).replace(microsecond=0).isoformat()
    artifact_manifest = build_runtime_artifact_manifest(registry, now="mcp-freshness")
    mcp_artifact = next(
        (
            artifact
            for artifact in artifact_manifest["artifacts"]
            if artifact["name"] == "cynic-mcp"
        ),
        None,
    )
    base = {
        "schema": MCP_FRESHNESS_PLAN_SCHEMA,
        "status": "ok",
        "observed_at": observed_at,
        "artifact": mcp_artifact,
        "freshness_status": status,
        "controller_status": controller.get("status", "planned"),
        "profile": controller.get("dispatch_profile", "mcp-reload"),
        "consumer": controller.get("consumer", "organ-forge-hermes-agent"),
        "check_command": controller.get("check_command"),
        "trigger_sources": controller.get("trigger_sources", []),
        "proof_requirements": controller.get("proof_requirements", []),
    }
    if status == "fresh":
        return {
            **base,
            "action": "no_action",
            "reason": "mcp surface is fresh",
            "dispatch": None,
        }
    if pending_duplicate:
        return {
            **base,
            "action": "observe_existing_request",
            "reason": "mcp refresh already pending or recently dispatched",
            "dispatch": None,
            "debounce_seconds": controller.get("debounce_seconds", 900),
        }
    if status in stale_statuses:
        return {
            **base,
            "action": "dispatch_lifecycle_profile",
            "reason": f"mcp surface status is {status}",
            "dispatch": {
                "profile": controller.get("dispatch_profile", "mcp-reload"),
                "tool": controller.get("dispatch_tool", "cynic_lifecycle_dispatch"),
                "fallback_command": controller.get("fallback_command", "cynic mcp-reload"),
                "legacy_fallback_command": controller.get(
                    "legacy_fallback_command",
                    "make mcp-reload-request",
                ),
                "kind": lifecycle.get("task_kind", "hermes"),
                "domain": lifecycle.get("task_domain", "organ-forge"),
                "consumer": controller.get("consumer", "organ-forge-hermes-agent"),
            },
            "debounce_seconds": controller.get("debounce_seconds", 900),
        }
    return {
        **base,
        "status": "blocked",
        "action": "observe_unknown_status",
        "reason": f"mcp freshness status {status!r} is not actionable",
        "dispatch": None,
    }
