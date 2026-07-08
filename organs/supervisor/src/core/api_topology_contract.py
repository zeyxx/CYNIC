"""Compatibility mirror for the CYNIC API topology contract validation service.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
module as a Python compatibility mirror until remaining callers are ported.
"""

from __future__ import annotations

import json
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


FindingFactory = Callable[[str, str, str, str], Any]


def openapi_operations(openapi: dict[str, Any]) -> set[str]:
    operations: set[str] = set()
    for path, methods in openapi.get("paths", {}).items():
        if not isinstance(methods, dict):
            continue
        for method in methods:
            if method.startswith("x-"):
                continue
            operations.add(f"{method.upper()} {path}")
    return operations


def operation_root(operation: str) -> str:
    path = operation.split(" ", 1)[1]
    return "/" + path.strip("/").split("/")[0]


def api_geometry_report(
    *,
    openapi: dict[str, Any],
    topology: dict[str, Any],
    openapi_source: str,
    topology_source: str,
) -> dict[str, Any]:
    operations = topology["operations"]
    by_root: dict[str, list[tuple[str, dict[str, Any]]]] = defaultdict(list)
    by_family: dict[str, list[tuple[str, dict[str, Any]]]] = defaultdict(list)
    for operation, meta in operations.items():
        by_root[operation_root(operation)].append((operation, meta))
        by_family[meta["canonical_family"]].append((operation, meta))

    root_splits = []
    for root, items in sorted(by_root.items()):
        families = Counter(meta["canonical_family"] for _, meta in items)
        if len(families) > 1:
            root_splits.append(
                {
                    "root": root,
                    "families": dict(sorted(families.items())),
                    "operations": [
                        {
                            "operation": operation,
                            "canonical_family": meta["canonical_family"],
                            "status": meta["status"],
                            "mutation_level": meta["mutation_level"],
                        }
                        for operation, meta in sorted(items)
                    ],
                }
            )

    transitional_by_family = {}
    for family, items in sorted(by_family.items()):
        transitional = [
            {
                "operation": operation,
                "mutation_level": meta["mutation_level"],
                "agent_safe_default": meta["agent_safe_default"],
            }
            for operation, meta in sorted(items)
            if meta["status"] == "transitional"
        ]
        if transitional:
            transitional_by_family[family] = transitional

    dangerous = [
        {
            "operation": operation,
            "canonical_family": meta["canonical_family"],
            "status": meta["status"],
            "mutation_level": meta["mutation_level"],
        }
        for operation, meta in sorted(operations.items())
        if meta["mutation_level"] in {"deployment", "destructive"}
    ]

    next_migrations = []
    priority = 1
    for split in root_splits:
        next_migrations.append(
            {
                "priority": priority,
                "family": ",".join(sorted(split["families"])),
                "reason": f"Root {split['root']} still mixes canonical families.",
                "action": "Move clients to canonical roots and remove the split-producing aliases.",
            }
        )
        priority += 1
    for family, transitional in transitional_by_family.items():
        next_migrations.append(
            {
                "priority": priority,
                "family": family,
                "reason": f"{len(transitional)} transitional operation(s) remain in this family.",
                "action": "Move clients to canonical operations, then remove or hard-deprecate the transitional routes.",
            }
        )
        priority += 1

    return {
        "schema": "cynic.api_geometry_report.v1",
        "source": {
            "openapi": openapi_source,
            "topology": topology_source,
        },
        "topology_budget": topology.get("topology_budget", {}),
        "summary": {
            "paths": len(openapi["paths"]),
            "operations": len(operations),
            "layers": dict(sorted(Counter(meta["layer"] for meta in operations.values()).items())),
            "mutation_levels": dict(
                sorted(Counter(meta["mutation_level"] for meta in operations.values()).items())
            ),
            "statuses": dict(
                sorted(Counter(meta["status"] for meta in operations.values()).items())
            ),
            "agent_safe_default": dict(
                sorted(
                    (str(key).lower(), value)
                    for key, value in Counter(
                        meta["agent_safe_default"] for meta in operations.values()
                    ).items()
                )
            ),
            "root_split_count": len(root_splits),
            "transitional_operation_count": sum(
                1 for meta in operations.values() if meta["status"] == "transitional"
            ),
            "dangerous_operation_count": len(dangerous),
        },
        "root_splits": root_splits,
        "transitional_by_family": transitional_by_family,
        "dangerous_operations": dangerous,
        "next_migrations": next_migrations,
    }


@dataclass(frozen=True)
class ApiTopologyContractService:
    workspace_root: Path
    openapi_path: Path
    topology_path: Path
    alignment_path: Path
    discovery_rs_path: Path
    middleware_rs_path: Path
    load_json: Callable[[Path], Any]
    relative: Callable[[Path], str]
    finding: FindingFactory

    def _load_json_contract(self, path: Path, *, kind: str) -> tuple[dict[str, Any] | None, list[Any]]:
        if not path.exists():
            return None, [
                self.finding(
                    "critical",
                    f"{kind}_missing",
                    self.relative(path),
                    f"{self.relative(path)} must exist for API topology validation.",
                )
            ]
        try:
            value = self.load_json(path)
        except json.JSONDecodeError as exc:
            return None, [
                self.finding(
                    "critical",
                    f"{kind}_invalid_json",
                    self.relative(path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]
        if not isinstance(value, dict):
            return None, [
                self.finding(
                    "critical",
                    f"{kind}_invalid_shape",
                    self.relative(path),
                    "contract must be a JSON object.",
                )
            ]
        return value, []

    def _read_text(self, path: Path, *, kind: str) -> tuple[str, list[Any]]:
        if not path.exists():
            return "", [
                self.finding(
                    "critical",
                    f"{kind}_missing",
                    self.relative(path),
                    f"{self.relative(path)} must exist for API discovery validation.",
                )
            ]
        return path.read_text(encoding="utf-8"), []

    def validate(self) -> list[Any]:
        findings: list[Any] = []
        openapi, loaded = self._load_json_contract(self.openapi_path, kind="openapi_contract")
        findings.extend(loaded)
        topology, loaded = self._load_json_contract(self.topology_path, kind="api_topology_contract")
        findings.extend(loaded)
        alignment, loaded = self._load_json_contract(self.alignment_path, kind="api_layer_alignment_contract")
        findings.extend(loaded)
        discovery_rs, loaded = self._read_text(self.discovery_rs_path, kind="api_discovery_rs")
        findings.extend(loaded)
        middleware_rs, loaded = self._read_text(self.middleware_rs_path, kind="api_middleware_rs")
        findings.extend(loaded)
        if openapi is None or topology is None or alignment is None:
            return findings

        if topology.get("schema") != "cynic.api_topology.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_api_topology_schema",
                    self.relative(self.topology_path),
                    f"expected cynic.api_topology.v1, got {topology.get('schema')!r}",
                )
            )
        if topology.get("status") != "normative":
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_not_normative",
                    self.relative(self.topology_path),
                    "api-topology-v1 must be normative before agents can use API geometry.",
                )
            )

        operations = openapi_operations(openapi)
        topology_operations = set(topology.get("operations", {}).keys())
        allowed = topology.get("allowed", {})
        allowed_layers = set(allowed.get("layers", []))
        allowed_mutation_levels = set(allowed.get("mutation_levels", []))
        allowed_statuses = set(allowed.get("statuses", []))
        topology_budget = topology.get("topology_budget", {})

        extension = openapi.get("x-cynic-api-topology-contract")
        expected_extension = {
            "schema": topology.get("schema"),
            "path": self.relative(self.topology_path),
        }
        if extension != expected_extension:
            findings.append(
                self.finding(
                    "critical",
                    "openapi_topology_extension_drift",
                    self.relative(self.openapi_path),
                    "OpenAPI must link api-topology-v1 through x-cynic-api-topology-contract.",
                )
            )
        alignment_extension = openapi.get("x-cynic-api-layer-alignment-contract")
        expected_alignment_extension = {
            "schema": alignment.get("schema"),
            "path": self.relative(self.alignment_path),
        }
        if alignment_extension != expected_alignment_extension:
            findings.append(
                self.finding(
                    "critical",
                    "openapi_layer_alignment_extension_drift",
                    self.relative(self.openapi_path),
                    "OpenAPI must link api-layer-alignment-v1 through x-cynic-api-layer-alignment-contract.",
                )
            )
        if topology.get("source_of_truth", {}).get("layer_alignment") != self.relative(self.alignment_path):
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_layer_alignment_source_drift",
                    self.relative(self.topology_path),
                    f"source_of_truth.layer_alignment must point to {self.relative(self.alignment_path)}.",
                )
            )

        alignment_layers = set(alignment.get("layers", {}))
        for layer in sorted(allowed_layers - alignment_layers):
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_alignment_layer_missing",
                    self.relative(self.alignment_path),
                    f"missing layer alignment for topology layer {layer!r}.",
                )
            )
        for layer in sorted(alignment_layers - allowed_layers):
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_alignment_layer_unknown",
                    self.relative(self.alignment_path),
                    f"alignment declares layer {layer!r} absent from topology allowed.layers.",
                )
            )

        findings.extend(
            self._validate_public_discovery(
                openapi=openapi,
                topology=topology,
                discovery_rs=discovery_rs,
                middleware_rs=middleware_rs,
            )
        )
        findings.extend(
            self._validate_operation_geometry(
                operations=operations,
                topology=topology,
                topology_operations=topology_operations,
                allowed_layers=allowed_layers,
                allowed_mutation_levels=allowed_mutation_levels,
                allowed_statuses=allowed_statuses,
                topology_budget=topology_budget,
            )
        )
        return findings

    def _validate_public_discovery(
        self,
        *,
        openapi: dict[str, Any],
        topology: dict[str, Any],
        discovery_rs: str,
        middleware_rs: str,
    ) -> list[Any]:
        findings: list[Any] = []
        required_discovery = {
            "GET /openapi.json": {
                "serves": self.relative(self.openapi_path),
                "const": "OPENAPI_JSON",
                "handler": "openapi_handler",
            },
            "GET /api/topology": {
                "serves": self.relative(self.topology_path),
                "const": "API_TOPOLOGY_JSON",
                "handler": "api_topology_handler",
            },
            "GET /api/layer-alignment": {
                "serves": self.relative(self.alignment_path),
                "const": "API_LAYER_ALIGNMENT_JSON",
                "handler": "api_layer_alignment_handler",
            },
        }
        public_discovery_endpoints = topology.get("public_discovery_endpoints", {})
        for operation, discovery_spec in sorted(required_discovery.items()):
            served_path = discovery_spec["serves"]
            if public_discovery_endpoints.get(operation, {}).get("serves") != served_path:
                findings.append(
                    self.finding(
                        "critical",
                        "api_public_discovery_serves_drift",
                        operation,
                        f"public_discovery_endpoints must serve {served_path}.",
                    )
                )
            method, path = operation.split(" ", 1)
            include_tail = "../" * 5 + served_path
            if (
                f"const {discovery_spec['const']}: &str" not in discovery_rs
                or f'include_str!("{include_tail}")' not in discovery_rs
            ):
                findings.append(
                    self.finding(
                        "critical",
                        "api_public_discovery_include_missing",
                        self.relative(self.discovery_rs_path),
                        f"{operation} must include {served_path} through {discovery_spec['const']}.",
                    )
                )
            handler_body = f"pub async fn {discovery_spec['handler']}() -> Response"
            if handler_body not in discovery_rs or f"json_contract({discovery_spec['const']})" not in discovery_rs:
                findings.append(
                    self.finding(
                        "critical",
                        "api_public_discovery_handler_missing",
                        self.relative(self.discovery_rs_path),
                        f"{operation} handler {discovery_spec['handler']} must serve {discovery_spec['const']}.",
                    )
                )
            if f'path == "{path}"' not in middleware_rs:
                findings.append(
                    self.finding(
                        "critical",
                        "api_public_discovery_auth_exemption_missing",
                        self.relative(self.middleware_rs_path),
                        f"auth middleware must exempt public discovery path {path}.",
                    )
                )
            openapi_operation = openapi.get("paths", {}).get(path, {}).get(method.lower())
            if openapi_operation is None:
                findings.append(
                    self.finding(
                        "critical",
                        "api_public_discovery_openapi_operation_missing",
                        self.relative(self.openapi_path),
                        f"{operation} is missing from OpenAPI.",
                    )
                )
            elif openapi_operation.get("security") != []:
                findings.append(
                    self.finding(
                        "critical",
                        "api_public_discovery_security_not_empty",
                        self.relative(self.openapi_path),
                        f"{operation} must declare security=[].",
                    )
                )
            discovery_contract = topology.get("operations", {}).get(operation)
            if discovery_contract is None:
                findings.append(
                    self.finding(
                        "critical",
                        "api_public_discovery_topology_operation_missing",
                        self.relative(self.topology_path),
                        f"{operation} is missing from API topology.",
                    )
                )
            else:
                expected = {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": True,
                    "canonical_family": "/api",
                    "status": "canonical",
                }
                for field, expected_value in expected.items():
                    if discovery_contract.get(field) != expected_value:
                        findings.append(
                            self.finding(
                                "critical",
                                "api_public_discovery_contract_drift",
                                operation,
                                f"public discovery {field} must be {expected_value!r}.",
                            )
                        )
        return findings

    def _validate_operation_geometry(
        self,
        *,
        operations: set[str],
        topology: dict[str, Any],
        topology_operations: set[str],
        allowed_layers: set[str],
        allowed_mutation_levels: set[str],
        allowed_statuses: set[str],
        topology_budget: dict[str, Any],
    ) -> list[Any]:
        findings: list[Any] = []
        by_root: dict[str, list[dict[str, Any]]] = defaultdict(list)
        for operation, contract in topology.get("operations", {}).items():
            by_root[operation_root(operation)].append(contract)
        root_splits = {
            root: Counter(contract.get("canonical_family") for contract in contracts)
            for root, contracts in by_root.items()
            if len({contract.get("canonical_family") for contract in contracts}) > 1
        }
        transitional_count = sum(
            1
            for contract in topology.get("operations", {}).values()
            if contract.get("status") == "transitional"
        )
        legacy_count = sum(
            1
            for contract in topology.get("operations", {}).values()
            if contract.get("status") == "legacy"
        )
        max_root_splits = topology_budget.get("max_root_splits")
        max_transitional = topology_budget.get("max_transitional_operations")
        max_legacy = topology_budget.get("max_legacy_operations")
        if max_root_splits is None or len(root_splits) > max_root_splits:
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_root_split_budget_exceeded",
                    self.relative(self.topology_path),
                    f"found {len(root_splits)} root splits, budget {max_root_splits}.",
                )
            )
        if max_transitional is None or transitional_count > max_transitional:
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_transitional_budget_exceeded",
                    self.relative(self.topology_path),
                    f"found {transitional_count} transitional operations, budget {max_transitional}.",
                )
            )
        if max_legacy is None or legacy_count > max_legacy:
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_legacy_budget_exceeded",
                    self.relative(self.topology_path),
                    f"found {legacy_count} legacy operations, budget {max_legacy}.",
                )
            )

        for operation in sorted(operations - topology_operations):
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_operation_missing",
                    self.relative(self.topology_path),
                    f"missing topology entry for OpenAPI operation: {operation}.",
                )
            )
        for operation in sorted(topology_operations - operations):
            findings.append(
                self.finding(
                    "critical",
                    "api_topology_operation_stale",
                    self.relative(self.topology_path),
                    f"stale topology entry absent from OpenAPI: {operation}.",
                )
            )

        dangerous = {"deployment", "destructive"}
        required_fields = {
            "layer",
            "mutation_level",
            "agent_safe_default",
            "canonical_family",
            "status",
        }
        for operation, contract in sorted(topology.get("operations", {}).items()):
            missing_fields = sorted(required_fields - set(contract))
            if missing_fields:
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_operation_metadata_missing",
                        operation,
                        f"missing fields {missing_fields}.",
                    )
                )
                continue
            if contract["layer"] not in allowed_layers:
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_operation_layer_invalid",
                        operation,
                        f"invalid layer {contract['layer']!r}.",
                    )
                )
            if contract["mutation_level"] not in allowed_mutation_levels:
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_operation_mutation_level_invalid",
                        operation,
                        f"invalid mutation_level {contract['mutation_level']!r}.",
                    )
                )
            if contract["status"] not in allowed_statuses:
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_operation_status_invalid",
                        operation,
                        f"invalid status {contract['status']!r}.",
                    )
                )
            if not isinstance(contract["agent_safe_default"], bool):
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_agent_safe_default_invalid",
                        operation,
                        "agent_safe_default must be boolean.",
                    )
                )
            canonical_family = contract["canonical_family"]
            if not isinstance(canonical_family, str) or not canonical_family.startswith("/"):
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_canonical_family_invalid",
                        operation,
                        "canonical_family must be an absolute path family.",
                    )
                )
            if contract.get("mutation_level") in dangerous and contract.get("agent_safe_default") is not False:
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_dangerous_mutation_agent_safe",
                        operation,
                        "dangerous mutation cannot be agent safe by default.",
                    )
                )
            if (
                contract.get("mutation_level") == "deployment"
                and contract.get("layer") != "metabolism"
                and contract.get("canonical_family") != "/lifecycle"
            ):
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_deployment_not_metabolism",
                        operation,
                        "deployment mutation must be metabolism or point to /lifecycle.",
                    )
                )
            if contract.get("layer") == "perception" and contract.get("mutation_level") in dangerous:
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_perception_mutates_dangerously",
                        operation,
                        "perception cannot use dangerous mutation.",
                    )
                )
            if contract.get("status") in {"compat", "transitional"} and canonical_family == operation.split(" ", 1)[1]:
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_compat_family_not_distinct",
                        operation,
                        "compat/transitional operation must name a distinct canonical_family.",
                    )
                )
            if contract.get("status") == "compat" and not topology_budget.get("compat_operations", {}).get("allowed", False):
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_compat_budget_disabled",
                        operation,
                        "compat operation present but compat budget is disabled.",
                    )
                )

        canonical_by_family_and_mutation = {
            (contract.get("canonical_family"), contract.get("mutation_level"))
            for contract in topology.get("operations", {}).values()
            if contract.get("status") == "canonical"
        }
        for operation, contract in sorted(topology.get("operations", {}).items()):
            if contract.get("status") != "transitional":
                continue
            key = (contract.get("canonical_family"), contract.get("mutation_level"))
            if key not in canonical_by_family_and_mutation:
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_transitional_without_canonical",
                        operation,
                        "transitional operation has no canonical operation in the same family and mutation level.",
                    )
                )
        return findings
