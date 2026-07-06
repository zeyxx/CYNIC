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
        errors.append(
            "lifecycle workflow.state_layers must describe desired/declared/observed/effective/applied/verified"
        )
        return errors

    expected_layers = {
        "desired": "human_intent",
        "declared": "infra/registry.json",
        "observed": "runtime_probes",
        "effective": "organ-forge",
        "applied": "organ-lifecycle",
        "verified": "kernel_readback",
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
    errors.extend(validate_transition_contract(workflow))
    return errors


def validate_transition_contract(workflow: dict) -> list[str]:
    errors: list[str] = []
    contract = workflow.get("transition_contract", {})
    if not isinstance(contract, dict) or not contract:
        return ["lifecycle workflow.transition_contract missing"]
    if contract.get("schema") != "cynic.lifecycle_transition_contract.v1":
        errors.append(
            "lifecycle workflow.transition_contract.schema must be cynic.lifecycle_transition_contract.v1"
        )
    expected_layers = [
        "desired",
        "declared",
        "observed",
        "effective",
        "applied",
        "verified",
    ]
    if contract.get("layers") != expected_layers:
        errors.append(
            "lifecycle workflow.transition_contract.layers must be desired/declared/observed/effective/applied/verified"
        )
    transitions = contract.get("transitions", [])
    if not isinstance(transitions, list) or len(transitions) != len(expected_layers) - 1:
        errors.append("lifecycle workflow.transition_contract.transitions must cover each adjacent layer")
        return errors
    expected_pairs = list(zip(expected_layers, expected_layers[1:]))
    for transition, (source, target) in zip(transitions, expected_pairs):
        if not isinstance(transition, dict):
            errors.append("lifecycle workflow.transition_contract transition must be an object")
            continue
        if transition.get("from") != source or transition.get("to") != target:
            errors.append(
                f"lifecycle workflow.transition_contract must include transition {source}->{target}"
            )
        if not transition.get("authority"):
            errors.append(f"lifecycle transition {source}->{target} authority missing")
        evidence = transition.get("evidence", [])
        if not isinstance(evidence, list) or not evidence:
            errors.append(f"lifecycle transition {source}->{target} evidence missing")
        if not isinstance(transition.get("read_back_required"), bool):
            errors.append(
                f"lifecycle transition {source}->{target} read_back_required must be boolean"
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
        "applied": {
            "source": workflow.get("state_layers", {}).get("applied", {}),
            "status": "external_until_runner_result_observed",
        },
        "verified": {
            "source": workflow.get("state_layers", {}).get("verified", {}),
            "read_back_required": True,
            "status": "external_until_kernel_readback_observed",
        },
        "transition_contract": workflow.get("transition_contract", {}),
    }
    return workflow_state
