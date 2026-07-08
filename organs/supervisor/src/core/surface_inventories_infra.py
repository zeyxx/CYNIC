#!/usr/bin/env python3
"""Validator for the infra-surface inventory contract."""

from __future__ import annotations

import subprocess
from typing import Any

from organs.supervisor.src.core.surface_inventories_common import (
    load_inventory_payload,
    validate_surface_status_transition,
)


def validate_infra_surface_inventory_contract(service: Any) -> list[Any]:
    findings: list[Any] = []
    inventory, load_findings = load_inventory_payload(
        service,
        path=service.infra_surface_inventory_path,
        expected_schema="cynic.infra_surface_inventory.v1",
        missing_kind="infra_surface_inventory_missing",
        invalid_kind="infra_surface_inventory_invalid_json",
        missing_detail="infra-surface-inventory-v1.json must classify immediate infra/* source-of-truth and projection files.",
    )
    findings.extend(load_findings)
    if not inventory:
        return findings

    allowed_statuses = set(inventory.get("allowed_statuses", []))
    required_open_membrane_evidence = str(
        inventory.get("required_evidence_for_open_membrane") or "untracked infra surface file"
    )
    surfaces = inventory.get("surfaces")
    if not isinstance(surfaces, list):
        findings.append(
            service.finding(
                "critical",
                "infra_surface_inventory_surfaces_missing",
                service.relative(service.infra_surface_inventory_path),
                "infra-surface-inventory-v1 must declare surfaces as a list.",
            )
        )
        surfaces = []

    try:
        visible = sorted(set(service.git_ls_files("infra")) | set(service.git_ls_untracked("infra")))
        untracked = set(service.git_ls_untracked("infra"))
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            service.finding(
                "critical",
                "infra_surface_inventory_git_scan_failed",
                "infra/",
                f"unable to query git visible infra files: {exc}",
            )
        ]

    visible_root_files = {path for path in visible if path.startswith("infra/") and path.count("/") == 1}
    seen_paths: set[str] = set()
    inventory_paths: set[str] = set()
    by_status: dict[str, int] = {}
    for index, item in enumerate(surfaces):
        item_path = str(item.get("path") or f"surfaces[{index}]")
        for field in sorted(service.infra_surface_required_fields):
            if not item.get(field):
                findings.append(
                    service.finding(
                        "critical",
                        "infra_surface_inventory_metadata_missing",
                        item_path,
                        f"surfaces entries must declare {field}.",
                    )
                )
        path_value = item.get("path")
        if path_value:
            path = str(path_value)
            inventory_paths.add(path)
            if path in seen_paths:
                findings.append(
                    service.finding(
                        "critical",
                        "infra_surface_inventory_duplicate_path",
                        path,
                        "Each immediate infra file must appear only once in infra-surface-inventory-v1.",
                    )
                )
            seen_paths.add(path)
            if not path.startswith("infra/") or path.count("/") != 1:
                findings.append(
                    service.finding(
                        "critical",
                        "infra_surface_inventory_path_not_immediate_file",
                        path,
                        "Infra surface inventory entries must target immediate infra/* files.",
                    )
                )
            if not (service.workspace_root / path).is_file():
                findings.append(
                    service.finding(
                        "critical",
                        "infra_surface_inventory_path_missing",
                        path,
                        "Inventory entry points to an infra file that does not exist.",
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
                        "infra_surface_inventory_status_unknown",
                        item_path,
                        f"status {status!r} is not declared in allowed_statuses.",
                    )
                )
            findings.extend(
                validate_surface_status_transition(
                    service=service,
                    inventory=inventory,
                    inventory_path=service.infra_surface_inventory_path,
                    inventory_kind="infra_surface_inventory",
                    item_path=item_path,
                    status=status,
                )
            )
        evidence = item.get("evidence", [])
        if path_value and str(path_value) in untracked and required_open_membrane_evidence not in evidence:
            findings.append(
                service.finding(
                    "critical",
                    "infra_surface_untracked_file_missing_evidence",
                    str(path_value),
                    f"Untracked infra surface files must declare {required_open_membrane_evidence!r} in evidence.",
                )
            )

    for path in sorted(visible_root_files - inventory_paths):
        findings.append(
            service.finding(
                "critical",
                "infra_surface_missing_inventory",
                path,
                "Immediate infra/* files must be classified in infra-surface-inventory-v1.json.",
            )
        )
    for path in sorted(inventory_paths - visible_root_files):
        findings.append(
            service.finding(
                "critical",
                "infra_surface_inventory_stale_path",
                path,
                "Infra surface inventory contains a path that is not visible to Git.",
            )
        )

    totals = inventory.get("totals", {})
    if totals:
        declared_count = totals.get("surfaces")
        if declared_count is not None and declared_count != len(surfaces):
            findings.append(
                service.finding(
                    "critical",
                    "infra_surface_inventory_total_mismatch",
                    service.relative(service.infra_surface_inventory_path),
                    f"totals.surfaces={declared_count!r} but inventory contains {len(surfaces)} entries.",
                )
            )
        declared_by_status = totals.get("by_status", {})
        if declared_by_status and {str(key): value for key, value in declared_by_status.items()} != by_status:
            findings.append(
                service.finding(
                    "critical",
                    "infra_surface_inventory_status_total_mismatch",
                    service.relative(service.infra_surface_inventory_path),
                    f"totals.by_status does not match actual inventory counts {by_status}.",
                )
            )

    return findings
