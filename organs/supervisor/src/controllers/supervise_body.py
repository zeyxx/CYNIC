#!/usr/bin/env python3
"""Supervise CYNIC organ coherence."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root

PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

import organs.supervisor.src.core.body_supervision as _body


SUPERVISOR_ROOT = PROJECT_ROOT / "organs" / "supervisor"
ORGANS_ROOT = PROJECT_ROOT / "organs"
WORKSPACE_ROOT = PROJECT_ROOT
REGISTRY = WORKSPACE_ROOT / "infra/registry.json"
DESIRED = SUPERVISOR_ROOT / "desired/body-supervision.json"
SUPERVISION_STATE = SUPERVISOR_ROOT / "state/supervision.json"
SSOT_STATE = SUPERVISOR_ROOT / "state/SSOT.json"
ACTION_QUEUE = SUPERVISOR_ROOT / "state/action-queue.jsonl"
INFRA_SYSTEMD = WORKSPACE_ROOT / "infra/systemd"

OrganRecord = _body.OrganRecord
Incoherence = _body.Incoherence
RepairSuggestion = _body.RepairSuggestion
FRESH_SECONDS = _body.FRESH_SECONDS
ssot_freshness = _body.ssot_freshness


def build_inventory() -> tuple[list[OrganRecord], list[Incoherence], list[RepairSuggestion]]:
    return _body.build_inventory(
        workspace_root=WORKSPACE_ROOT,
        organs_root=ORGANS_ROOT,
        registry_path=REGISTRY,
        infra_systemd=INFRA_SYSTEMD,
    )


def build_report() -> dict[str, object]:
    return _body.build_report(
        workspace_root=WORKSPACE_ROOT,
        desired_path=DESIRED,
        registry_path=REGISTRY,
        organs_root=ORGANS_ROOT,
        infra_systemd=INFRA_SYSTEMD,
    )


def write_report(report: dict[str, object]) -> None:
    _body.write_report(
        report=report,
        workspace_root=WORKSPACE_ROOT,
        supervisor_root=SUPERVISOR_ROOT,
        desired_path=DESIRED,
        supervision_state=SUPERVISION_STATE,
        ssot_state=SSOT_STATE,
        action_queue=ACTION_QUEUE,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Build CYNIC organ body inventory")
    parser.add_argument("--json", action="store_true", help="print report JSON to stdout")
    parser.add_argument("--write", action="store_true", help="write supervisor state outputs")
    args = parser.parse_args()

    report = build_report()
    if args.write:
        write_report(report)
    if args.json or not args.write:
        print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
