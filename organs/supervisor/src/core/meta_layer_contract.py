"""Validation service for the CYNIC meta-layer contract."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class MetaLayerContractService:
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
                    "meta_layer_contract_missing",
                    self.relative(self.contract_path),
                    "meta-layer-contract-v1.json must define CYNIC meta-layer precedence.",
                )
            ]

        try:
            contract = self.load_json(self.contract_path)
        except json.JSONDecodeError as exc:
            return [
                self.finding(
                    "critical",
                    "meta_layer_contract_invalid_json",
                    self.relative(self.contract_path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]

        if contract.get("schema") != "cynic.meta_layer_contract.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_meta_layer_contract_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.meta_layer_contract.v1, got {contract.get('schema')!r}",
                )
            )
        if contract.get("status") != "normative":
            findings.append(
                self.finding(
                    "critical",
                    "meta_layer_contract_not_normative",
                    self.relative(self.contract_path),
                    "meta-layer-contract-v1 must be normative before it can anchor layer precedence.",
                )
            )

        recursion_boundary = contract.get("recursion_boundary")
        if not isinstance(recursion_boundary, dict) or not recursion_boundary:
            findings.append(
                self.finding(
                    "critical",
                    "meta_layer_recursion_boundary_missing",
                    self.relative(self.contract_path),
                    "meta-layer-contract-v1 must declare recursion_boundary so contract-of-contracts recursion has a mechanical root.",
                )
            )
            recursion_boundary = {}
        required_boundary_fields = {
            "rule",
            "root_contract",
            "registry_contract",
            "allowed_meta_contracts",
            "projection_rule",
            "new_contract_admission",
            "forbidden",
        }
        for field in sorted(required_boundary_fields):
            if not recursion_boundary.get(field):
                findings.append(
                    self.finding(
                        "critical",
                        "meta_layer_recursion_boundary_metadata_missing",
                        self.relative(self.contract_path),
                        f"recursion_boundary must declare {field}.",
                    )
                )
        expected_root = self.relative(self.contract_path)
        if recursion_boundary.get("root_contract") and recursion_boundary.get("root_contract") != expected_root:
            findings.append(
                self.finding(
                    "critical",
                    "meta_layer_recursion_root_drift",
                    self.relative(self.contract_path),
                    f"recursion_boundary.root_contract must be {expected_root!r}.",
                )
            )
        allowed_meta_contracts = recursion_boundary.get("allowed_meta_contracts")
        if allowed_meta_contracts is not None:
            if not isinstance(allowed_meta_contracts, list) or not allowed_meta_contracts:
                findings.append(
                    self.finding(
                        "critical",
                        "meta_layer_allowed_meta_contracts_invalid",
                        self.relative(self.contract_path),
                        "recursion_boundary.allowed_meta_contracts must be a non-empty list.",
                    )
                )
            elif expected_root not in allowed_meta_contracts:
                findings.append(
                    self.finding(
                        "critical",
                        "meta_layer_allowed_meta_contracts_missing_root",
                        self.relative(self.contract_path),
                        "recursion_boundary.allowed_meta_contracts must include the root meta-layer contract.",
                    )
                )
        for field in ("new_contract_admission", "forbidden"):
            values = recursion_boundary.get(field)
            if values is not None and (
                not isinstance(values, list)
                or not values
                or any(not isinstance(value, str) or not value.strip() for value in values)
            ):
                findings.append(
                    self.finding(
                        "critical",
                        "meta_layer_recursion_boundary_list_invalid",
                        self.relative(self.contract_path),
                        f"recursion_boundary.{field} must be a non-empty list of strings.",
                    )
                )

        symbolic_vocabulary = contract.get("symbolic_vocabulary")
        if not isinstance(symbolic_vocabulary, dict) or not symbolic_vocabulary:
            findings.append(
                self.finding(
                    "critical",
                    "meta_layer_symbolic_vocabulary_missing",
                    self.relative(self.contract_path),
                    "meta-layer-contract-v1 must declare symbolic_vocabulary.",
                )
            )
            symbolic_vocabulary = {}

        precedence = contract.get("precedence")
        if not isinstance(precedence, list) or not precedence:
            findings.append(
                self.finding(
                    "critical",
                    "meta_layer_precedence_missing",
                    self.relative(self.contract_path),
                    "meta-layer-contract-v1 must declare a non-empty precedence list.",
                )
            )
            precedence = []

        layers = contract.get("layers")
        if not isinstance(layers, list) or not layers:
            findings.append(
                self.finding(
                    "critical",
                    "meta_layer_contract_layers_missing",
                    self.relative(self.contract_path),
                    "meta-layer-contract-v1 must declare a non-empty layers list.",
                )
            )
            layers = []

        seen_names: set[str] = set()
        for index, layer in enumerate(layers):
            item_path = str(layer.get("name") or f"layers[{index}]")
            for field in (
                "name",
                "canonical_home",
                "engineering_role",
                "agent_question",
                "system_primitives",
                "symbolic_alignment",
                "may_decide",
            ):
                if not layer.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "meta_layer_metadata_missing",
                            item_path,
                            f"layers entries must declare {field}.",
                        )
                    )
            name = str(layer.get("name") or "")
            if name:
                if name in seen_names:
                    findings.append(
                        self.finding(
                            "critical",
                            "meta_layer_duplicate_name",
                            item_path,
                            f"layer {name!r} is declared more than once.",
                        )
                    )
                seen_names.add(name)
            for field in ("system_primitives", "symbolic_alignment", "may_decide"):
                values = layer.get(field)
                if not isinstance(values, list) or not values:
                    findings.append(
                        self.finding(
                            "critical",
                            "meta_layer_list_metadata_invalid",
                            item_path,
                            f"layer field {field} must be a non-empty list.",
                        )
                    )
                    continue
                if any(not isinstance(value, str) or not value.strip() for value in values):
                    findings.append(
                        self.finding(
                            "critical",
                            "meta_layer_list_metadata_invalid",
                            item_path,
                            f"layer field {field} entries must be non-empty strings.",
                        )
                    )
            if len(layer.get("system_primitives", [])) < 3:
                findings.append(
                    self.finding(
                        "critical",
                        "meta_layer_system_primitives_too_weak",
                        item_path,
                        "system_primitives must name at least three concrete OS or engineering primitives.",
                    )
                )
            for symbol in layer.get("symbolic_alignment", []):
                if symbol not in symbolic_vocabulary:
                    findings.append(
                        self.finding(
                            "critical",
                            "meta_layer_unknown_symbolic_alignment",
                            item_path,
                            f"symbolic_alignment references {symbol!r}, absent from symbolic_vocabulary.",
                        )
                    )

        missing_from_layers = [name for name in precedence if name not in seen_names]
        if missing_from_layers:
            findings.append(
                self.finding(
                    "critical",
                    "meta_layer_precedence_target_missing",
                    self.relative(self.contract_path),
                    f"precedence references layers absent from layers: {missing_from_layers!r}.",
                )
            )

        return findings
