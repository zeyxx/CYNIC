from __future__ import annotations

from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import read_json_file_or_default
from organs.supervisor.src.core.organism_stack_actions import (
    convergence_actions,
    dispatchable_requests,
)
from organs.supervisor.src.core.organism_stack_constants import ASYNC_PRIMITIVES, LAYERS
from organs.supervisor.src.core.organism_stack_fragmentation import fragmentation_patterns as _fragmentation_patterns
from organs.supervisor.src.core.organism_stack_patterns import (
    annotate_patterns as _annotate_patterns,
    attach_pattern_causes as _attach_pattern_causes,
    pattern_evolution as _pattern_evolution,
)
from organs.supervisor.src.core.organism_stack_layers import (
    circulation_entities as _circulation_entities,
    cortex_entities as _cortex_entities,
    membrane_entities as _membrane_entities,
    recursion_entities as _recursion_entities,
    soma_entities as _soma_entities,
)
from organs.supervisor.src.core.organism_stack_organs import (
    organ_entities as _organ_entities,
    organ_manifests as _organ_manifests,
)
from organs.supervisor.src.core.organism_stack_runtime import cell_evidence as _cell_evidence
from organs.supervisor.src.core.organism_stack_topology import (
    async_muscle_entities as _async_muscle_entities,
    domain_topology_entities as _domain_topology_entities,
    kernel_surface_entities as _kernel_surface_entities,
    node_entities as _node_entities,
    node_role_topology_entities as _node_role_topology_entities,
    supply_chain_topology_entities as _supply_chain_topology_entities,
    synapse_entities as _synapse_entities,
    topology_summary as _topology_summary,
)
def organism_stack_report(project_root: object, runtime: dict[str, object] | None = None) -> dict[str, object]:
    root = Path(str(project_root)).resolve()
    registry = _read_json_file(root / "infra" / "registry.json")
    manifests = _organ_manifests(root)
    cells = _cell_evidence(root)
    entities: list[dict[str, object]] = []
    entities.extend(_node_entities(registry))
    entities.extend(_node_role_topology_entities(registry))
    entities.extend(_membrane_entities(runtime or {}))
    entities.extend(_synapse_entities(registry))
    entities.extend(_kernel_surface_entities(root))
    entities.extend(cells)
    entities.extend(_organ_entities(manifests))
    entities.extend(_domain_topology_entities(manifests))
    entities.extend(_circulation_entities())
    entities.extend(_async_muscle_entities(root, registry))
    entities.extend(_supply_chain_topology_entities(registry))
    entities.extend(_soma_entities(manifests))
    entities.extend(_recursion_entities())
    entities.extend(_cortex_entities(root))
    patterns = _annotate_patterns(_fragmentation_patterns(root, registry, manifests, cells, entities))
    actions = convergence_actions(entities, patterns)
    _attach_pattern_causes(actions, patterns)
    requests = dispatchable_requests(actions)
    blockers = [item for item in entities if item.get("health") in {"blocked", "degraded"}]
    return {
        "schema": "cynic.organism.stack.v1",
        "mutates": False,
        "async_primitives": ASYNC_PRIMITIVES,
        "recursion_rule": "Every layer exposes desired, observed, health, owner, next_action and proof so the layer above can converge it.",
        "layer_count": len(LAYERS),
        "layers": LAYERS,
        "entity_count": len(entities),
        "topology_summary": _topology_summary(entities),
        "entities": entities,
        "blockers": blockers,
        "patterns": patterns,
        "pattern_evolution": _pattern_evolution(patterns, actions, requests),
        "actions": actions,
        "dispatchable_requests": requests,
        "status": "blocked" if any(item.get("severity") == "critical" for item in patterns) or blockers else "ok",
    }
def _read_json_file(path: Path) -> dict[str, Any]:
    payload = read_json_file_or_default(path, {})
    return payload if isinstance(payload, dict) else {}
