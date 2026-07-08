#!/usr/bin/env python3
"""Validator for the diagnostic surface inventory contract."""

from __future__ import annotations

import fnmatch
import subprocess
from typing import Any

from organs.supervisor.src.core.surface_inventories_common import (
    load_inventory_payload,
    validate_surface_status_transition,
)


def validate_diagnostic_surface_inventory_contract(service: Any) -> list[Any]:
    findings: list[Any] = []
    inventory, load_findings = load_inventory_payload(
        service,
        path=service.diagnostic_surface_inventory_path,
        expected_schema="cynic.diagnostic_surface_inventory.v1",
        missing_kind="diagnostic_surface_inventory_missing",
        invalid_kind="diagnostic_surface_inventory_invalid_json",
        missing_detail="diagnostic-surface-inventory-v1.json must classify root-level local diagnostics and dev projections.",
    )
    findings.extend(load_findings)
    if not inventory:
        return findings

    watched_globs = [str(item) for item in inventory.get("watched_root_globs", [])]
    if not watched_globs:
        findings.append(
            service.finding(
                "critical",
                "diagnostic_surface_inventory_watched_globs_missing",
                service.relative(service.diagnostic_surface_inventory_path),
                "diagnostic-surface-inventory-v1 must declare watched_root_globs.",
            )
        )
    allowed_statuses = set(inventory.get("allowed_statuses", []))
    surfaces = inventory.get("surfaces")
    if not isinstance(surfaces, list):
        findings.append(
            service.finding(
                "critical",
                "diagnostic_surface_inventory_surfaces_missing",
                service.relative(service.diagnostic_surface_inventory_path),
                "diagnostic-surface-inventory-v1 must declare surfaces as a list.",
            )
        )
        surfaces = []

    seen_paths: set[str] = set()
    inventory_paths: set[str] = set()
    by_status: dict[str, int] = {}
    for index, item in enumerate(surfaces):
        item_path = str(item.get("path") or f"surfaces[{index}]")
        for field in sorted(service.diagnostic_surface_required_fields):
            if not item.get(field):
                findings.append(
                    service.finding(
                        "critical",
                        "diagnostic_surface_inventory_metadata_missing",
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
                        "diagnostic_surface_inventory_duplicate_path",
                        path,
                        "Each diagnostic surface must appear only once in diagnostic-surface-inventory-v1.",
                    )
                )
            seen_paths.add(path)
            if "/" in path.strip("/"):
                findings.append(
                    service.finding(
                        "critical",
                        "diagnostic_surface_inventory_path_not_root_file",
                        path,
                        "Diagnostic surface inventory entries must target root-level files only.",
                    )
                )
            if not (service.workspace_root / path).is_file():
                findings.append(
                    service.finding(
                        "critical",
                        "diagnostic_surface_inventory_path_missing",
                        path,
                        "Inventory entry points to a root-level diagnostic file that does not exist.",
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
                        "diagnostic_surface_inventory_status_unknown",
                        item_path,
                        f"status {status!r} is not declared in allowed_statuses.",
                    )
                )
            findings.extend(
                validate_surface_status_transition(
                    service=service,
                    inventory=inventory,
                    inventory_path=service.diagnostic_surface_inventory_path,
                    inventory_kind="diagnostic_surface_inventory",
                    item_path=item_path,
                    status=status,
                )
            )

    try:
        visible_root_files = {
            path
            for path in sorted(set(service.git_ls_files()) | set(service.git_ls_untracked()))
            if "/" not in path and any(fnmatch.fnmatchcase(path, pattern) for pattern in watched_globs)
        }
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            service.finding(
                "critical",
                "diagnostic_surface_inventory_git_scan_failed",
                ".",
                f"unable to query git visible diagnostic surfaces: {exc}",
            )
        ]

    for path in sorted(visible_root_files - inventory_paths):
        findings.append(
            service.finding(
                "critical",
                "diagnostic_surface_missing_inventory",
                path,
                "Root-level diagnostic/dev projection files matching watched_root_globs must be classified in diagnostic-surface-inventory-v1.json.",
            )
        )
    for path in sorted(inventory_paths - visible_root_files):
        findings.append(
            service.finding(
                "medium",
                "diagnostic_surface_inventory_stale_path",
                path,
                "Diagnostic surface inventory entry is not visible to Git; remove it or restore the bounded diagnostic.",
            )
        )

    totals = inventory.get("totals", {})
    if totals:
        declared_count = totals.get("surfaces")
        if declared_count is not None and declared_count != len(surfaces):
            findings.append(
                service.finding(
                    "critical",
                    "diagnostic_surface_inventory_total_mismatch",
                    service.relative(service.diagnostic_surface_inventory_path),
                    f"totals.surfaces={declared_count!r} but inventory contains {len(surfaces)} entries.",
                )
            )
        declared_by_status = totals.get("by_status", {})
        if declared_by_status and {str(key): value for key, value in declared_by_status.items()} != by_status:
            findings.append(
                service.finding(
                    "critical",
                    "diagnostic_surface_inventory_status_total_mismatch",
                    service.relative(service.diagnostic_surface_inventory_path),
                    f"totals.by_status does not match actual inventory counts {by_status}.",
                )
            )

    return findings
