"""Legacy Python mirror for the infrastructure MCP contract.

The stable deterministic authority is migrating into `crates/cynic-mcp`.
Keep this module as a compatibility surface until supervisor wrappers consume
the Rust validator directly.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class InfrastructureMcpContractService:
    workspace_root: Path
    contract_path: Path
    load_json: Callable[[Path], Any]
    relative: Callable[[Path], str]
    finding: FindingFactory

    def validate(self) -> list[Any]:
        findings: list[Any] = []
        if not self.contract_path.exists():
            return [
                self.finding(
                    "critical",
                    "infrastructure_mcp_contract_missing",
                    self.relative(self.contract_path),
                    "infrastructure-mcp-v1.json must define canonical infrastructure MCP operations.",
                )
            ]

        try:
            contract = self.load_json(self.contract_path)
        except json.JSONDecodeError as exc:
            return [
                self.finding(
                    "critical",
                    "infrastructure_mcp_contract_invalid_json",
                    self.relative(self.contract_path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]

        if contract.get("schema") != "cynic.infrastructure_mcp.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_infrastructure_mcp_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.infrastructure_mcp.v1, got {contract.get('schema')!r}",
                )
            )
        if contract.get("status") != "target_contract":
            findings.append(
                self.finding(
                    "critical",
                    "infrastructure_mcp_status_invalid",
                    self.relative(self.contract_path),
                    "infrastructure MCP contract status must remain target_contract until all mutating ops are implemented.",
                )
            )

        principles = set(contract.get("principles") or [])
        for principle in (
            "deny_by_default",
            "registry_desired_state_first",
            "synapse_dns_is_the_service_contract",
            "mutating_operations_require_preflight_and_audit",
            "private_keys_never_leave_the_node",
        ):
            if principle not in principles:
                findings.append(
                    self.finding(
                        "critical",
                        "infrastructure_mcp_principle_missing",
                        self.relative(self.contract_path),
                        f"principles must include {principle}.",
                    )
                )

        families = contract.get("operation_families")
        if not isinstance(families, dict):
            findings.append(
                self.finding(
                    "critical",
                    "infrastructure_mcp_operation_families_missing",
                    self.relative(self.contract_path),
                    "operation_families must be an object keyed by family name.",
                )
            )
            families = {}

        operations = self._operations(families)
        expected_tools = {
            "fleet.node.exec": ("fleet_node_exec", True),
            "lxc_exec_bridge.plan": ("lxc_exec_bridge_plan", False),
            "lxc_exec_bridge.bundle": ("lxc_exec_bridge_bundle", False),
            "lxc_exec_bridge.status": ("lxc_exec_bridge_status", False),
            "network.mesh.status": ("network_mesh_status", False),
            "network.mesh.dns.render": ("network_mesh_dns_render", False),
            "network.mesh.dns.activation_plan": ("network_mesh_dns_activation_plan", False),
            "network.mesh.dns.activation_bundle": ("network_mesh_dns_activation_bundle", False),
            "network.mesh.dns.activation_status": ("network_mesh_dns_activation_status", False),
            "storage.primary.plan": ("storage_primary_plan", False),
            "storage.primary.bundle": ("storage_primary_bundle", False),
            "storage.primary.status": ("storage_primary_status", False),
            "storage.primary.readiness": ("storage_primary_readiness", False),
        }
        for operation, (tool, mutates) in expected_tools.items():
            item = operations.get(operation)
            if not isinstance(item, dict):
                findings.append(
                    self.finding(
                        "critical",
                        "infrastructure_mcp_operation_missing",
                        self.relative(self.contract_path),
                        f"operation {operation!r} must be declared.",
                    )
                )
                continue
            if item.get("mcp_tool") != tool:
                findings.append(
                    self.finding(
                        "critical",
                        "infrastructure_mcp_tool_drift",
                        self.relative(self.contract_path),
                        f"{operation}.mcp_tool must be {tool!r}, got {item.get('mcp_tool')!r}.",
                    )
                )
            if item.get("mutates") is not mutates:
                findings.append(
                    self.finding(
                        "critical",
                        "infrastructure_mcp_mutability_drift",
                        self.relative(self.contract_path),
                        f"{operation}.mutates must be {mutates!r}.",
                    )
                )

        mesh_status = operations.get("network.mesh.status", {})
        runtime_observation_contract = (
            mesh_status.get("runtime_observation_contract")
            if isinstance(mesh_status, dict)
            else None
        )
        if not isinstance(runtime_observation_contract, dict):
            findings.append(
                self.finding(
                    "critical",
                    "infrastructure_mcp_runtime_observation_contract_missing",
                    self.relative(self.contract_path),
                    "network.mesh.status must declare runtime_observation_contract.",
                )
            )
        else:
            if runtime_observation_contract.get("schema") != "cynic.network_mesh.runtime_observation.v1":
                findings.append(
                    self.finding(
                        "critical",
                        "infrastructure_mcp_runtime_observation_schema_drift",
                        self.relative(self.contract_path),
                        "network.mesh.status runtime_observation_contract.schema must be cynic.network_mesh.runtime_observation.v1.",
                    )
                )
            required_fields = set(runtime_observation_contract.get("required_fields") or [])
            for field in (
                "schema",
                "source",
                "status",
                "severity",
                "blockers",
                "missing",
                "drift",
                "blocking",
            ):
                if field not in required_fields:
                    findings.append(
                        self.finding(
                            "critical",
                            "infrastructure_mcp_runtime_observation_field_missing",
                            self.relative(self.contract_path),
                            f"network.mesh.status runtime_observation_contract.required_fields must include {field}.",
                        )
                    )
            shared_between = set(runtime_observation_contract.get("shared_between") or [])
            for participant in ("crates/cynic-node", "organs/fleet"):
                if participant not in shared_between:
                    findings.append(
                        self.finding(
                            "critical",
                            "infrastructure_mcp_runtime_observation_participant_missing",
                            self.relative(self.contract_path),
                            f"network.mesh.status runtime_observation_contract.shared_between must include {participant}.",
                        )
                    )

        fleet_exec = operations.get("fleet.node.exec", {})
        if fleet_exec.get("lxc_bridge_contract") != "axioms/lxc-exec-bridge-v1.json":
            findings.append(
                self.finding(
                    "critical",
                    "infrastructure_mcp_lxc_bridge_contract_missing",
                    self.relative(self.contract_path),
                    "fleet.node.exec must point to axioms/lxc-exec-bridge-v1.json.",
                )
            )
        current_coverage = set(fleet_exec.get("current_coverage") or [])
        if "lxc_structured_precondition_missing_until_exec_bridge_exists_or_transport_configured" not in current_coverage:
            findings.append(
                self.finding(
                    "critical",
                    "infrastructure_mcp_lxc_precondition_guard_missing",
                    self.relative(self.contract_path),
                    "fleet.node.exec current_coverage must retain the structured LXC precondition guard.",
                )
            )

        readiness = operations.get("storage.primary.readiness", {})
        composes = set(readiness.get("composes") or [])
        for dependency in (
            "storage.primary.status",
            "lxc_exec_bridge.status",
            "network.mesh.status",
            "network.mesh.dns.activation_status",
            "fleet.node.status",
            "service.health",
        ):
            if dependency not in composes:
                findings.append(
                    self.finding(
                        "critical",
                        "infrastructure_mcp_readiness_dependency_missing",
                        self.relative(self.contract_path),
                        f"storage.primary.readiness must compose {dependency}.",
                    )
                )

        storage_status = operations.get("storage.primary.status", {})
        checks = set(storage_status.get("preconditions_reported") or [])
        for check in (
            "registry_declares_storage_node",
            "endpoint_is_synapse_dns",
            "bridge_active",
            "fleet_observes_node_running",
            "mesh_reaches_storage",
            "bundle_complete",
            "service_health_healthy",
        ):
            if check not in checks:
                findings.append(
                    self.finding(
                        "critical",
                        "infrastructure_mcp_storage_status_check_missing",
                        self.relative(self.contract_path),
                        f"storage.primary.status must report {check}.",
                    )
                )

        for operation in ("lxc_exec_bridge.bundle", "network.mesh.dns.activation_bundle", "storage.primary.bundle"):
            item = operations.get(operation, {})
            if item.get("writes_local_output_dir") is not True:
                findings.append(
                    self.finding(
                        "critical",
                        "infrastructure_mcp_bundle_output_contract_missing",
                        self.relative(self.contract_path),
                        f"{operation} must declare writes_local_output_dir=true.",
                    )
                )

        return findings

    def _operations(self, families: dict[str, Any]) -> dict[str, Any]:
        operations: dict[str, Any] = {}
        for family in families.values():
            if not isinstance(family, dict):
                continue
            family_operations = family.get("operations")
            if isinstance(family_operations, dict):
                operations.update(family_operations)
            for key, value in family.items():
                if isinstance(value, dict) and "." in key:
                    operations[key] = value
        return operations
