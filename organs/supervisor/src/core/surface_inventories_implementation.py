#!/usr/bin/env python3
"""Validator for the implementation-surface inventory contract."""

from __future__ import annotations

import subprocess
from typing import Any

from organs.supervisor.src.core.surface_inventories_common import (
    load_inventory_payload,
    validate_surface_status_transition,
)


def validate_implementation_surface_inventory_contract(service: Any) -> list[Any]:
    findings: list[Any] = []
    inventory, load_findings = load_inventory_payload(
        service,
        path=service.implementation_surface_inventory_path,
        expected_schema="cynic.implementation_surface_inventory.v1",
        missing_kind="implementation_surface_inventory_missing",
        invalid_kind="implementation_surface_inventory_invalid_json",
        missing_detail="implementation-surface-inventory-v1.json must classify non-organ implementation surfaces with open Git membranes.",
    )
    findings.extend(load_findings)
    if not inventory:
        return findings

    allowed_statuses = set(inventory.get("allowed_statuses", []))
    required_open_membrane_evidence = str(
        inventory.get("required_evidence_for_open_membrane")
        or "untracked implementation source files"
    )
    surfaces = inventory.get("surfaces")
    if not isinstance(surfaces, list):
        findings.append(
            service.finding(
                "critical",
                "implementation_surface_inventory_surfaces_missing",
                service.relative(service.implementation_surface_inventory_path),
                "implementation-surface-inventory-v1 must declare surfaces as a list.",
            )
        )
        surfaces = []

    seen_paths: set[str] = set()
    by_status: dict[str, int] = {}
    try:
        untracked = set(service.git_ls_untracked())
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            service.finding(
                "critical",
                "implementation_surface_inventory_git_scan_failed",
                ".",
                f"unable to query git untracked implementation files: {exc}",
            )
        ]

    for index, item in enumerate(surfaces):
        item_path = str(item.get("path") or f"surfaces[{index}]")
        for field in sorted(service.implementation_surface_required_fields):
            if not item.get(field):
                findings.append(
                    service.finding(
                        "critical",
                        "implementation_surface_inventory_metadata_missing",
                        item_path,
                        f"surfaces entries must declare {field}.",
                    )
                )
        path_value = item.get("path")
        path = ""
        if path_value:
            path = str(path_value).rstrip("/") + "/"
            if path in seen_paths:
                findings.append(
                    service.finding(
                        "critical",
                        "implementation_surface_inventory_duplicate_path",
                        path,
                        "Each implementation surface must appear only once in implementation-surface-inventory-v1.",
                    )
                )
            seen_paths.add(path)
            if not (service.workspace_root / path).exists():
                findings.append(
                    service.finding(
                        "critical",
                        "implementation_surface_inventory_path_missing",
                        path,
                        "Inventory entry points to an implementation surface that does not exist.",
                    )
                )
        status = item.get("status")
        if status:
            status_value = str(status)
            by_status[status_value] = by_status.get(status_value, 0) + 1
            if allowed_statuses and status_value not in allowed_statuses:
                findings.append(
                    service.finding(
                        "critical",
                        "implementation_surface_inventory_status_unknown",
                        item_path,
                        f"status {status!r} is not declared in allowed_statuses.",
                    )
                )
            findings.extend(
                validate_surface_status_transition(
                    service=service,
                    inventory=inventory,
                    inventory_path=service.implementation_surface_inventory_path,
                    inventory_kind="implementation_surface_inventory",
                    item_path=item_path,
                    status=status,
                )
            )
        evidence = item.get("evidence", [])
        if path and any(untracked_item.startswith(path) for untracked_item in untracked):
            if required_open_membrane_evidence not in evidence:
                findings.append(
                    service.finding(
                        "critical",
                        "implementation_surface_untracked_body_missing_evidence",
                        path,
                        f"Implementation surfaces with visible untracked files must declare {required_open_membrane_evidence!r} in evidence.",
                    )
                )

    totals = inventory.get("totals", {})
    if totals:
        declared_count = totals.get("surfaces")
        if declared_count is not None and declared_count != len(surfaces):
            findings.append(
                service.finding(
                    "critical",
                    "implementation_surface_inventory_total_mismatch",
                    service.relative(service.implementation_surface_inventory_path),
                    f"totals.surfaces={declared_count!r} but inventory contains {len(surfaces)} entries.",
                )
            )
        declared_by_status = totals.get("by_status", {})
        if declared_by_status and {str(key): value for key, value in declared_by_status.items()} != by_status:
            findings.append(
                service.finding(
                    "critical",
                    "implementation_surface_inventory_status_total_mismatch",
                    service.relative(service.implementation_surface_inventory_path),
                    f"totals.by_status does not match actual inventory counts {by_status}.",
                )
            )

    return findings
