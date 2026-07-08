#!/usr/bin/env python3
"""Common helpers for surface inventory validation."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def load_inventory_payload(
    service: Any,
    *,
    path: Path,
    expected_schema: str,
    missing_kind: str,
    invalid_kind: str,
    missing_detail: str,
) -> tuple[dict[str, Any], list[Any]]:
    if not path.exists():
        return (
            {},
            [
                service.finding(
                    "critical",
                    missing_kind,
                    service.relative(path),
                    missing_detail,
                )
            ],
        )

    findings: list[Any] = []
    try:
        payload = service.load_json(path)
    except json.JSONDecodeError as exc:
        return (
            {},
            [
                service.finding(
                    "critical",
                    invalid_kind,
                    service.relative(path),
                    f"{exc.msg} at line {exc.lineno} column {exc.colno}",
                )
            ],
        )

    if payload.get("schema") != expected_schema:
        findings.append(
            service.finding(
                "critical",
                f"unexpected_{path.stem.replace('-', '_')}_schema",
                service.relative(path),
                f"expected {expected_schema!r}, got {payload.get('schema')!r}",
            )
        )
    return payload, findings


def validate_surface_status_transition(
    service: Any,
    *,
    inventory: dict[str, Any],
    inventory_path: Path,
    inventory_kind: str,
    item_path: str,
    status: Any,
) -> list[Any]:
    if not status:
        return []

    status_value = str(status)
    status_transition_policy = inventory.get("status_transition_policy") or {}
    if not isinstance(status_transition_policy, dict):
        return [
            service.finding(
                "critical",
                f"{inventory_kind}_status_transition_policy_invalid",
                service.relative(inventory_path),
                "status_transition_policy must be an object keyed by surface status.",
            )
        ]

    transition = status_transition_policy.get(status_value)
    if not isinstance(transition, dict):
        return [
            service.finding(
                "critical",
                f"{inventory_kind}_status_transition_policy_missing",
                item_path,
                f"status {status!r} must have a status_transition_policy entry.",
            )
        ]

    findings: list[Any] = []
    for field in sorted(service.surface_status_transition_required_fields):
        if not transition.get(field):
            findings.append(
                service.finding(
                    "critical",
                    f"{inventory_kind}_status_transition_policy_metadata_missing",
                    item_path,
                    f"status_transition_policy[{status_value!r}] must declare {field}.",
                )
            )
    return findings
