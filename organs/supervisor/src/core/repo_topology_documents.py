#!/usr/bin/env python3
"""Repo topology checks for document, evidence, and theme surfaces."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any

import organs.supervisor.src.core.repo_topology_host as host


def _exists_or_tracked(path: str) -> bool:
    if (host.WORKSPACE_ROOT / path).exists():
        return True
    try:
        return path in set(host.git_ls_files(path))
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def validate_theme_topology_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not host.THEME_CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "theme_topology_contract_missing",
                host.relative(host.THEME_CONTRACT_PATH),
                "organism-theme-topology-v1.json is the machine contract for thematic ownership.",
            )
        ]

    try:
        contract = host.load_json(host.THEME_CONTRACT_PATH)
    except json.JSONDecodeError as exc:
        return [
            host.Finding(
                "critical",
                "theme_topology_contract_invalid_json",
                host.relative(host.THEME_CONTRACT_PATH),
                f"{exc.msg} at line {exc.lineno} column {exc.colno}",
            )
        ]

    if contract.get("schema") != "cynic.organism_theme_topology.v1":
        findings.append(
            host.Finding(
                "critical",
                "unexpected_theme_topology_schema",
                host.relative(host.THEME_CONTRACT_PATH),
                f"expected cynic.organism_theme_topology.v1, got {contract.get('schema')!r}",
            )
        )

    for name, value in sorted(contract.get("authorities", {}).items()):
        authority = host.WORKSPACE_ROOT / value
        if not authority.exists():
            findings.append(
                host.Finding(
                    "critical",
                    "theme_topology_authority_missing",
                    host.relative(authority),
                    f"authority {name!r} is declared by organism-theme-topology-v1 but does not exist.",
                )
            )

    themes = contract.get("themes")
    if not isinstance(themes, list) or not themes:
        findings.append(
            host.Finding(
                "critical",
                "theme_topology_themes_missing",
                host.relative(host.THEME_CONTRACT_PATH),
                "organism-theme-topology-v1 must declare a non-empty themes list.",
            )
        )
        themes = []

    seen_theme_ids: set[str] = set()
    canonical_owner_by_path: dict[str, str] = {}
    shared_sources = set(contract.get("shared_canonical_sources", []))
    for index, theme in enumerate(themes):
        item_path = str(theme.get("id") or f"themes[{index}]")
        for field in sorted(host.THEME_REQUIRED_FIELDS):
            if not theme.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "theme_topology_theme_metadata_missing",
                        item_path,
                        f"themes entries must declare {field}.",
                    )
                )
        theme_id = str(theme.get("id", ""))
        if theme_id:
            if theme_id in seen_theme_ids:
                findings.append(
                    host.Finding(
                        "critical",
                        "theme_topology_duplicate_theme_id",
                        item_path,
                        f"theme id {theme_id!r} is declared more than once.",
                    )
                )
            seen_theme_ids.add(theme_id)

        for source in theme.get("canonical_sources", []):
            source_path = str(source)
            if not _exists_or_tracked(source_path):
                findings.append(
                    host.Finding(
                        "critical",
                        "theme_topology_canonical_source_missing",
                        source_path,
                        f"canonical source declared by theme {theme_id!r} does not exist.",
                    )
                )
            previous_owner = canonical_owner_by_path.get(source_path)
            if previous_owner and source_path not in shared_sources:
                findings.append(
                    host.Finding(
                        "critical",
                        "theme_topology_canonical_source_ambiguous",
                        source_path,
                        f"canonical source is owned by both {previous_owner!r} and {theme_id!r}; add shared_canonical_sources only when the overlap is intentional.",
                    )
                )
            canonical_owner_by_path[source_path] = theme_id

    doc_zones = contract.get("doc_zones")
    if not isinstance(doc_zones, list) or not doc_zones:
        findings.append(
            host.Finding(
                "critical",
                "theme_topology_doc_zones_missing",
                host.relative(host.THEME_CONTRACT_PATH),
                "organism-theme-topology-v1 must declare a non-empty doc_zones list.",
            )
        )
        doc_zones = []

    for index, zone in enumerate(doc_zones):
        item_path = str(zone.get("path") or f"doc_zones[{index}]")
        for field in sorted(host.DOC_ZONE_REQUIRED_FIELDS):
            if not zone.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "theme_topology_doc_zone_metadata_missing",
                        item_path,
                        f"doc_zones entries must declare {field}.",
                    )
                )
        path_value = zone.get("path")
        if path_value and not (host.WORKSPACE_ROOT / str(path_value)).exists():
            findings.append(
                host.Finding(
                    "medium",
                    "theme_topology_doc_zone_missing",
                    str(path_value),
                    "Declared thematic document zone does not exist yet.",
                )
            )

    promotion = contract.get("promotion_rule")
    if not isinstance(promotion, dict) or not promotion.get("from") or not promotion.get("to") or not promotion.get("requires"):
        findings.append(
            host.Finding(
                "critical",
                "theme_topology_promotion_rule_incomplete",
                host.relative(host.THEME_CONTRACT_PATH),
                "promotion_rule must declare from, to and requires lists.",
            )
        )

    return findings


def validate_document_theme_inventory_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not host.DOCUMENT_THEME_INVENTORY_PATH.exists():
        return [
            host.Finding(
                "critical",
                "document_theme_inventory_missing",
                host.relative(host.DOCUMENT_THEME_INVENTORY_PATH),
                "document-theme-inventory-v1.json must classify sensitive scattered docs by theme.",
            )
        ]

    try:
        inventory = host.load_json(host.DOCUMENT_THEME_INVENTORY_PATH)
    except json.JSONDecodeError as exc:
        return [
            host.Finding(
                "critical",
                "document_theme_inventory_invalid_json",
                host.relative(host.DOCUMENT_THEME_INVENTORY_PATH),
                f"{exc.msg} at line {exc.lineno} column {exc.colno}",
            )
        ]

    if inventory.get("schema") != "cynic.document_theme_inventory.v1":
        findings.append(
            host.Finding(
                "critical",
                "unexpected_document_theme_inventory_schema",
                host.relative(host.DOCUMENT_THEME_INVENTORY_PATH),
                f"expected cynic.document_theme_inventory.v1, got {inventory.get('schema')!r}",
            )
        )

    try:
        theme_contract = host.load_json(host.THEME_CONTRACT_PATH)
    except (json.JSONDecodeError, FileNotFoundError):
        theme_contract = {}
    theme_ids = {str(item.get("id")) for item in theme_contract.get("themes", []) if item.get("id")}
    canonical_doc_zone_prefixes = [
        str(item.get("path"))
        for item in theme_contract.get("doc_zones", [])
        if item.get("path") and item.get("canonicality") in host.DOCUMENT_CANONICAL_ZONE_VALUES
    ]
    allowed_statuses = set(inventory.get("allowed_statuses", []))
    status_transition_policy = inventory.get("status_transition_policy") or {}
    if not isinstance(status_transition_policy, dict):
        findings.append(
            host.Finding(
                "critical",
                "document_theme_inventory_status_transition_policy_missing",
                host.relative(host.DOCUMENT_THEME_INVENTORY_PATH),
                "status_transition_policy must be an object keyed by transitional document status when provided.",
            )
        )
        status_transition_policy = {}
    documents = inventory.get("documents", [])
    if not isinstance(documents, list):
        findings.append(
            host.Finding(
                "critical",
                "document_theme_inventory_documents_not_list",
                host.relative(host.DOCUMENT_THEME_INVENTORY_PATH),
                "documents must be a list.",
            )
        )
        documents = []

    inventory_paths: set[str] = set()
    inventory_prefixes: set[str] = set()
    for index, item in enumerate(documents):
        item_path = str(item.get("path") or f"documents[{index}]")
        for field in sorted(host.DOCUMENT_INVENTORY_REQUIRED_FIELDS):
            if not item.get(field):
                findings.append(
                    host.Finding(
                        "critical",
                        "document_theme_inventory_metadata_missing",
                        item_path,
                        f"documents entries must declare {field}.",
                    )
                )
        path_value = item.get("path")
        if path_value:
            path = str(path_value)
            inventory_paths.add(path)
            if path.endswith("/"):
                inventory_prefixes.add(path)
            if not _exists_or_tracked(path):
                findings.append(
                    host.Finding(
                        "critical",
                        "document_theme_inventory_path_missing",
                        path,
                        "Inventory entry points to a file that does not exist.",
                    )
                )
        theme = item.get("theme")
        if theme_ids and theme and str(theme) not in theme_ids:
            findings.append(
                host.Finding(
                    "critical",
                    "document_theme_inventory_theme_unknown",
                    item_path,
                    f"theme {theme!r} is not declared in organism-theme-topology-v1.",
                )
            )
        status = item.get("status")
        if allowed_statuses and status and str(status) not in allowed_statuses:
            findings.append(
                host.Finding(
                    "critical",
                    "document_theme_inventory_status_unknown",
                    item_path,
                    f"status {status!r} is not declared in allowed_statuses.",
                )
            )
        if status in host.DOCUMENT_INVENTORY_TRANSIENT_STATUSES:
            transition = status_transition_policy.get(str(status))
            if not isinstance(transition, dict):
                findings.append(
                    host.Finding(
                        "critical",
                        "document_theme_inventory_status_transition_missing",
                        item_path,
                        f"status {status!r} must have a status_transition_policy entry.",
                    )
                )
            else:
                for field in sorted(host.DOCUMENT_INVENTORY_STATUS_TRANSITION_REQUIRED_FIELDS):
                    if not transition.get(field):
                        findings.append(
                            host.Finding(
                                "critical",
                                "document_theme_inventory_status_transition_metadata_missing",
                                item_path,
                                f"status_transition_policy[{status!r}] must declare {field}.",
                            )
                        )
        if status == "canonical" and canonical_doc_zone_prefixes:
            path = str(item.get("path") or "")
            if not any(path.startswith(prefix) for prefix in canonical_doc_zone_prefixes):
                findings.append(
                    host.Finding(
                        "critical",
                        "document_theme_inventory_canonical_outside_canonical_zone",
                        item_path,
                        "Documents may be marked canonical only inside theme doc_zones whose canonicality is canonical_when_indexed or canonical_by_schema_and_status.",
                    )
                )

    try:
        visible_paths = sorted(set(host.git_ls_files()) | set(host.git_ls_untracked()))
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            host.Finding(
                "critical",
                "document_theme_inventory_scan_failed",
                ".",
                f"unable to query git visible files: {exc}",
            )
        ]

    for path in visible_paths:
        if not path.endswith(".md"):
            continue
        if not (host.WORKSPACE_ROOT / path).exists():
            continue
        if not any(path.startswith(root) for root in host.DOCUMENT_INVENTORY_SENSITIVE_MARKDOWN_ROOTS):
            continue
        if path not in inventory_paths and not any(path.startswith(prefix) for prefix in inventory_prefixes):
            findings.append(
                host.Finding(
                    "critical",
                    "sensitive_markdown_missing_theme_inventory",
                    path,
                    "Sensitive markdown files must be classified in document-theme-inventory-v1.json before they can enter the repo.",
                )
            )

    return findings


def document_inventory_entries() -> list[dict[str, Any]]:
    try:
        inventory = host.load_json(host.DOCUMENT_THEME_INVENTORY_PATH)
    except (json.JSONDecodeError, FileNotFoundError):
        return []
    documents = inventory.get("documents", [])
    if not isinstance(documents, list):
        return []
    return [item for item in documents if isinstance(item, dict)]


def data_emergence_capsule_roots(root: Path, depth: int) -> list[Path]:
    if not root.exists():
        return []
    roots: list[Path] = []

    def walk(path: Path, remaining: int) -> None:
        if remaining == 0:
            if path.is_dir():
                roots.append(path)
            return
        for child in sorted(item for item in path.iterdir() if item.is_dir()):
            walk(child, remaining - 1)

    walk(root, depth)
    return roots


def validate_evidence_capsule_contract() -> list[host.Finding]:
    findings: list[host.Finding] = []
    if not host.EVIDENCE_CAPSULE_CONTRACT_PATH.exists():
        return [
            host.Finding(
                "critical",
                "evidence_capsule_contract_missing",
                host.relative(host.EVIDENCE_CAPSULE_CONTRACT_PATH),
                "evidence-capsule-v1.json must define the machine membrane for data/emergence snapshots.",
            )
        ]

    try:
        contract = host.load_json(host.EVIDENCE_CAPSULE_CONTRACT_PATH)
    except json.JSONDecodeError as exc:
        return [
            host.Finding(
                "critical",
                "evidence_capsule_contract_invalid_json",
                host.relative(host.EVIDENCE_CAPSULE_CONTRACT_PATH),
                f"{exc.msg} at line {exc.lineno} column {exc.colno}",
            )
        ]

    if contract.get("schema") != "cynic.evidence_capsule.v1":
        findings.append(
            host.Finding(
                "critical",
                "unexpected_evidence_capsule_schema",
                host.relative(host.EVIDENCE_CAPSULE_CONTRACT_PATH),
                f"expected cynic.evidence_capsule.v1, got {contract.get('schema')!r}",
            )
        )

    for field in sorted(host.EVIDENCE_CAPSULE_REQUIRED_FIELDS):
        if not contract.get(field):
            findings.append(
                host.Finding(
                    "critical",
                    "evidence_capsule_contract_metadata_missing",
                    host.relative(host.EVIDENCE_CAPSULE_CONTRACT_PATH),
                    f"evidence-capsule-v1.json must declare {field}.",
                )
            )

    root_value = str(contract.get("root") or "data/emergence/").rstrip("/") + "/"
    depth = int(contract.get("capsule_depth") or 2)
    manifest_name = str(contract.get("manifest_name") or "MANIFEST.json")
    required_manifest_fields = set(contract.get("manifest_required_fields") or [])
    inventory_requirements = contract.get("document_inventory_requirements") or {}
    required_theme = inventory_requirements.get("theme")
    required_status = inventory_requirements.get("status")

    inventory_by_path = {str(item.get("path")): item for item in document_inventory_entries() if item.get("path")}
    for capsule_root in data_emergence_capsule_roots(host.WORKSPACE_ROOT / root_value, depth):
        capsule_path = host.relative(capsule_root).rstrip("/") + "/"
        manifest_rel = capsule_path + manifest_name
        manifest_path = host.WORKSPACE_ROOT / manifest_rel
        if not manifest_path.exists():
            findings.append(
                host.Finding(
                    "critical",
                    "evidence_capsule_manifest_missing",
                    capsule_path,
                    f"Evidence capsule roots must contain {manifest_name}.",
                )
            )
            continue

        try:
            manifest = host.load_json(manifest_path)
        except json.JSONDecodeError as exc:
            findings.append(
                host.Finding(
                    "critical",
                    "evidence_capsule_manifest_invalid_json",
                    manifest_rel,
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            )
            continue

        for field in sorted(required_manifest_fields):
            if field not in manifest:
                findings.append(
                    host.Finding(
                        "critical",
                        "evidence_capsule_manifest_metadata_missing",
                        manifest_rel,
                        f"Evidence capsule manifest must declare {field}.",
                    )
                )
        if manifest.get("root") and str(manifest.get("root")).rstrip("/") + "/" != capsule_path:
            findings.append(
                host.Finding(
                    "critical",
                    "evidence_capsule_manifest_root_mismatch",
                    manifest_rel,
                    f"manifest root must equal {capsule_path!r}.",
                )
            )

        inventory_item = inventory_by_path.get(capsule_path)
        if not inventory_item:
            findings.append(
                host.Finding(
                    "critical",
                    "evidence_capsule_missing_document_inventory",
                    capsule_path,
                    "Evidence capsule root must be declared in document-theme-inventory-v1.json.",
                )
            )
            continue
        if required_theme and inventory_item.get("theme") != required_theme:
            findings.append(
                host.Finding(
                    "critical",
                    "evidence_capsule_inventory_theme_mismatch",
                    capsule_path,
                    f"Evidence capsule inventory theme must be {required_theme!r}.",
                )
            )
        if required_status and inventory_item.get("status") != required_status:
            findings.append(
                host.Finding(
                    "critical",
                    "evidence_capsule_inventory_status_mismatch",
                    capsule_path,
                    f"Evidence capsule inventory status must be {required_status!r}.",
                )
            )
        if inventory_item.get("canonical_target") != manifest_rel:
            findings.append(
                host.Finding(
                    "critical",
                    "evidence_capsule_inventory_target_mismatch",
                    capsule_path,
                    f"Evidence capsule canonical_target must be {manifest_rel!r}.",
                )
            )

    return findings
