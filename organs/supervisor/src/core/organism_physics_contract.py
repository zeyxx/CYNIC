from __future__ import annotations

from pathlib import Path

import organs.supervisor.src.core.repo_topology_host as host


CONTRACT_PATH = host.WORKSPACE_ROOT / "axioms" / "organism-physics-v1.json"
EXPECTED_SCHEMA = "cynic.organism_physics.v1"


def validate_organism_physics_contract() -> list[host.Finding]:
    if not CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "organism_physics_contract_missing",
                host.relative(CONTRACT_PATH),
                "organism-physics-v1.json must exist to audit causal propagation, coupling, and energy debt.",
            )
        ]

    contract = host.load_json(CONTRACT_PATH)
    findings: list[host.Finding] = []
    if contract.get("schema") != EXPECTED_SCHEMA:
        return [
            host.Finding(
                "critical",
                "unexpected_organism_physics_schema",
                host.relative(CONTRACT_PATH),
                f"expected {EXPECTED_SCHEMA}, got {contract.get('schema')!r}",
            )
        ]

    authorities = contract.get("authorities") or {}
    for name, value in sorted(authorities.items()):
        path = host.WORKSPACE_ROOT / str(value)
        if not path.exists():
            findings.append(
                host.Finding(
                    "critical",
                    "organism_physics_authority_missing",
                    str(value),
                    f"authority {name!r} points to a path that does not exist.",
                )
            )

    findings.extend(_validate_lifecycle_closure(contract))
    findings.extend(_validate_watch_propagation(contract))
    findings.extend(_validate_runtime_coherence(contract))
    findings.extend(_validate_inertia_surfaces(contract))
    findings.extend(_validate_mcp_freshness_inertia())
    return findings


def _validate_lifecycle_closure(contract: dict) -> list[host.Finding]:
    registry = host.load_json(host.WORKSPACE_ROOT / "infra" / "registry.json")
    kernel_release = registry.get("lifecycle", {}).get("kernel_release", {})
    workflow = kernel_release.get("workflow", {})
    transition_contract = workflow.get("transition_contract", {})
    findings: list[host.Finding] = []

    required_layers = contract.get("required_lifecycle_layers") or []
    actual_layers = transition_contract.get("layers") or []
    if actual_layers != required_layers:
        findings.append(
            host.Finding(
                "critical",
                "physics_lifecycle_layers_incoherent",
                "infra/registry.json:lifecycle.kernel_release.workflow.transition_contract.layers",
                f"expected lifecycle layers {required_layers!r}, got {actual_layers!r}.",
            )
        )

    expected_evidence = contract.get("required_transition_evidence") or {}
    transitions = {
        (item.get("from"), item.get("to")): item for item in transition_contract.get("transitions", [])
    }
    for key, transition_key in (
        ("effective_to_applied", ("effective", "applied")),
        ("applied_to_verified", ("applied", "verified")),
    ):
        expected = expected_evidence.get(key) or []
        item = transitions.get(transition_key, {})
        actual = item.get("evidence") or []
        if actual != expected:
            findings.append(
                host.Finding(
                    "critical",
                    "physics_transition_evidence_incoherent",
                    "infra/registry.json:lifecycle.kernel_release.workflow.transition_contract",
                    f"transition {transition_key!r} must require evidence {expected!r}, got {actual!r}.",
                )
            )
        if item.get("read_back_required") is not True:
            findings.append(
                host.Finding(
                    "critical",
                    "physics_transition_readback_missing",
                    "infra/registry.json:lifecycle.kernel_release.workflow.transition_contract",
                    f"transition {transition_key!r} must require read-back.",
                )
            )
    return findings


def _validate_watch_propagation(contract: dict) -> list[host.Finding]:
    api_topology = host.load_json(host.WORKSPACE_ROOT / "axioms" / "api-topology-v1.json")
    watch_rule = contract.get("watch_propagation_rule") or {}
    route_name = watch_rule.get("kernel_surface")
    expected_tool = watch_rule.get("expected_mcp_tool")
    findings: list[host.Finding] = []

    route = (
        api_topology.get("operations", {}).get(route_name, {})
        or api_topology.get("routes", {}).get(route_name, {})
    )
    if route.get("status") != "canonical":
        return findings

    exposure = host.load_json(host.WORKSPACE_ROOT / "infra" / "agent-exposure.json")
    memory_tools = (
        exposure.get("mcp_tool_domains", {})
        .get(str(watch_rule.get("consumer_surface_group")), {})
        .get("tools", [])
    )
    if expected_tool not in memory_tools:
        findings.append(
            host.Finding(
                "warning",
                "physics_watch_not_propagated_to_mcp_surface",
                "infra/agent-exposure.json",
                f"{route_name} is canonical in api-topology but {expected_tool!r} is not exposed in the declared MCP memory surface.",
            )
        )

    router_path = host.WORKSPACE_ROOT / "crates" / "cynic-mcp" / "src" / "routers" / "forward.rs"
    if router_path.exists():
        router_text = router_path.read_text(encoding="utf-8", errors="ignore")
        if expected_tool not in router_text:
            findings.append(
                host.Finding(
                    "warning",
                    "physics_watch_not_propagated_to_mcp_router",
                    host.relative(router_path),
                    f"{route_name} is canonical but the MCP forward router does not implement {expected_tool!r}.",
                )
            )
    return findings


def _validate_runtime_coherence(contract: dict) -> list[host.Finding]:
    surfaces = contract.get("runtime_coherence_surfaces") or {}
    findings: list[host.Finding] = []

    daemon_text = _read_text(surfaces.get("daemon_default_capabilities_path"))
    lifecycle_text = _read_text(surfaces.get("lifecycle_default_capabilities_path"))
    if daemon_text and lifecycle_text:
        if "observe,claim,wait_for_tasks" in daemon_text and '"poll_tasks"' in lifecycle_text and '"wait_for_tasks"' not in lifecycle_text:
            findings.append(
                host.Finding(
                    "warning",
                    "physics_runtime_capability_vocabulary_split",
                    str(surfaces.get("lifecycle_default_capabilities_path")),
                    "daemon defaults use wait_for_tasks while lifecycle fallback capabilities still advertise poll_tasks.",
                )
            )

    watch_adapter_text = _read_text(surfaces.get("kernel_watch_adapter_path"))
    if "falling back to transitional polling" in watch_adapter_text:
        findings.append(
            host.Finding(
                "warning",
                "physics_kernel_watch_fallback_dissipation",
                str(surfaces.get("kernel_watch_adapter_path")),
                "kernel watch adapter still dissipates energy through transitional polling fallback.",
            )
        )

    executor_text = _read_text(surfaces.get("executor_status_path"))
    if "hermes kanban show" in executor_text and "_sleep_poll" in executor_text:
        findings.append(
            host.Finding(
                "warning",
                "physics_executor_status_open_loop",
                str(surfaces.get("executor_status_path")),
                "Hermes execution status still relies on kanban show polling instead of a blocking or streamed membrane.",
            )
        )
    return findings


def _validate_inertia_surfaces(contract: dict) -> list[host.Finding]:
    findings: list[host.Finding] = []
    for raw_path in contract.get("inertia_surfaces") or []:
        path = host.WORKSPACE_ROOT / str(raw_path)
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        if "HERMES_AGENT_POLL" in text:
            findings.append(
                host.Finding(
                    "warning",
                    "physics_polling_inertia_surface_present",
                    str(raw_path),
                    "service surface still exposes HERMES_AGENT_POLL* wiring, so legacy polling inertia remains visible at deployment boundaries.",
                )
            )
    return findings


def _validate_mcp_freshness_inertia() -> list[host.Finding]:
    registry = host.load_json(host.WORKSPACE_ROOT / "infra" / "registry.json")
    controller = registry.get("lifecycle", {}).get("kernel_release", {}).get("mcp_freshness_controller", {})
    if controller.get("status") == "planned":
        return [
            host.Finding(
                "warning",
                "physics_mcp_freshness_loop_open",
                "infra/registry.json:lifecycle.kernel_release.mcp_freshness_controller",
                "MCP freshness controller remains planned, so the watch/MCP supply chain is not yet a closed automatic loop.",
            )
        ]
    return []


def _read_text(path_value: str | None) -> str:
    if not path_value:
        return ""
    path = host.WORKSPACE_ROOT / path_value
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8", errors="ignore")
