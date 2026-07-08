"""Validation service for the storage primary activation contract."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class StoragePrimaryActivationContractService:
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
                    "storage_primary_activation_contract_missing",
                    self.relative(self.contract_path),
                    "storage-primary-activation-v1.json must define the storage activation supply chain.",
                )
            ]

        try:
            contract = self.load_json(self.contract_path)
        except json.JSONDecodeError as exc:
            return [
                self.finding(
                    "critical",
                    "storage_primary_activation_contract_invalid_json",
                    self.relative(self.contract_path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]

        if contract.get("schema") != "cynic.storage_primary_activation.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_storage_primary_activation_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.storage_primary_activation.v1, got {contract.get('schema')!r}",
                )
            )
        if contract.get("status") != "normative":
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_not_normative",
                    self.relative(self.contract_path),
                    "storage primary activation must be normative before it can guide recovery.",
                )
            )

        authority = contract.get("authority")
        if not isinstance(authority, dict):
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_authority_missing",
                    self.relative(self.contract_path),
                    "storage primary activation must declare authority.",
                )
            )
            authority = {}
        for field in ("registry", "storage_schema", "lxc_exec_bridge", "network_mesh_dns", "organ"):
            if not authority.get(field):
                findings.append(
                    self.finding(
                        "critical",
                        "storage_primary_activation_authority_field_missing",
                        self.relative(self.contract_path),
                        f"authority must declare {field}.",
                    )
                )

        target = contract.get("target")
        if not isinstance(target, dict):
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_target_missing",
                    self.relative(self.contract_path),
                    "storage primary activation must declare target.",
                )
            )
            target = {}
        expected_target = {
            "node": "cynic-lxc-surrealdb",
            "role": "storage_primary",
            "service": "surrealdb",
            "canonical_endpoint": "http://organ-surrealdb.synapse.cynic:8000",
        }
        for field, expected in expected_target.items():
            if target.get(field) != expected:
                findings.append(
                    self.finding(
                        "critical",
                        "storage_primary_activation_target_drift",
                        self.relative(self.contract_path),
                        f"target.{field} must be {expected!r}, got {target.get(field)!r}.",
                    )
                )

        status_contract = contract.get("status_contract")
        if not isinstance(status_contract, dict):
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_status_contract_missing",
                    self.relative(self.contract_path),
                    "storage primary activation must declare status_contract.",
                )
            )
            status_contract = {}
        required_checks = {
            "endpoint_is_synapse_dns",
            "bridge_active",
            "fleet_observes_node_running",
            "mesh_reaches_storage",
            "bundle_complete",
            "service_health_healthy",
        }
        active_requires = set(status_contract.get("active_requires") or [])
        missing_checks = sorted(required_checks - active_requires)
        if missing_checks:
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_active_checks_missing",
                    self.relative(self.contract_path),
                    f"status_contract.active_requires is missing {missing_checks!r}.",
                )
            )

        request = contract.get("activation_request_contract")
        if not isinstance(request, dict):
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_request_contract_missing",
                    self.relative(self.contract_path),
                    "storage primary activation must declare activation_request_contract.",
                )
            )
            request = {}

        bundles = request.get("bundles") if isinstance(request.get("bundles"), dict) else {}
        expected_bundles = {
            "lxc_exec_bridge": "/tmp/cynic-lxc-bridge",
            "network_mesh_dns": "/tmp/cynic-synapse-dns",
            "storage_primary": "/tmp/cynic-storage-primary",
        }
        for name, expected in expected_bundles.items():
            if bundles.get(name) != expected:
                findings.append(
                    self.finding(
                        "critical",
                        "storage_primary_activation_bundle_missing",
                        self.relative(self.contract_path),
                        f"activation_request_contract.bundles[{name!r}] must be {expected!r}.",
                    )
                )

        coord_targets = set(request.get("coord_targets") or [])
        for target_path in ("organs/fleet", "organs/keep", "infra/registry.json"):
            if target_path not in coord_targets:
                findings.append(
                    self.finding(
                        "critical",
                        "storage_primary_activation_coord_target_missing",
                        self.relative(self.contract_path),
                        f"coord_targets must include {target_path}.",
                    )
                )

        sequence = request.get("required_sequence")
        if not isinstance(sequence, list) or not sequence:
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_sequence_missing",
                    self.relative(self.contract_path),
                    "activation_request_contract.required_sequence must be a non-empty list.",
                )
            )
            sequence = []
        sequence_text = "\n".join(str(item) for item in sequence)
        for marker in (
            "lxc_exec_bridge",
            "network_mesh_dns",
            "storage_primary",
            "storage_primary_readiness.ready",
        ):
            if marker not in sequence_text:
                findings.append(
                    self.finding(
                        "critical",
                        "storage_primary_activation_sequence_step_missing",
                        self.relative(self.contract_path),
                        f"required_sequence must include {marker}.",
                    )
                )
        if "kernel health" not in sequence_text or ("storage_down" not in sequence_text and "storage down" not in sequence_text):
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_sequence_step_missing",
                    self.relative(self.contract_path),
                    "required_sequence must include kernel health recovery from storage_down.",
                )
            )

        if not request.get("no_cortex_root_execution"):
            findings.append(
                self.finding(
                    "critical",
                    "storage_primary_activation_allows_cortex_root_execution",
                    self.relative(self.contract_path),
                    "activation_request_contract.no_cortex_root_execution must be true.",
                )
            )

        forbidden = set(contract.get("forbidden_patterns") or [])
        for pattern in ("curl | sh", "root/root default credentials", "192.168.* service endpoint", "direct pct exec exposed to agents"):
            if pattern not in forbidden:
                findings.append(
                    self.finding(
                        "critical",
                        "storage_primary_activation_forbidden_pattern_missing",
                        self.relative(self.contract_path),
                        f"forbidden_patterns must include {pattern!r}.",
                    )
                )

        return findings
