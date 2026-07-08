"""Validation service for the organ layer topology contract."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from organs.supervisor.src.core import repo_topology_host_constants as host
from organs.supervisor.src.core.repo_topology_host_validation import Finding


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class OrganLayerTopologyService:
    workspace_root: Path
    contract_path: Path
    organs_root: Path
    load_json: Callable[[Path], Any]
    relative: Callable[[Path], str]
    finding: FindingFactory

    def _layer_by_id(self, layers: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
        mapping: dict[str, dict[str, Any]] = {}
        for index, layer in enumerate(layers):
            layer_id = str(layer.get("id") or f"layers[{index}]")
            mapping[layer_id] = layer
        return mapping

    def validate(self) -> list[Any]:
        findings: list[Any] = []
        if not self.contract_path.exists():
            return [
                self.finding(
                    "critical",
                    "organ_layer_topology_contract_missing",
                    self.relative(self.contract_path),
                    "organ-layer-topology-v1.json must classify every immediate organ by layer and rank.",
                )
            ]

        try:
            contract = self.load_json(self.contract_path)
        except json.JSONDecodeError as exc:
            return [
                self.finding(
                    "critical",
                    "organ_layer_topology_contract_invalid_json",
                    self.relative(self.contract_path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]

        if contract.get("schema") != "cynic.organ_layer_topology.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_organ_layer_topology_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.organ_layer_topology.v1, got {contract.get('schema')!r}",
                )
            )
        if contract.get("status") != "normative":
            findings.append(
                self.finding(
                    "critical",
                    "organ_layer_topology_not_normative",
                    self.relative(self.contract_path),
                    "organ layer topology must be normative before it can gate organ rank claims.",
                )
            )

        for name, value in sorted(contract.get("authorities", {}).items()):
            authority = self.workspace_root / value
            if not authority.exists():
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_topology_authority_missing",
                        self.relative(authority),
                        f"authority {name!r} is declared by organ-layer-topology-v1 but does not exist.",
                    )
                )

        layer_policy = contract.get("layer_policy")
        if not isinstance(layer_policy, dict):
            findings.append(
                self.finding(
                    "critical",
                    "organ_layer_policy_missing",
                    self.relative(self.contract_path),
                    "organ-layer-topology-v1 must declare layer_policy.",
                )
            )
            layer_policy = {}
        for field in ("ordering", "rank_meaning", "required_fields"):
            if not layer_policy.get(field):
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_policy_metadata_missing",
                        self.relative(self.contract_path),
                        f"layer_policy must declare {field}.",
                    )
                )
        symbolic_vocabulary = contract.get("symbolic_vocabulary")
        if not isinstance(symbolic_vocabulary, dict) or not symbolic_vocabulary:
            findings.append(
                self.finding(
                    "critical",
                    "organ_layer_symbolic_vocabulary_missing",
                    self.relative(self.contract_path),
                    "organ-layer-topology-v1 must declare symbolic_vocabulary.",
                )
            )
            symbolic_vocabulary = {}

        layers = contract.get("layers")
        if not isinstance(layers, list) or not layers:
            findings.append(
                self.finding(
                    "critical",
                    "organ_layer_topology_layers_missing",
                    self.relative(self.contract_path),
                    "organ-layer-topology-v1 must declare a non-empty layers list.",
                )
            )
            layers = []

        layer_by_id: dict[str, dict[str, Any]] = {}
        layer_order: dict[str, int] = {}
        seen_layer_ids: set[str] = set()
        seen_ranks: set[int] = set()
        for index, layer in enumerate(layers):
            item_path = str(layer.get("id") or f"layers[{index}]")
            for field in sorted(host.ORGAN_LAYER_REQUIRED_FIELDS):
                if not layer.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_metadata_missing",
                            item_path,
                            f"layers entries must declare {field}.",
                        )
                    )
            layer_id = str(layer.get("id") or "")
            for field in ("system_primitives", "symbolic_alignment", "must_not_own", "canonical_examples"):
                values = layer.get(field)
                if not isinstance(values, list) or not values:
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_list_metadata_invalid",
                            item_path,
                            f"layer field {field} must be a non-empty list.",
                        )
                    )
                    continue
                if any(not isinstance(value, str) or not value.strip() for value in values):
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_list_metadata_invalid",
                            item_path,
                            f"layer field {field} entries must be non-empty strings.",
                        )
                    )
            if len(layer.get("system_primitives", [])) < 3:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_system_primitives_too_weak",
                        item_path,
                        "system_primitives must name at least three concrete OS or engineering primitives.",
                    )
                )
            for symbol in layer.get("symbolic_alignment", []):
                if symbol not in symbolic_vocabulary:
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_unknown_symbolic_alignment",
                            item_path,
                            f"symbolic_alignment references {symbol!r}, absent from symbolic_vocabulary.",
                        )
                    )
            if layer_id:
                if layer_id in seen_layer_ids:
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_duplicate_id",
                            item_path,
                            f"layer id {layer_id!r} is declared more than once.",
                        )
                    )
                seen_layer_ids.add(layer_id)
                layer_by_id[layer_id] = layer
                layer_order[layer_id] = index
            rank = layer.get("rank")
            if not isinstance(rank, int):
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_rank_invalid",
                        item_path,
                        "layer rank must be an integer.",
                    )
                )
            elif rank in seen_ranks:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_duplicate_rank",
                        item_path,
                        f"layer rank {rank!r} is declared more than once.",
                    )
                )
            elif rank <= 0:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_rank_non_positive",
                        item_path,
                        "layer rank must be positive.",
                    )
                )
            else:
                seen_ranks.add(rank)

        for index, layer in enumerate(layers):
            item_path = str(layer.get("id") or f"layers[{index}]")
            parent_layer = layer.get("parent_layer")
            layer_id = str(layer.get("id") or "")
            if layer_id == "foundational_contract":
                if parent_layer not in (None, ""):
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_root_parent_invalid",
                            item_path,
                            "foundational_contract must not declare a parent_layer.",
                        )
                    )
                continue
            if not parent_layer:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_parent_missing",
                        item_path,
                        "non-root layers must declare parent_layer.",
                    )
                )
                continue
            if parent_layer not in layer_by_id:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_parent_unknown",
                        item_path,
                        f"parent_layer {parent_layer!r} is not declared in layers.",
                    )
                )
                continue
            if layer_order[str(parent_layer)] >= layer_order[layer_id]:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_parent_order_invalid",
                        item_path,
                        f"parent_layer {parent_layer!r} must appear before layer {layer_id!r} in ascending abstraction order.",
                    )
                )

        organs = contract.get("organs")
        if not isinstance(organs, list) or not organs:
            findings.append(
                self.finding(
                    "critical",
                    "organ_layer_topology_organs_missing",
                    self.relative(self.contract_path),
                    "organ-layer-topology-v1 must declare a non-empty organs list.",
                )
            )
            organs = []

        actual_organs = {
            self.relative(path).rstrip("/") + "/"
            for path in self.organs_root.iterdir()
            if path.is_dir()
        } if self.organs_root.exists() else set()
        inventory_paths: set[str] = set()
        by_layer: dict[str, int] = {}
        for index, item in enumerate(organs):
            item_path = str(item.get("path") or f"organs[{index}]")
            for field in sorted(host.ORGAN_LAYER_ENTRY_REQUIRED_FIELDS):
                if not item.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_entry_metadata_missing",
                            item_path,
                            f"organs entries must declare {field}.",
                        )
                    )
            path_value = item.get("path")
            if path_value:
                path = str(path_value).rstrip("/") + "/"
                inventory_paths.add(path)
                if path.count("/") != 2 or not path.startswith("organs/"):
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_path_not_immediate_organ",
                            path,
                            "Organ layer inventory entries must target immediate organs/<role>/ directories.",
                        )
                    )
                if not (self.workspace_root / path).is_dir():
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_path_missing",
                            path,
                            "Declared organ layer entry points to a directory that does not exist.",
                        )
                    )
            layer_id = str(item.get("layer_id") or "")
            layer = layer_by_id.get(layer_id)
            if not layer:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_unknown_layer",
                        item_path,
                        f"layer_id {layer_id!r} is not declared in layers.",
                    )
                )
            layer_rank = item.get("layer_rank")
            if isinstance(layer_rank, int):
                by_layer[layer_id] = by_layer.get(layer_id, 0) + 1
                if layer and layer.get("rank") != layer_rank:
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_layer_rank_mismatch",
                            item_path,
                            f"layer_rank {layer_rank!r} does not match layer {layer_id!r} rank {layer.get('rank')!r}.",
                        )
                    )
            else:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_rank_entry_invalid",
                        item_path,
                        "organ entries must declare an integer layer_rank.",
                    )
                )

        for path in sorted(actual_organs - inventory_paths):
            findings.append(
                self.finding(
                    "critical",
                    "organ_layer_missing_organ_entry",
                    path,
                    "Every immediate organ directory must appear in organ-layer-topology-v1.",
                )
            )
        for path in sorted(inventory_paths - actual_organs):
            findings.append(
                self.finding(
                    "critical",
                    "organ_layer_stale_path",
                    path,
                    "Organ layer inventory contains a path that is not an immediate organ directory.",
                )
            )

        totals = contract.get("totals", {})
        if totals:
            declared_layers = totals.get("layers")
            if declared_layers is not None and declared_layers != len(layers):
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_total_layer_mismatch",
                        self.relative(self.contract_path),
                        f"totals.layers={declared_layers!r} but inventory contains {len(layers)} layer definitions.",
                    )
                )
            declared_organs = totals.get("organs")
            if declared_organs is not None and declared_organs != len(organs):
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_total_organ_mismatch",
                        self.relative(self.contract_path),
                        f"totals.organs={declared_organs!r} but inventory contains {len(organs)} organ entries.",
                    )
                )
            declared_by_layer = totals.get("by_layer", {})
            if declared_by_layer and {str(key): value for key, value in declared_by_layer.items()} != by_layer:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_layer_by_layer_total_mismatch",
                        self.relative(self.contract_path),
                        f"totals.by_layer does not match actual inventory counts {by_layer}.",
                    )
                )

        return findings
