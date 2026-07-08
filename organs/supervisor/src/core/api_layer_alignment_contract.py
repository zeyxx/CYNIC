"""Compatibility mirror for the CYNIC API layer alignment contract validation service.

Stable deterministic authority is migrating into crates/cynic-mcp. Keep this
module as a Python compatibility mirror until remaining callers are ported.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class ApiLayerAlignmentContractService:
    workspace_root: Path
    contract_path: Path
    topology_path: Path
    meta_layer_path: Path
    organ_layer_path: Path
    load_json: Callable[[Path], Any]
    relative: Callable[[Path], str]
    finding: FindingFactory

    def _load_contract(self, path: Path, *, kind: str) -> tuple[Any | None, list[Any]]:
        if not path.exists():
            return None, [
                self.finding(
                    "critical",
                    f"{kind}_missing",
                    self.relative(path),
                    f"{self.relative(path)} must exist for API layer alignment validation.",
                )
            ]
        try:
            return self.load_json(path), []
        except json.JSONDecodeError as exc:
            return None, [
                self.finding(
                    "critical",
                    f"{kind}_invalid_json",
                    self.relative(path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]

    def _symbolic_vocabulary(
        self,
        contract: Any,
        *,
        path: Path,
        kind: str,
    ) -> tuple[dict[str, str], list[Any]]:
        if not isinstance(contract, dict):
            return {}, []
        vocabulary = contract.get("symbolic_vocabulary")
        if not isinstance(vocabulary, dict) or not vocabulary:
            return {}, [
                self.finding(
                    "critical",
                    f"{kind}_symbolic_vocabulary_missing",
                    self.relative(path),
                    f"{self.relative(path)} must declare symbolic_vocabulary.",
                )
            ]
        invalid = [
            key
            for key, value in vocabulary.items()
            if not isinstance(key, str)
            or not key.strip()
            or not isinstance(value, str)
            or not value.strip()
        ]
        if invalid:
            return {}, [
                self.finding(
                    "critical",
                    f"{kind}_symbolic_vocabulary_invalid",
                    self.relative(path),
                    f"symbolic_vocabulary contains invalid entries: {invalid!r}.",
                )
            ]
        return {str(key): str(value) for key, value in vocabulary.items()}, []

    def _validate_symbolic_compatibility(
        self,
        api_vocabulary: dict[str, str],
        peer_vocabulary: dict[str, str],
        *,
        peer_path: Path,
        peer_name: str,
    ) -> list[Any]:
        findings: list[Any] = []
        api_symbols = set(api_vocabulary)
        peer_symbols = set(peer_vocabulary)
        missing_from_api = sorted(peer_symbols - api_symbols)
        missing_from_peer = sorted(api_symbols - peer_symbols)
        if missing_from_api:
            findings.append(
                self.finding(
                    "critical",
                    "api_layer_symbolic_vocabulary_missing_peer_symbols",
                    self.relative(self.contract_path),
                    f"API layer alignment is missing symbols from {peer_name}: {missing_from_api!r}.",
                )
            )
        if missing_from_peer:
            findings.append(
                self.finding(
                    "critical",
                    "api_layer_symbolic_vocabulary_peer_missing_symbols",
                    self.relative(peer_path),
                    f"{peer_name} is missing API layer symbols: {missing_from_peer!r}.",
                )
            )
        return findings

    def validate(self) -> list[Any]:
        findings: list[Any] = []
        contract, contract_findings = self._load_contract(
            self.contract_path,
            kind="api_layer_alignment_contract",
        )
        findings.extend(contract_findings)
        if not isinstance(contract, dict):
            return findings

        if contract.get("schema") != "cynic.api_layer_alignment.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_api_layer_alignment_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.api_layer_alignment.v1, got {contract.get('schema')!r}",
                )
            )
        if contract.get("status") != "normative":
            findings.append(
                self.finding(
                    "critical",
                    "api_layer_alignment_not_normative",
                    self.relative(self.contract_path),
                    "api-layer-alignment-v1 must be normative before agents can use it for API geometry.",
                )
            )

        topology, topology_findings = self._load_contract(
            self.topology_path,
            kind="api_topology_contract",
        )
        findings.extend(topology_findings)
        meta_layer, meta_findings = self._load_contract(
            self.meta_layer_path,
            kind="meta_layer_contract",
        )
        findings.extend(meta_findings)
        organ_layer, organ_findings = self._load_contract(
            self.organ_layer_path,
            kind="organ_layer_topology_contract",
        )
        findings.extend(organ_findings)

        if isinstance(topology, dict):
            source = topology.get("source_of_truth", {})
            if source.get("layer_alignment") != self.relative(self.contract_path):
                findings.append(
                    self.finding(
                        "critical",
                        "api_topology_layer_alignment_source_drift",
                        self.relative(self.topology_path),
                        f"source_of_truth.layer_alignment must be {self.relative(self.contract_path)!r}.",
                    )
                )

        api_vocabulary, vocabulary_findings = self._symbolic_vocabulary(
            contract,
            path=self.contract_path,
            kind="api_layer_alignment",
        )
        findings.extend(vocabulary_findings)
        for peer_contract, peer_path, peer_name in (
            (meta_layer, self.meta_layer_path, "meta-layer-contract-v1"),
            (organ_layer, self.organ_layer_path, "organ-layer-topology-v1"),
        ):
            peer_vocabulary, peer_findings = self._symbolic_vocabulary(
                peer_contract,
                path=peer_path,
                kind=peer_name.replace("-", "_"),
            )
            findings.extend(peer_findings)
            if api_vocabulary and peer_vocabulary:
                findings.extend(
                    self._validate_symbolic_compatibility(
                        api_vocabulary,
                        peer_vocabulary,
                        peer_path=peer_path,
                        peer_name=peer_name,
                    )
                )

        layers = contract.get("layers")
        if not isinstance(layers, dict) or not layers:
            findings.append(
                self.finding(
                    "critical",
                    "api_layer_alignment_layers_missing",
                    self.relative(self.contract_path),
                    "api-layer-alignment-v1 must declare non-empty layers.",
                )
            )
            layers = {}
        if isinstance(topology, dict):
            allowed_layers = set(topology.get("allowed", {}).get("layers", []))
            alignment_layers = set(layers)
            for layer in sorted(allowed_layers - alignment_layers):
                findings.append(
                    self.finding(
                        "critical",
                        "api_layer_alignment_layer_missing",
                        self.relative(self.contract_path),
                        f"missing alignment for API topology layer {layer!r}.",
                    )
                )
            for layer in sorted(alignment_layers - allowed_layers):
                findings.append(
                    self.finding(
                        "critical",
                        "api_layer_alignment_layer_unknown",
                        self.relative(self.contract_path),
                        f"alignment declares layer {layer!r}, absent from api topology allowed.layers.",
                    )
                )

        required_fields = {
            "engineering_role",
            "agent_question",
            "system_primitives",
            "symbolic_alignment",
            "must",
            "must_not",
        }
        for layer_name, layer in sorted(layers.items()):
            if not isinstance(layer, dict):
                findings.append(
                    self.finding(
                        "critical",
                        "api_layer_alignment_layer_invalid",
                        str(layer_name),
                        "alignment layers must be objects.",
                    )
                )
                continue
            for field in sorted(required_fields):
                if not layer.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "api_layer_alignment_metadata_missing",
                            str(layer_name),
                            f"alignment layer must declare {field}.",
                        )
                    )
            for field in ("system_primitives", "symbolic_alignment", "must", "must_not"):
                values = layer.get(field)
                if not isinstance(values, list) or not values:
                    findings.append(
                        self.finding(
                            "critical",
                            "api_layer_alignment_list_metadata_invalid",
                            str(layer_name),
                            f"alignment field {field} must be a non-empty list.",
                        )
                    )
                    continue
                if any(not isinstance(value, str) or not value.strip() for value in values):
                    findings.append(
                        self.finding(
                            "critical",
                            "api_layer_alignment_list_metadata_invalid",
                            str(layer_name),
                            f"alignment field {field} entries must be non-empty strings.",
                        )
                    )
            if len(layer.get("system_primitives", [])) < 3:
                findings.append(
                    self.finding(
                        "critical",
                        "api_layer_alignment_system_primitives_too_weak",
                        str(layer_name),
                        "system_primitives must name at least three concrete OS or engineering primitives.",
                    )
                )
            for symbol in layer.get("symbolic_alignment", []):
                if api_vocabulary and symbol not in api_vocabulary:
                    findings.append(
                        self.finding(
                            "critical",
                            "api_layer_alignment_unknown_symbol",
                            str(layer_name),
                            f"symbolic_alignment references {symbol!r}, absent from symbolic_vocabulary.",
                        )
                    )

        if not contract.get("guardrails"):
            findings.append(
                self.finding(
                    "critical",
                    "api_layer_alignment_guardrails_missing",
                    self.relative(self.contract_path),
                    "api-layer-alignment-v1 must declare guardrails that keep analogy below executable evidence.",
                )
            )
        if not contract.get("vertical_alignment_rules"):
            findings.append(
                self.finding(
                    "critical",
                    "api_layer_alignment_vertical_rules_missing",
                    self.relative(self.contract_path),
                    "api-layer-alignment-v1 must declare vertical_alignment_rules.",
                )
            )

        return findings
