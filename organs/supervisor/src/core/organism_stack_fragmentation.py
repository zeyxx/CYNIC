"""Fragmentation pattern detection for the L0-L7 organism stack."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from organs.supervisor.src.core.organism_stack_constants import LAYERS
from organs.supervisor.src.core.organism_stack_organs import organ_shape_fragmentation_patterns
from organs.supervisor.src.core.organism_stack_topology import (
    async_readback_gaps,
    domains_without_primary,
    duplicate_domains,
    kernel_http_membrane_fragmentation,
    lifecycle_target_gaps,
    mempool_contract_fragmentation,
    observe_contract_fragmentation,
)
from organs.supervisor.src.core.organism_stack_utils import (
    count_files,
    duplicates,
    utility_function_duplicates,
)


def fragmentation_patterns(
    root: Path,
    registry: dict[str, Any],
    manifests: list[dict[str, object]],
    cells: list[dict[str, object]],
    entities: list[dict[str, object]],
) -> list[dict[str, object]]:
    patterns: list[dict[str, object]] = []
    duplicate_organs = duplicates([str(item.get("name")) for item in manifests])
    if duplicate_organs:
        patterns.append(
            {
                "pattern": "duplicate_organ_identity",
                "severity": "critical",
                "entities": duplicate_organs,
                "resolution": "merge or rename manifests so every organ identity is unique",
            }
        )
    duplicate_functions = duplicate_domains(manifests)
    if duplicate_functions:
        patterns.append(
            {
                "pattern": "overlapping_organ_domain",
                "severity": "warning",
                "entities": duplicate_functions,
                "resolution": "declare one owner and adapters/consumers for the rest",
            }
        )
    target_gaps = lifecycle_target_gaps(registry)
    if target_gaps:
        patterns.append(
            {
                "pattern": "lifecycle_supply_chain_topology_gap",
                "severity": "warning",
                "entities": target_gaps,
                "resolution": "align lifecycle build/deploy roles, deploy target flag and required node statuses in infra/registry.json",
            }
        )
    missing_primary = domains_without_primary(manifests)
    if missing_primary:
        patterns.append(
            {
                "pattern": "domain_without_primary_owner",
                "severity": "warning",
                "entities": missing_primary,
                "resolution": "declare exactly one primary organ per domain and downgrade the rest to sensor/consumer/adapter",
            }
        )
    patterns.extend(organ_shape_fragmentation_patterns(manifests))
    utility_duplicates = utility_function_duplicates(root)
    if utility_duplicates:
        patterns.append(
            {
                "pattern": "utility_duplicate_pattern",
                "severity": "warning",
                "entities": utility_duplicates,
                "count": len(utility_duplicates),
                "resolution": "extract duplicated utility primitives into an organ adapter or shared contract port",
            }
        )
    legacy_scripts = count_files(root / "scripts")
    if legacy_scripts:
        patterns.append(
            {
                "pattern": "legacy_script_muscle_fragmentation",
                "severity": "warning",
                "count": legacy_scripts,
                "resolution": "move actions behind organ entrypoints, MCP tools or infra wrappers",
            }
        )
    async_gaps = async_readback_gaps(root)
    if async_gaps:
        patterns.append(
            {
                "pattern": "async_muscle_without_readback",
                "severity": "warning",
                "entities": async_gaps[:24],
                "count": len(async_gaps),
                "resolution": "route async dispatch through lifecycle/agent-task ports that prove pending or completed read-back",
            }
        )
    mempool_fragments = mempool_contract_fragmentation(root)
    if mempool_fragments:
        patterns.append(
            {
                "pattern": "mempool_contract_fragmentation",
                "severity": "warning",
                "entities": mempool_fragments[:24],
                "count": len(mempool_fragments),
                "resolution": "route /agent-tasks clients through organs.contracts agent-task ports or a declared kernel task adapter",
            }
        )
    kernel_http_fragments = kernel_http_membrane_fragmentation(root)
    if kernel_http_fragments:
        patterns.append(
            {
                "pattern": "kernel_http_membrane_fragmentation",
                "severity": "warning",
                "entities": kernel_http_fragments[:24],
                "count": len(kernel_http_fragments),
                "resolution": "route Kernel URL normalization, loopback policy and auth headers through organs.contracts kernel_http helpers",
            }
        )
    degraded_surfaces = [
        str(entity.get("entity"))
        for entity in entities
        if entity.get("kind") == "kernel_surface_topology" and entity.get("health") == "degraded"
    ]
    if degraded_surfaces:
        patterns.append(
            {
                "pattern": "kernel_surface_without_contract_proof",
                "severity": "warning",
                "entities": degraded_surfaces,
                "resolution": "attach every Kernel surface to a shared contract adapter and expose proof files in the L3 topology",
            }
        )
    observe_fragments = observe_contract_fragmentation(root)
    if observe_fragments:
        patterns.append(
            {
                "pattern": "observe_contract_fragmentation",
                "severity": "warning",
                "entities": observe_fragments[:24],
                "count": len(observe_fragments),
                "resolution": "route /observe producers through organs.contracts kernel_observe or an organ adapter that delegates to it",
            }
        )
    for cell in cells:
        if cell.get("health") in {"blocked", "degraded"}:
            patterns.append(
                {
                    "pattern": "cell_runtime_gap",
                    "severity": "critical" if cell.get("entity") == "cynic-mcp" else "warning",
                    "entities": [cell.get("entity")],
                    "observed": cell.get("observed"),
                    "resolution": cell.get("next_action"),
                }
            )
    missing_layers = _missing_stack_layers(entities)
    if missing_layers:
        patterns.append(
            {
                "pattern": "missing_stack_layer_evidence",
                "severity": "critical",
                "entities": missing_layers,
                "resolution": "add observed entity evidence for every L0-L7 layer",
            }
        )
    return patterns


def _missing_stack_layers(entities: list[dict[str, object]]) -> list[str]:
    layer_counts: dict[str, int] = {}
    for entity in entities:
        layer = str(entity.get("layer"))
        layer_counts[layer] = layer_counts.get(layer, 0) + 1
    return [layer["id"] for layer in LAYERS if layer_counts.get(layer["id"], 0) == 0]
