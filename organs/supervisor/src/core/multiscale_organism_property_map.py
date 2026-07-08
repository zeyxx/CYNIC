"""Validation service for the multiscale organism property map."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from organs.supervisor.src.core import repo_topology_host_constants as host
from organs.supervisor.src.core.repo_topology_host_validation import Finding


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class MultiscaleOrganismPropertyMapService:
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
                    "multiscale_organism_property_map_missing",
                    self.relative(self.contract_path),
                    "multiscale-organism-property-map-v1.json must exist to describe empirical scale projections.",
                )
            ]

        try:
            contract = self.load_json(self.contract_path)
        except json.JSONDecodeError as exc:
            return [
                self.finding(
                    "critical",
                    "multiscale_organism_property_map_invalid_json",
                    self.relative(self.contract_path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ]

        if contract.get("schema") != "cynic.multiscale_organism_property_map.v1":
            findings.append(
                self.finding(
                    "critical",
                    "unexpected_multiscale_organism_property_map_schema",
                    self.relative(self.contract_path),
                    f"expected cynic.multiscale_organism_property_map.v1, got {contract.get('schema')!r}",
                )
            )

        if contract.get("status") not in {"draft", "normative"}:
            findings.append(
                self.finding(
                    "critical",
                    "multiscale_organism_property_map_status_invalid",
                    self.relative(self.contract_path),
                    "multiscale-organism-property-map-v1 must be draft or normative.",
                )
            )

        if contract.get("meta_contract") != "axioms/meta-layer-contract-v1.json":
            findings.append(
                self.finding(
                    "critical",
                    "multiscale_organism_property_map_meta_contract_invalid",
                    self.relative(self.contract_path),
                    "multiscale-organism-property-map-v1 must point to axioms/meta-layer-contract-v1.json.",
                )
            )

        for field in ("scale_order", "property_nucleus", "validity_rule", "evidence_sources", "scales"):
            if not contract.get(field):
                findings.append(
                    self.finding(
                        "critical",
                        "multiscale_organism_property_map_metadata_missing",
                        self.relative(self.contract_path),
                        f"multiscale-organism-property-map-v1 must declare {field}.",
                    )
                )

        scale_order = contract.get("scale_order", [])
        if "planck_limit" not in scale_order:
            findings.append(
                self.finding(
                    "critical",
                    "multiscale_organism_property_map_planck_missing",
                    self.relative(self.contract_path),
                    "scale_order must include planck_limit as the description boundary.",
                )
            )

        scale_index = {str(name): index for index, name in enumerate(scale_order) if isinstance(name, str)}

        property_nucleus = contract.get("property_nucleus", [])
        for field in ("identity", "boundary", "state", "energy", "flux", "memory", "observation", "action", "dependency", "invariant", "failure_mode"):
            if field not in property_nucleus:
                findings.append(
                    self.finding(
                        "critical",
                        "multiscale_organism_property_map_property_missing",
                        self.relative(self.contract_path),
                        f"property_nucleus must include {field}.",
                    )
                )

        validity_rule = contract.get("validity_rule", {})
        if not isinstance(validity_rule, dict):
            findings.append(
                self.finding(
                    "critical",
                    "multiscale_organism_property_map_validity_rule_invalid",
                    self.relative(self.contract_path),
                    "validity_rule must be an object.",
                )
            )
        elif validity_rule.get("limit_semantics") != "description_limit_only":
            findings.append(
                self.finding(
                    "critical",
                    "multiscale_organism_property_map_limit_semantics_invalid",
                    self.relative(self.contract_path),
                    "Planck limit must remain a description limit only.",
                )
            )

        scales = contract.get("scales")
        if not isinstance(scales, list) or not scales:
            findings.append(
                self.finding(
                    "critical",
                    "multiscale_organism_property_map_scales_missing",
                    self.relative(self.contract_path),
                    "multiscale-organism-property-map-v1 must declare a non-empty scales list.",
                )
            )
            scales = []

        for index, item in enumerate(scales):
            item_path = str(item.get("name") or f"scales[{index}]")
            for field in ("name", "representation", "evidence", "projection_kind"):
                if not item.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "multiscale_organism_property_map_scale_metadata_missing",
                            item_path,
                            f"scales entries must declare {field}.",
                        )
                    )
            parent_scale = item.get("parent_scale")
            name = str(item.get("name") or "")
            if name == "organism":
                if parent_scale not in (None, ""):
                    findings.append(
                        self.finding(
                            "critical",
                            "multiscale_organism_property_map_root_parent_invalid",
                            item_path,
                            "organism must not declare a parent_scale.",
                        )
                    )
            elif not parent_scale:
                findings.append(
                    self.finding(
                        "critical",
                        "multiscale_organism_property_map_parent_scale_missing",
                        item_path,
                        "non-root scales must declare a parent_scale.",
                    )
                )
            elif parent_scale not in scale_index:
                findings.append(
                    self.finding(
                        "critical",
                        "multiscale_organism_property_map_parent_scale_unknown",
                        item_path,
                        f"parent_scale {parent_scale!r} is not declared in scale_order.",
                    )
                )
            elif scale_index[str(parent_scale)] >= scale_index.get(name, -1):
                findings.append(
                    self.finding(
                        "critical",
                        "multiscale_organism_property_map_parent_scale_order_invalid",
                        item_path,
                        f"parent_scale {parent_scale!r} must be more abstract than scale {name!r}.",
                    )
                )
            representation = item.get("representation")
            if isinstance(representation, dict):
                for field in ("identity", "boundary", "state", "energy", "flux", "memory", "observation", "action", "dependency", "invariant", "failure_mode"):
                    if not representation.get(field):
                        findings.append(
                            self.finding(
                                "critical",
                                "multiscale_organism_property_map_scale_projection_missing",
                                item_path,
                                f"scale projection must declare {field}.",
                            )
                        )
            evidence = item.get("evidence")
            if isinstance(evidence, list):
                for ref in evidence:
                    ref_text = str(ref)
                    if any(ref_text.endswith(ext) for ext in (".md", ".json", ".yaml", ".yml", ".rs", ".py")):
                        if not (self.workspace_root / ref_text).exists():
                            findings.append(
                                self.finding(
                                    "critical",
                                    "multiscale_organism_property_map_evidence_missing",
                                    item_path,
                                    f"evidence reference {ref_text!r} does not exist.",
                                )
                            )

        return findings
