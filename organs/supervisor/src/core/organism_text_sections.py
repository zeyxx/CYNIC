"""Shared text sections for organism operator surfaces."""

from __future__ import annotations


def append_topology_summary(lines: list[str], summary: object, *, title: str) -> None:
    if not isinstance(summary, dict):
        return
    lines.append(title)
    for key in (
        "domain_topologies",
        "kernel_surfaces",
        "node_role_topologies",
        "async_muscles",
        "supply_chains",
        "recursion_contracts",
    ):
        if key in summary:
            lines.append(f"  - {key}: {summary.get(key)}")
    unhealthy = summary.get("unhealthy_by_layer")
    if isinstance(unhealthy, dict) and unhealthy:
        lines.append(
            "  - unhealthy_by_layer: "
            + ", ".join(f"{layer}={count}" for layer, count in sorted(unhealthy.items()))
        )


def append_pattern_evolution(lines: list[str], evolution: object) -> None:
    if not isinstance(evolution, dict):
        return
    lines.append("Pattern evolution:")
    by_severity = evolution.get("by_severity")
    if isinstance(by_severity, dict):
        lines.append(
            "  - by_severity: "
            + ", ".join(f"{key}={value}" for key, value in sorted(by_severity.items()))
        )
    family_index = evolution.get("family_index")
    if isinstance(family_index, dict) and family_index:
        lines.append("  - families:")
        for family, ids in list(sorted(family_index.items()))[:6]:
            if isinstance(ids, list):
                lines.append(f"    - {family}: {len(ids)}")
    blocking = evolution.get("blocking_ids")
    if isinstance(blocking, list) and blocking:
        lines.append("  - blocking:")
        for item in blocking[:6]:
            lines.append(f"    - {item}")
    dispatched = evolution.get("dispatchable_pattern_ids")
    if isinstance(dispatched, list) and dispatched:
        lines.append("  - dispatchable:")
        for item in dispatched[:6]:
            lines.append(f"    - {item}")
    informational = evolution.get("informational_ids")
    if isinstance(informational, list) and informational:
        lines.append("  - informational:")
        for item in informational[:4]:
            lines.append(f"    - {item}")
