"""Pure CYNIC compatibility mapping for external frameworks."""

from __future__ import annotations

from typing import Any


FRAMEWORK_CYNIC_COMPATIBILITY_SCHEMA = "cynic.framework_cynic_compatibility.v1"


def compatibility_from_reports(
    *,
    framework: str,
    capabilities: dict[str, Any],
    dependencies: dict[str, Any],
    architecture_patterns: dict[str, Any],
) -> dict[str, Any]:
    capability_ids = {str(item.get("id", "")) for item in capabilities.get("capabilities", []) if isinstance(item, dict)}
    pattern_ids = {str(item.get("id", "")) for item in architecture_patterns.get("patterns", []) if isinstance(item, dict)}
    dependency_count = int(dependencies.get("dependency_count", 0) or 0)
    coverage_gaps = []
    coverage_gaps.extend(item for item in capabilities.get("coverage_gaps", []) if isinstance(item, dict))
    coverage_gaps.extend(item for item in dependencies.get("coverage_gaps", []) if isinstance(item, dict))

    compatible_surfaces = []
    if "agent_task_execution" in capability_ids:
        compatible_surfaces.append("kernel_agent_tasks")
    if "repo_review" in capability_ids:
        compatible_surfaces.append("git_matter_review_provider")
    if "skill_lifecycle" in capability_ids:
        compatible_surfaces.append("cynic_skill_observation")
    if "external_dependency_inventory" in pattern_ids or dependency_count:
        compatible_surfaces.append("dependency_risk_mapping")

    risk_flags = []
    if coverage_gaps:
        risk_flags.append("coverage_incomplete")
    if dependency_count > 100:
        risk_flags.append("large_dependency_surface")
    if "multi_transport_gateway" in pattern_ids:
        risk_flags.append("multi_transport_security_boundary")

    return {
        "schema": FRAMEWORK_CYNIC_COMPATIBILITY_SCHEMA,
        "framework": framework,
        "compatible_surface_count": len(compatible_surfaces),
        "compatible_surfaces": sorted(compatible_surfaces),
        "risk_flags": sorted(risk_flags),
        "coverage_gap_count": len(coverage_gaps),
        "coverage_gaps": coverage_gaps,
    }
