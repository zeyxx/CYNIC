"""Repo topology contract validation service."""

from __future__ import annotations

import json
import subprocess
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class TopologyContractService:
    workspace_root: Path
    contract_path: Path
    contracts_root: Path
    rust_hexagonal_crate_required_fields: set[str]
    root_class_required_fields: set[str]
    load_json: Callable[[Path], Any]
    relative: Callable[[Path], str]
    git_ls_untracked: Callable[..., list[str]]
    finding: FindingFactory

    def validate_contract_file_policy(self, contract: dict[str, Any]) -> list[Any]:
        findings: list[Any] = []
        policy = contract.get("contract_file_policy", {})
        allowed_extensions = set(policy.get("allowed_extensions", [".json"]))
        extension_exceptions = set(policy.get("extension_exceptions", []))
        allowed_subdirectories = {
            str(item.get("path", "")).rstrip("/") + "/"
            for item in policy.get("allowed_subdirectories", [])
            if item.get("path")
        }
        required_fields = policy.get("json_required_top_level_fields", ["schema", "status"])
        allowed_statuses = set(policy.get("allowed_statuses", []))
        schema_prefix = policy.get("schema_prefix")

        if not self.contracts_root.exists():
            return findings

        for item in policy.get("allowed_subdirectories", []):
            item_path = str(item.get("path", ""))
            for field in ("path", "class", "owner", "reason"):
                if not item.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "contract_subdirectory_metadata_missing",
                            item_path or "contract_file_policy.allowed_subdirectories",
                            f"allowed_subdirectories entries must declare {field}.",
                        )
                    )

        for item in policy.get("legacy_contract_roots_forbidden", []):
            item_path = str(item.get("path", ""))
            for field in ("path", "replacement", "reason"):
                if not item.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "legacy_contract_root_metadata_missing",
                            item_path or "contract_file_policy.legacy_contract_roots_forbidden",
                            f"legacy_contract_roots_forbidden entries must declare {field}.",
                        )
                    )
            if item_path and (self.workspace_root / item_path).exists():
                findings.append(
                    self.finding(
                        "critical",
                        "legacy_contract_root_present",
                        item_path,
                        f"Legacy topology/contract surface must not exist; use {item.get('replacement')!r}.",
                    )
                )

        for path in sorted(item for item in self.contracts_root.iterdir() if item.is_dir()):
            rel = self.relative(path).rstrip("/") + "/"
            if rel not in allowed_subdirectories:
                findings.append(
                    self.finding(
                        "critical",
                        "contract_subdirectory_undeclared",
                        rel,
                        "Subdirectories under axioms must be declared in repo-topology-v1.contract_file_policy.allowed_subdirectories.",
                    )
                )

        for path in sorted(item for item in self.contracts_root.rglob("*") if item.is_file()):
            rel = self.relative(path)
            if rel in extension_exceptions:
                continue
            if path.suffix not in allowed_extensions:
                findings.append(
                    self.finding(
                        "critical",
                        "contract_file_extension_forbidden",
                        rel,
                        f"axioms files must use one of {sorted(allowed_extensions)} or declare an explicit extension exception.",
                    )
                )

        for path in sorted(self.contracts_root.rglob("*.json")):
            try:
                payload = self.load_json(path)
            except json.JSONDecodeError:
                continue
            rel = self.relative(path)
            if rel.startswith("axioms/schemas/"):
                continue
            for field in required_fields:
                if not payload.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "contract_required_field_missing",
                            rel,
                            f"JSON contracts under axioms must declare top-level {field!r}.",
                        )
                    )
            status = payload.get("status")
            if allowed_statuses and status and status not in allowed_statuses:
                findings.append(
                    self.finding(
                        "critical",
                        "contract_status_unknown",
                        rel,
                        f"status {status!r} is not declared in repo-topology-v1.contract_file_policy.allowed_statuses.",
                    )
                )
            schema = payload.get("schema")
            if schema_prefix and schema and not str(schema).startswith(str(schema_prefix)):
                findings.append(
                    self.finding(
                        "critical",
                        "contract_schema_prefix_invalid",
                        rel,
                        f"schema {schema!r} must start with {schema_prefix!r}.",
                    )
                )
        findings.extend(self._validate_axiom_registry(policy))
        return findings

    def _validate_axiom_registry(self, policy: dict[str, Any]) -> list[Any]:
        registry_ref = policy.get("registry_contract")
        if not registry_ref:
            return []

        findings: list[Any] = []
        registry_path = self.workspace_root / str(registry_ref)
        registry_rel = self.relative(registry_path)
        if not registry_path.exists():
            return [
                self.finding(
                    "critical",
                    "axiom_registry_missing",
                    registry_rel,
                    "contract_file_policy.registry_contract must point to the axiom layer registry.",
                )
            ]

        try:
            registry = self.load_json(registry_path)
        except json.JSONDecodeError as exc:
            return [
                self.finding(
                    "critical",
                    "axiom_registry_invalid_json",
                    registry_rel,
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]

        if registry.get("schema") != "cynic.axiom_registry.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_axiom_registry_schema",
                    registry_rel,
                    f"expected cynic.axiom_registry.v1, got {registry.get('schema')!r}",
                )
            )

        functional_layers = {str(item.get("id")) for item in registry.get("functional_layers", []) if item.get("id")}
        topological_layers = {str(item.get("id")) for item in registry.get("topological_layers", []) if item.get("id")}
        raw_contract_scales = registry.get("contract_scales")
        if not isinstance(raw_contract_scales, list) or not raw_contract_scales:
            findings.append(
                self.finding(
                    "critical",
                    "axiom_registry_contract_scales_missing",
                    registry_rel,
                    "axiom-registry-v1 must declare contract_scales as a non-empty list.",
                )
            )
            raw_contract_scales = []
        contract_scales = {str(item.get("id")) for item in raw_contract_scales if item.get("id")}
        topology_axes = {str(item.get("root_layer")) for item in registry.get("topology_axes", []) if item.get("root_layer")}
        for item in raw_contract_scales:
            item_path = str(item.get("id") or "contract_scales")
            for field in ("id", "projection", "description"):
                if not item.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "axiom_registry_contract_scale_metadata_missing",
                            item_path,
                            f"contract_scales entries must declare {field}.",
                        )
                    )
        for item in registry.get("topology_axes", []):
            item_path = str(item.get("id") or "topology_axes")
            for field in ("id", "root_layer", "owns", "must_not_own"):
                if not item.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "axiom_registry_topology_axis_metadata_missing",
                            item_path,
                            f"topology_axes entries must declare {field}.",
                        )
                    )
        for item in registry.get("topological_layers", []):
            item_path = str(item.get("id") or "topological_layers")
            parent = item.get("parent")
            if parent not in (None, "") and str(parent) not in topological_layers:
                findings.append(
                    self.finding(
                        "critical",
                        "axiom_registry_topological_layer_parent_unknown",
                        item_path,
                        f"parent {parent!r} for topological layer {item.get('id')!r} is not declared in axiom-registry-v1.topological_layers.",
                    )
                )
        if topology_axes:
            for layer in sorted(topological_layers):
                root = layer.split(".", 1)[0]
                if root not in topology_axes:
                    findings.append(
                        self.finding(
                            "critical",
                            "axiom_registry_topological_layer_axis_unknown",
                            layer,
                            f"topological layer root {root!r} is not declared in axiom-registry-v1.topology_axes.",
                        )
                    )
        contracts = registry.get("contracts")
        if not isinstance(contracts, list):
            return findings + [
                self.finding(
                    "critical",
                    "axiom_registry_contracts_missing",
                    registry_rel,
                    "axiom-registry-v1 must declare contracts as a list.",
                )
            ]

        required_fields = {
            "id",
            "path",
            "family",
            "functional_layer",
            "topological_layer",
            "scale",
            "owner",
            "merge_policy",
        }
        registered_paths: set[str] = set()
        registered_ids: set[str] = set()
        for index, item in enumerate(contracts):
            item_path = str(item.get("path") or f"contracts[{index}]")
            for field in sorted(required_fields):
                if not item.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "axiom_registry_metadata_missing",
                            item_path,
                            f"axiom registry entries must declare {field}.",
                        )
                    )
            path_value = item.get("path")
            if path_value:
                path = str(path_value)
                if path in registered_paths:
                    findings.append(
                        self.finding(
                            "critical",
                            "axiom_registry_duplicate_path",
                            path,
                            "Each axiom contract path must appear only once in axiom-registry-v1.",
                        )
                    )
                registered_paths.add(path)
                if not (self.workspace_root / path).exists():
                    findings.append(
                        self.finding(
                            "critical",
                            "axiom_registry_stale_path",
                            path,
                            "Axiom registry entry points to a file that does not exist.",
                        )
                    )
            id_value = item.get("id")
            if id_value:
                axiom_id = str(id_value)
                if axiom_id in registered_ids:
                    findings.append(
                        self.finding(
                            "critical",
                            "axiom_registry_duplicate_id",
                            item_path,
                            f"Axiom registry id {axiom_id!r} is declared more than once.",
                        )
                    )
                registered_ids.add(axiom_id)

            functional_layer = item.get("functional_layer")
            if functional_layers and functional_layer and str(functional_layer) not in functional_layers:
                findings.append(
                    self.finding(
                        "critical",
                        "axiom_registry_functional_layer_unknown",
                        item_path,
                        f"functional_layer {functional_layer!r} is not declared in axiom-registry-v1.functional_layers.",
                    )
                )
            topological_layer = item.get("topological_layer")
            if topological_layers and topological_layer and str(topological_layer) not in topological_layers:
                findings.append(
                    self.finding(
                        "critical",
                        "axiom_registry_topological_layer_unknown",
                        item_path,
                        f"topological_layer {topological_layer!r} is not declared in axiom-registry-v1.topological_layers.",
                    )
                )
            scale = item.get("scale")
            if contract_scales and scale and str(scale) not in contract_scales:
                findings.append(
                    self.finding(
                        "critical",
                        "axiom_registry_contract_scale_unknown",
                        item_path,
                        f"scale {scale!r} is not declared in axiom-registry-v1.contract_scales.",
                    )
                )

        root_contracts = {
            self.relative(path)
            for path in self.contracts_root.glob("*.json")
            if path.name != "axiom-registry-v1.json"
        }
        root_contracts.add(registry_rel)
        for path in sorted(root_contracts - registered_paths):
            findings.append(
                self.finding(
                    "critical",
                    "axiom_contract_unregistered",
                    path,
                    "Every root axiom JSON contract must be declared in axiom-registry-v1 with functional and topological layers.",
                )
            )
        for path in sorted(registered_paths - root_contracts):
            if path.startswith("axioms/") and path.count("/") == 1:
                findings.append(
                    self.finding(
                        "critical",
                        "axiom_registry_path_not_root_contract",
                        path,
                        "Root axiom registry entries must point to root axioms/*.json contracts.",
                    )
                )

        totals = registry.get("totals", {})
        if isinstance(totals, dict) and totals.get("contracts") is not None and totals.get("contracts") != len(contracts):
            findings.append(
                self.finding(
                    "critical",
                    "axiom_registry_total_mismatch",
                    registry_rel,
                    f"totals.contracts={totals.get('contracts')!r} but registry contains {len(contracts)} entries.",
                )
            )
        return findings

    def validate_rust_hexagonal_crate_policy(self, contract: dict[str, Any]) -> list[Any]:
        findings: list[Any] = []
        policy = contract.get("rust_hexagonal_crate_policy")
        if not isinstance(policy, dict):
            return findings

        allowed_crates = policy.get("allowed_crates", [])
        if not isinstance(allowed_crates, list):
            findings.append(
                self.finding(
                    "critical",
                    "rust_hexagonal_crate_policy_invalid",
                    self.relative(self.contract_path),
                    "rust_hexagonal_crate_policy.allowed_crates must be a list.",
                )
            )
            return findings

        for index, item in enumerate(allowed_crates):
            item_path = str(item.get("path") or f"allowed_crates[{index}]")
            for field in sorted(self.rust_hexagonal_crate_required_fields):
                if not item.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "rust_hexagonal_crate_metadata_missing",
                            item_path,
                            f"allowed_crates entries must declare {field}.",
                        )
                    )
            path_value = item.get("path")
            if not path_value:
                continue
            crate_root = self.workspace_root / str(path_value)
            if not crate_root.exists():
                findings.append(
                    self.finding(
                        "critical",
                        "rust_hexagonal_crate_missing",
                        str(path_value),
                        "Declared Rust hexagonal crate does not exist.",
                    )
                )
                continue
            required_paths = item.get("required_paths", [])
            if not isinstance(required_paths, list):
                findings.append(
                    self.finding(
                        "critical",
                        "rust_hexagonal_crate_required_paths_invalid",
                        str(path_value),
                        "required_paths must be a list.",
                    )
                )
                continue
            for required in required_paths:
                candidate = crate_root / str(required)
                if not candidate.exists():
                    findings.append(
                        self.finding(
                            "critical",
                            "rust_hexagonal_crate_required_path_missing",
                            f"{path_value}{required}",
                            "Declared Rust hexagonal crate shape is incomplete.",
                        )
                    )
            findings.extend(
                self._validate_rust_file_line_policy(
                    path_value=str(path_value),
                    crate_root=crate_root,
                    item=item,
                )
            )
        return findings

    def _validate_rust_file_line_policy(
        self,
        *,
        path_value: str,
        crate_root: Path,
        item: dict[str, Any],
    ) -> list[Any]:
        findings: list[Any] = []
        max_rs_file_lines = item.get("max_rs_file_lines")
        if max_rs_file_lines is None:
            return findings
        if not isinstance(max_rs_file_lines, int) or max_rs_file_lines <= 0:
            findings.append(
                self.finding(
                    "critical",
                    "rust_hexagonal_crate_line_policy_invalid",
                    str(path_value),
                    "max_rs_file_lines must be a positive integer when declared.",
                )
            )
            return findings

        exception_max_lines: dict[str, int] = {}
        for exception in item.get("line_count_exceptions", []):
            exception_path = exception.get("path")
            if not exception_path:
                findings.append(
                    self.finding(
                        "critical",
                        "rust_hexagonal_crate_line_exception_missing_path",
                        str(path_value),
                        "line_count_exceptions entries must declare path.",
                    )
                )
                continue
            max_lines = exception.get("max_lines")
            if not isinstance(max_lines, int) or max_lines <= 0:
                findings.append(
                    self.finding(
                        "critical",
                        "rust_hexagonal_crate_line_exception_invalid",
                        f"{path_value}{exception_path}",
                        "line_count_exceptions.max_lines must be a positive integer.",
                    )
                )
                continue
            if not exception.get("reason"):
                findings.append(
                    self.finding(
                        "critical",
                        "rust_hexagonal_crate_line_exception_missing_reason",
                        f"{path_value}{exception_path}",
                        "line_count_exceptions entries must declare reason.",
                    )
                )
            exception_max_lines[str(exception_path)] = max_lines

        for source_path in sorted(crate_root.rglob("*.rs")):
            rel_from_crate = source_path.relative_to(crate_root).as_posix()
            rel_path = f"{str(path_value).rstrip('/')}/{rel_from_crate}"
            max_lines = exception_max_lines.get(rel_from_crate, max_rs_file_lines)
            with source_path.open("r", encoding="utf-8") as handle:
                line_count = sum(1 for _ in handle)
            if line_count > max_lines:
                findings.append(
                    self.finding(
                        "critical",
                        "rust_hexagonal_crate_file_too_large",
                        rel_path,
                        f"Rust source file has {line_count} lines; max allowed is {max_lines}. Split responsibilities or declare a justified exception.",
                    )
                )

        return findings

    def validate_topology_contract(self) -> list[Any]:
        findings: list[Any] = []
        if not self.contract_path.exists():
            return [
                self.finding(
                    "critical",
                    "repo_topology_contract_missing",
                    self.relative(self.contract_path),
                    "repo-topology-v1.json is the machine contract for filesystem concern boundaries.",
                )
            ]

        contract = self.load_json(self.contract_path)
        if contract.get("schema") != "cynic.repo_topology.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_repo_topology_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.repo_topology.v1, got {contract.get('schema')!r}",
                )
            )

        for name, value in sorted(contract.get("authorities", {}).items()):
            authority = self.workspace_root / value
            if not authority.exists():
                findings.append(
                    self.finding(
                        "critical",
                        "topology_authority_missing",
                        self.relative(authority),
                        f"authority {name!r} is declared by repo-topology-v1 but does not exist.",
                    )
                )

        declared_root_paths: set[str] = set()
        for root_class in contract.get("root_classes", []):
            path_value = root_class.get("path")
            if not path_value:
                findings.append(
                    self.finding(
                        "critical",
                        "root_class_path_missing",
                        self.relative(self.contract_path),
                        "root_classes entries must declare path.",
                    )
                )
                continue
            declared_root_paths.add(str(path_value).rstrip("/") + "/")
            for field in sorted(self.root_class_required_fields - {"path"}):
                if not root_class.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "root_class_metadata_missing",
                            str(path_value),
                            f"root_classes entries must declare {field}.",
                        )
                    )
            path = self.workspace_root / path_value
            if not path.exists():
                findings.append(
                    self.finding(
                        "medium",
                        "declared_topology_root_missing",
                        self.relative(path),
                        f"declared root class {root_class.get('class')!r} does not exist yet.",
                    )
                )

        policy = contract.get("top_level_root_policy", {})
        ignored_roots = set(policy.get("ignored_roots", []))
        if policy:
            for path in sorted(item for item in self.workspace_root.iterdir() if item.is_dir()):
                name = path.name
                if name in ignored_roots:
                    continue
                rel = f"{name}/"
                if rel not in declared_root_paths:
                    findings.append(
                        self.finding(
                            "critical",
                            "top_level_root_unclassified",
                            rel,
                            "Every top-level directory must be declared in repo-topology-v1.root_classes or ignored by top_level_root_policy.",
                        )
                    )
            if ignored_roots and (self.workspace_root / ".git").exists():
                try:
                    visible_ignored = self.git_ls_untracked(*sorted(ignored_roots))
                except (subprocess.CalledProcessError, FileNotFoundError) as exc:
                    findings.append(
                        self.finding(
                            "critical",
                            "topology_ignored_root_scan_failed",
                            self.relative(self.contract_path),
                            f"unable to query git untracked ignored roots: {exc}",
                        )
                    )
                    visible_ignored = []
                for item in visible_ignored:
                    root = item.split("/", 1)[0]
                    if root in ignored_roots:
                        findings.append(
                            self.finding(
                                "critical",
                                "topology_ignored_root_visible_untracked",
                                root + "/",
                                "Roots ignored by top_level_root_policy must also be ignored by Git or they remain visible as source candidates.",
                            )
                        )
                        break
        return findings
