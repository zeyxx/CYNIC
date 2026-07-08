"""Lifecycle diagnostic composition for organ-forge."""

from __future__ import annotations

from ..core.lifecycle_readiness import (
    fleet_nodes_ready as _readiness_fleet_nodes_ready,
    lifecycle_blockers as _readiness_lifecycle_blockers,
)
from ..core.lifecycle_state import build_workflow_state


class LifecycleDiagnosticService:
    @staticmethod
    def fleet_probe_value(probe: dict | None) -> dict:
        if isinstance(probe, dict):
            return probe
        return {"enabled": False, "status": "not_run"}

    @staticmethod
    def probe_dict_value(probe: dict | None) -> dict:
        if isinstance(probe, dict):
            return probe
        return {"enabled": False, "status": "not_run"}

    def fleet_nodes_ready(
        self,
        lifecycle: dict,
        fleet_probe: dict | None,
        fleet_inventory_probe: dict | None,
    ) -> bool:
        return _readiness_fleet_nodes_ready(
            lifecycle,
            self.fleet_probe_value(fleet_probe),
            self.probe_dict_value(fleet_inventory_probe),
        )

    def lifecycle_blockers(
        self,
        lifecycle: dict,
        build_node: dict,
        deploy_node: dict,
        *,
        activation_gap_details: list[dict],
        required_status: str | None,
        fleet_probe: dict | None,
        fleet_inventory_probe: dict | None,
        status_nodes: list[dict] | None = None,
    ) -> list[dict]:
        return _readiness_lifecycle_blockers(
            lifecycle,
            build_node,
            deploy_node,
            activation_gap_details=activation_gap_details,
            required_status=required_status,
            fleet_probe=self.fleet_probe_value(fleet_probe),
            fleet_inventory_probe=self.probe_dict_value(fleet_inventory_probe),
            status_nodes=status_nodes,
        )

    def build_context(
        self,
        *,
        lifecycle: dict,
        build_node: dict,
        deploy_node: dict,
        build_target: str,
        deploy_target: str,
        kernel_endpoint_source: str,
        kernel_endpoint: str,
        kernel_bus_canonical: bool,
        kernel_probe: dict | None,
        fleet_probe: dict | None,
        fleet_inventory_probe: dict | None,
        activation_gap_details: list[dict],
        activation_gaps: list[str],
        activation_role_nodes: list[dict],
    ) -> dict:
        required_status = lifecycle.get("required_node_status")
        fleet_probe_value = self.fleet_probe_value(fleet_probe)
        fleet_inventory_value = self.probe_dict_value(fleet_inventory_probe)
        status_nodes_ready = (
            not any(
                node.get("status") != required_status
                for node in activation_role_nodes
            )
            if required_status
            else True
        )
        fleet_nodes_ready = self.fleet_nodes_ready(
            lifecycle,
            fleet_probe_value,
            fleet_inventory_value,
        )
        lifecycle_blockers = self.lifecycle_blockers(
            lifecycle,
            build_node,
            deploy_node,
            activation_gap_details=activation_gap_details,
            required_status=required_status,
            fleet_probe=fleet_probe_value,
            fleet_inventory_probe=fleet_inventory_value,
            status_nodes=activation_role_nodes,
        )
        workflow_state = build_workflow_state(
            lifecycle,
            build_node=build_node,
            deploy_node=deploy_node,
            required_status=required_status,
            activation_gap_details=activation_gap_details,
            activation_gaps=activation_gaps,
            fleet_probe=fleet_probe_value,
            kernel_probe=kernel_probe,
            fleet_inventory_probe=fleet_inventory_value,
            status_nodes_ready=status_nodes_ready,
            fleet_nodes_ready=fleet_nodes_ready,
            lifecycle_blockers=lifecycle_blockers,
        )
        return {
            "lifecycle": "kernel_release",
            "orchestrator_organ": lifecycle["orchestrator_organ"],
            "task_kind": lifecycle["task_kind"],
            "task_domain": lifecycle["task_domain"],
            "build_target": build_target,
            "build_target_status": build_node.get("status", "unknown"),
            "deploy_target": deploy_target,
            "deploy_target_status": deploy_node.get("status", "unknown"),
            "kernel_endpoint_source": kernel_endpoint_source,
            "kernel_endpoint": kernel_endpoint,
            "kernel_bus_canonical": kernel_bus_canonical,
            "kernel_probe": kernel_probe,
            "fleet_probe": fleet_probe_value,
            "fleet_inventory_probe": fleet_inventory_value,
            "fleet_requirements": lifecycle.get("fleet_requirements", {}),
            "required_node_status": required_status,
            "activation_requirements": lifecycle.get("activation_requirements", {}),
            "request_profiles": lifecycle.get("request_profiles", {}),
            "intents": lifecycle.get("intents", {}),
            "workflow": lifecycle.get("workflow", {}),
            "workflow_state": workflow_state,
            "activation_evidence_gaps": activation_gaps,
            "activation_evidence_gap_details": activation_gap_details,
            "status_nodes_ready": status_nodes_ready,
            "fleet_nodes_ready": fleet_nodes_ready,
            "lifecycle_nodes_ready": (
                status_nodes_ready and not activation_gaps and fleet_nodes_ready
            ),
            "lifecycle_blockers": lifecycle_blockers,
            "requires_e2e": lifecycle.get("requires_e2e", True),
            "repo_targets": lifecycle.get("repo_targets", []),
        }
