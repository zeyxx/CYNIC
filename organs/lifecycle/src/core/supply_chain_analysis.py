from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Set

from .ports import SupplyChainAnalysisPort


def _unique(items: Iterable[str]) -> list[str]:
    seen: list[str] = []
    for item in items:
        if item and item not in seen:
            seen.append(item)
    return seen


def _normalize_names(collection: Any) -> list[dict]:
    if isinstance(collection, list):
        return [item for item in collection if isinstance(item, dict)]
    if isinstance(collection, dict):
        return [item for item in collection.values() if isinstance(item, dict)]
    return []


@dataclass
class SupplyChainGraph:
    nodes: list[dict]
    edges: list[dict]
    known_targets: set[str]


def build_supply_chain_graph(registry: dict) -> SupplyChainGraph:
    lifecycle = registry.get("lifecycle", {}).get("kernel_release", {})
    nodes = _normalize_names(registry.get("nodes")) + _normalize_names(registry.get("organs"))
    service_names: list[str] = []
    for node in nodes:
        for service in node.get("services", []) or []:
            if isinstance(service, dict):
                service_name = service.get("name")
                if isinstance(service_name, str) and service_name:
                    service_names.append(service_name)
    for requirement in lifecycle.get("activation_requirements", {}).values():
        service_names.extend(requirement.get("services", []))
    for service_name in registry.get("lifecycle", {}).get("kernel_release", {}).get("repo_targets", []):
        service_names.append(service_name)
    for service_name in registry.get("service_placement", {}).keys():
        service_names.append(service_name)

    known_targets: set[str] = {
        node.get("name")
        for node in nodes
        if node.get("name")
    }
    known_targets.update(
        item
        for item in service_names
        if isinstance(item, str) and item
    )
    known_targets.update(
        {
            lifecycle.get("orchestrator_organ", ""),
            lifecycle.get("build_runner_role", ""),
            lifecycle.get("deploy_target_role", ""),
            lifecycle.get("task_kind", ""),
            lifecycle.get("task_domain", ""),
        }
    )
    known_targets.discard("")

    edges: list[dict] = []
    for source in nodes:
        source_name = source.get("name")
        if not source_name:
            continue
        for target in source.get("depends_on", []) or []:
            if not isinstance(target, str) or not target:
                continue
            edges.append(
                {
                    "source": source_name,
                    "target": target,
                    "resolved": target in known_targets,
                }
            )

    return SupplyChainGraph(nodes=nodes, edges=edges, known_targets=known_targets)


def _adjacency(graph: SupplyChainGraph) -> dict[str, list[str]]:
    adjacency: dict[str, list[str]] = {}
    for edge in graph.edges:
        if not edge.get("resolved"):
            continue
        adjacency.setdefault(edge["source"], []).append(edge["target"])
    return adjacency


def _find_cycles(adjacency: dict[str, list[str]]) -> list[list[str]]:
    cycles: list[list[str]] = []
    visiting: set[str] = set()
    visited: set[str] = set()
    stack: list[str] = []

    def dfs(node: str) -> None:
        if node in visiting:
            if node in stack:
                cycle = stack[stack.index(node) :] + [node]
                cycles.append(_unique(cycle))
            return
        if node in visited:
            return
        visiting.add(node)
        stack.append(node)
        for neighbor in adjacency.get(node, []):
            dfs(neighbor)
        stack.pop()
        visiting.remove(node)
        visited.add(node)

    for node in adjacency:
        dfs(node)
    unique_cycles: list[list[str]] = []
    seen_signatures: set[tuple[str, ...]] = set()
    for cycle in cycles:
        signature = tuple(cycle)
        if signature not in seen_signatures:
            seen_signatures.add(signature)
            unique_cycles.append(cycle)
    return unique_cycles


def _topological_order(adjacency: dict[str, list[str]], nodes: Iterable[str] | None = None) -> list[str]:
    in_degree: dict[str, int] = {node: 0 for node in (nodes or []) if node}
    for source, targets in adjacency.items():
        in_degree.setdefault(source, 0)
        for target in targets:
            in_degree[target] = in_degree.get(target, 0) + 1
    queue = [node for node, degree in in_degree.items() if degree == 0]
    order: list[str] = []
    while queue:
        node = queue.pop(0)
        order.append(node)
        for neighbor in adjacency.get(node, []):
            in_degree[neighbor] -= 1
            if in_degree[neighbor] == 0:
                queue.append(neighbor)
    return order


class RegistrySupplyChainAnalyzer(SupplyChainAnalysisPort):
    """Pure registry-backed analyzer for supply-chain and execution dependency hygiene."""

    def __init__(self, registry_provider):
        self.registry_provider = registry_provider

    def analyze_supply_chain(self) -> Dict[str, Any]:
        registry = self.registry_provider.load_registry()
        lifecycle = registry.get("lifecycle", {}).get("kernel_release", {})
        graph = build_supply_chain_graph(registry)
        adjacency = _adjacency(graph)
        cycles = _find_cycles(adjacency)
        unresolved = [edge for edge in graph.edges if not edge.get("resolved")]
        execution_layers = lifecycle.get("workflow", {}).get("state_layers", {})
        canonical_stages = lifecycle.get("workflow", {}).get("canonical_stages", [])
        graph_nodes = [
            node.get("name")
            for node in graph.nodes
            if isinstance(node, dict) and node.get("name")
        ]
        stage_edges = [
            {
                "source": stage.get("name"),
                "target": (
                    canonical_stages[index + 1].get("name")
                    if index + 1 < len(canonical_stages)
                    and isinstance(canonical_stages[index + 1], dict)
                    else None
                ),
            }
            for index, stage in enumerate(canonical_stages)
            if isinstance(stage, dict)
        ]
        return {
            "status": "blocked" if cycles or unresolved else "ok",
            "summary": {
                "nodes": len(graph.nodes),
                "edges": len(graph.edges),
                "resolved_edges": len(graph.edges) - len(unresolved),
                "unresolved_edges": len(unresolved),
                "cycle_count": len(cycles),
            },
            "graph": {
                "nodes": graph.nodes,
                "edges": graph.edges,
                "adjacency": adjacency,
                "topological_order": _topological_order(adjacency, graph_nodes),
            },
            "cycles": cycles,
            "unresolved_dependencies": unresolved,
            "execution": {
                "state_layers": execution_layers,
                "canonical_stages": canonical_stages,
                "stage_edges": stage_edges,
                "request_profiles": lifecycle.get("request_profiles", {}),
                "repo_targets": lifecycle.get("repo_targets", []),
            },
        }
