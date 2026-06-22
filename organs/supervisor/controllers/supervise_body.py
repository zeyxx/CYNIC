#!/usr/bin/env python3
"""Supervise CYNIC body health.

This controller is intentionally deterministic and side-effect-light:
it reads existing audit outputs and writes guard state plus a repair queue.
It does not start services, edit other organs, post externally, or call the
kernel. Its job is to make body instability actionable.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ORGANS_ROOT = Path(__file__).resolve().parents[2]
WORKSPACE_ROOT = ORGANS_ROOT.parent.parent
SUPERVISOR_ROOT = Path(__file__).resolve().parents[1]
DESIRED = SUPERVISOR_ROOT / "desired/body-supervision.json"
CONTRACT_AUDIT = ORGANS_ROOT / "organ-contract-audit.json"
RUNTIME_HEALTH = ORGANS_ROOT / "state/body-health.json"
SUPERVISION_STATE = SUPERVISOR_ROOT / "state/supervision.json"
ACTION_QUEUE = SUPERVISOR_ROOT / "state/action-queue.jsonl"


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {"_error": "invalid_json", "_path": str(path)}


def by_organ(items: list[Any]) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for item in items:
        if isinstance(item, dict) and item.get("organ"):
            result[str(item["organ"])] = item
    return result


def trust_level(contract: dict[str, Any], runtime: dict[str, Any]) -> str:
    contract_status = contract.get("status") or runtime.get("contract_status")
    runtime_status = runtime.get("runtime_status")
    if contract_status in {"uncontracted_directory", None}:
        return "undeclared"
    if contract_status == "manifest_only":
        return "declared"
    if contract_status in {"partial_workflow_organ", "contracted_organ", "contracted_organ_runtime_unverified"}:
        return "wired"
    if contract_status == "production_organ" and runtime_status in {"live", "warm"}:
        return "trusted"
    if contract_status == "production_organ":
        return "wired"
    return "declared"


def action_for(organ: str, contract: dict[str, Any], runtime: dict[str, Any], priority: int) -> dict[str, Any] | None:
    level = trust_level(contract, runtime)
    contract_status = contract.get("status") or runtime.get("contract_status")
    runtime_status = runtime.get("runtime_status")
    gaps = list(contract.get("gaps") or runtime.get("gaps") or [])

    if level == "trusted":
        return None
    if contract_status == "uncontracted_directory":
        action = "contract_or_demote_directory"
        severity = "medium"
    elif contract_status == "manifest_only":
        action = "add_desired_runbook_controller_ssot_runtime"
        severity = "high"
    elif contract_status in {"partial_workflow_organ", "contracted_organ", "contracted_organ_runtime_unverified"}:
        action = "complete_contract_or_runtime_evidence"
        severity = "high"
    elif runtime_status not in {"live", "warm"}:
        action = "repair_runtime_freshness"
        severity = "high"
    else:
        action = "review_untrusted_state"
        severity = "medium"

    return {
        "created_at": now_utc(),
        "organ": organ,
        "priority": priority,
        "severity": severity,
        "action": action,
        "trust_level": level,
        "contract_status": contract_status,
        "runtime_status": runtime_status,
        "gaps": gaps,
        "manual_gate": "human_selects_repair_before_mutation",
    }


def main() -> int:
    generated_at = now_utc()
    desired = load_json(DESIRED)
    contract_audit = load_json(CONTRACT_AUDIT)
    runtime_health = load_json(RUNTIME_HEALTH)

    contract_organs = by_organ(contract_audit.get("organs", []))
    runtime_organs = by_organ(runtime_health.get("organs", []))
    all_organs = sorted(set(contract_organs) | set(runtime_organs))
    priority_order = desired.get("repair_priority") if isinstance(desired.get("repair_priority"), list) else []
    priority_rank = {str(name): index for index, name in enumerate(priority_order)}

    organ_states: list[dict[str, Any]] = []
    actions: list[dict[str, Any]] = []
    for organ in all_organs:
        contract = contract_organs.get(organ, {})
        runtime = runtime_organs.get(organ, {})
        level = trust_level(contract, runtime)
        state = {
            "organ": organ,
            "trust_level": level,
            "contract_status": contract.get("status") or runtime.get("contract_status"),
            "runtime_status": runtime.get("runtime_status"),
            "manifest_status": contract.get("manifest_status") or runtime.get("manifest_status"),
            "priority": priority_rank.get(organ, 1000),
            "next_action": runtime.get("next_action") or contract.get("next_action"),
        }
        organ_states.append(state)
        action = action_for(organ, contract, runtime, state["priority"])
        if action:
            actions.append(action)

    trusted = [item["organ"] for item in organ_states if item["trust_level"] == "trusted"]
    untrusted = [item["organ"] for item in organ_states if item["trust_level"] != "trusted"]
    hard_blocks = []
    if untrusted:
        hard_blocks.append("new_organ_creation_requires_owner_reuse_or_repair_decision")
    if runtime_health.get("summary", {}).get("body_status") not in {"live", "warm"}:
        hard_blocks.append("body_runtime_not_globally_live")

    actions.sort(key=lambda item: (item["priority"], item["severity"] != "high", item["organ"]))
    organ_states.sort(key=lambda item: (item["priority"], item["organ"]))

    report = {
        "schema_version": "1.0.0",
        "generated_by": "CYNIC/organs/supervisor/controllers/supervise_body.py",
        "generated_at": generated_at,
        "purpose": "Convert CYNIC organ audits into guard decisions and repair queue entries.",
        "body": {
            "contract_summary": contract_audit.get("summary", {}),
            "runtime_summary": runtime_health.get("summary", {}),
            "trusted_organs": trusted,
            "untrusted_organs": untrusted,
            "hard_blocks": hard_blocks,
        },
        "guards": {
            "new_organ_creation": "blocked" if hard_blocks else "allowed",
            "public_external_actions": "manual_approval_required",
            "kernel_offline_policy": "degrade_to_durable_queue",
        },
        "organs": organ_states,
        "action_count": len(actions),
        "action_queue": str(ACTION_QUEUE.relative_to(WORKSPACE_ROOT)),
    }

    SUPERVISION_STATE.write_text(json.dumps(report, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    ACTION_QUEUE.write_text(
        "".join(json.dumps(action, sort_keys=False) + "\n" for action in actions),
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
