"""Lifecycle workflow state helpers for organ-forge.

This module keeps the registry policy model separate from the transport/client
adapter so the adapter can stay focused on kernel I/O.
"""

from __future__ import annotations


def validate_workflow_layers(workflow: dict, orchestrator_organ: str) -> list[str]:
    errors: list[str] = []
    if workflow.get("name") != "kernel_release":
        errors.append("lifecycle workflow.name must be kernel_release")
    if workflow.get("owner") != orchestrator_organ:
        errors.append("lifecycle workflow.owner must match orchestrator_organ")
    state_layers = workflow.get("state_layers", {})
    if not isinstance(state_layers, dict) or not state_layers:
        errors.append("lifecycle workflow.state_layers must describe desired/declared/observed/effective")
        return errors

    expected_layers = {
        "desired": "human_intent",
        "declared": "infra/registry.json",
        "observed": "runtime_probes",
        "effective": "organ-forge",
    }
    for layer_name, source in expected_layers.items():
        layer = state_layers.get(layer_name, {})
        if not isinstance(layer, dict):
            errors.append(f"lifecycle workflow.state_layers.{layer_name} must be an object")
            continue
        if layer.get("source") != source:
            errors.append(
                f"lifecycle workflow.state_layers.{layer_name}.source must be {source}"
            )
        if not layer.get("description"):
            errors.append(
                f"lifecycle workflow.state_layers.{layer_name}.description missing"
            )
    return errors


def build_workflow_state(
    lifecycle: dict,
    *,
    build_node: dict,
    deploy_node: dict,
    required_status: str | None,
    activation_gap_details: list[dict],
    activation_gaps: list[str],
    fleet_probe: dict,
    fleet_inventory_probe: dict,
    kernel_probe: dict,
    status_nodes_ready: bool,
    fleet_nodes_ready: bool,
    lifecycle_blockers: list[dict],
) -> dict:
    workflow = lifecycle.get("workflow", {})
    workflow_state = {
        "desired": workflow.get("state_layers", {}).get("desired", {}),
        "declared": {
            "workflow": workflow,
            "required_node_status": required_status,
            "activation_requirements": lifecycle.get("activation_requirements", {}),
            "fleet_requirements": lifecycle.get("fleet_requirements", {}),
            "request_profiles": lifecycle.get("request_profiles", {}),
            "repo_targets": lifecycle.get("repo_targets", []),
        },
        "observed": {
            "build_target_status": build_node.get("status", "unknown"),
            "deploy_target_status": deploy_node.get("status", "unknown"),
            "activation_evidence_gaps": activation_gaps,
            "activation_evidence_gap_details": activation_gap_details,
            "fleet_probe": fleet_probe,
            "fleet_inventory_probe": fleet_inventory_probe,
            "kernel_probe": kernel_probe,
            "status_nodes_ready": status_nodes_ready,
            "fleet_nodes_ready": fleet_nodes_ready,
        },
        "effective": {
            "lifecycle_nodes_ready": (
                status_nodes_ready and not activation_gaps and fleet_nodes_ready
            ),
            "dispatch_allowed": (
                status_nodes_ready and not activation_gaps and fleet_nodes_ready
            ),
            "release_allowed": (
                status_nodes_ready and not activation_gaps and fleet_nodes_ready
            ),
            "lifecycle_blockers": lifecycle_blockers,
        },
    }
    return workflow_state
