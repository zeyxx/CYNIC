"""Pure architecture-pattern extraction from framework reports."""

from __future__ import annotations

from typing import Any


FRAMEWORK_ARCHITECTURE_PATTERNS_SCHEMA = "cynic.framework_architecture_patterns.v1"


def architecture_patterns_from_reports(
    *,
    framework: str,
    capabilities: dict[str, Any],
    dependencies: dict[str, Any],
) -> dict[str, Any]:
    source_summary = capabilities.get("source_summary") if isinstance(capabilities.get("source_summary"), dict) else {}
    top_dirs = set(source_summary.get("top_level_directories", []))
    dependency_names = {str(item.get("name", "")) for item in dependencies.get("dependencies", []) if isinstance(item, dict)}
    capability_ids = {str(item.get("id", "")) for item in capabilities.get("capabilities", []) if isinstance(item, dict)}

    patterns: list[dict[str, Any]] = []
    if {"agent", "tools", "providers"} & top_dirs:
        patterns.append(
            {
                "id": "tool_calling_agent_core",
                "confidence": "observed",
                "evidence": sorted({"agent", "tools", "providers"} & top_dirs),
            }
        )
    if {"gateway", "plugins"} & top_dirs or "messaging_gateway" in capability_ids:
        patterns.append(
            {
                "id": "multi_transport_gateway",
                "confidence": "observed" if {"gateway", "plugins"} & top_dirs else "inferred",
                "evidence": sorted(({"gateway", "plugins"} & top_dirs) | ({"messaging_gateway"} & capability_ids)),
            }
        )
    if {"skills", "optional-skills"} & top_dirs or "skill_lifecycle" in capability_ids:
        patterns.append(
            {
                "id": "skill_lifecycle_loop",
                "confidence": "observed" if {"skills", "optional-skills"} & top_dirs else "inferred",
                "evidence": sorted(({"skills", "optional-skills"} & top_dirs) | ({"skill_lifecycle"} & capability_ids)),
            }
        )
    if {"cron", "mcp", "fastapi", "uvicorn"} & dependency_names or "scheduled_automation" in capability_ids:
        patterns.append(
            {
                "id": "background_scheduler_with_api_surfaces",
                "confidence": "inferred",
                "evidence": sorted(({"cron", "mcp", "fastapi", "uvicorn"} & dependency_names) | ({"scheduled_automation"} & capability_ids)),
            }
        )
    if {"modal", "daytona", "docker"} & dependency_names or "terminal_backend_multiplexing" in capability_ids:
        patterns.append(
            {
                "id": "pluggable_execution_backends",
                "confidence": "inferred",
                "evidence": sorted(({"modal", "daytona", "docker"} & dependency_names) | ({"terminal_backend_multiplexing"} & capability_ids)),
            }
        )

    return {
        "schema": FRAMEWORK_ARCHITECTURE_PATTERNS_SCHEMA,
        "framework": framework,
        "pattern_count": len(patterns),
        "patterns": patterns,
    }
