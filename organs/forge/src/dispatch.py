#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-forge dispatch script.
"""Dispatch one organ-forge repo lifecycle task to the Hermes Agent executor."""

from __future__ import annotations

import json
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

# Ensure the scripts directory is in sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent.parent / "scripts"))
from hermes_dispatch import HermesDispatcher

PROJECT_DIR = Path(__file__).resolve().parent.parent
STATE_FILE = PROJECT_DIR / "state" / "state.json"
PROPOSAL_STALE_AFTER = timedelta(minutes=45)


def repo_health() -> dict | None:
    if not STATE_FILE.exists():
        return None
    try:
        with open(STATE_FILE, "r") as f:
            return json.load(f)
    except json.JSONDecodeError:
        return None


def _task_created_at(task: dict) -> datetime | None:
    created_at = task.get("created_at")
    if not created_at:
        return None
    try:
        return datetime.fromisoformat(created_at.replace("Z", "+00:00"))
    except ValueError:
        return None


def _is_fresh_proposal(task: dict) -> bool:
    created_at = _task_created_at(task)
    if created_at is None:
        return False
    return datetime.now(timezone.utc) - created_at <= PROPOSAL_STALE_AFTER


def dispatch_proposal_task(dispatcher: HermesDispatcher) -> bool:
    pending = dispatcher.fetch_tasks("/agent-tasks?kind=organ-forge-proposal&domain=repo-lifecycle&limit=50")
    fresh_pending = [task for task in pending if _is_fresh_proposal(task)]
    if fresh_pending:
        print("organ-forge proposal dispatch skipped: fresh proposal already pending", file=sys.stderr)
        return False

    health = repo_health()
    if not health:
        return False

    alerts = health.get("alerts", [])
    recommendations = [alert for alert in alerts if alert.get("severity") in {"WARN", "CRITICAL"}]
    
    if not recommendations:
        return False

    summary_bits = []
    dirty_count = health.get("dirty_files", 0)
    if dirty_count:
        summary_bits.append(f"{dirty_count} dirty paths")
    
    untracked_count = health.get("untracked_files", 0)
    if untracked_count:
        summary_bits.append(f"{untracked_count} untracked files")

    summary = ", ".join(summary_bits) if summary_bits else "repo hygiene needs review"

    proposal = {
        "title": "Organ Forge fix proposals",
        "summary": summary,
        "source": "organ-forge",
        "review_mode": "approval_required",
        "current_branch": health.get("current_branch"),
        "dirty_count": dirty_count,
        "recommendations": recommendations[:5],
    }
    payload = {
        "kind": "organ-forge-proposal",
        "domain": "repo-lifecycle",
        "agent_id": "organ-forge-cron",
        "content": json.dumps(proposal, separators=(",", ":")),
    }
    return dispatcher.dispatch_task(payload)


def main() -> int:
    dispatcher = HermesDispatcher("organ-forge")
    key = dispatcher.kernel_key()
    if not key:
        print("organ-forge dispatch skipped: kernel API key missing", file=sys.stderr)
        return 1

    content = {
        "objective": "Organ Forge repo lifecycle perception",
        "domain": "organ-forge",
        "targets": [
            "organs/forge/state/state.json"
        ],
        "actions": [
            "Review the current repo state from state.json.",
            "Triage any CRITICAL or WARN alerts.",
            "Post a concise repo-health observation to /observe.",
        ],
    }
    payload = {
        "kind": "hermes",
        "domain": "organ-forge",
        "agent_id": "organ-forge-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }
    if not dispatcher.dispatch_task(payload):
        return 1

    dispatch_proposal_task(dispatcher)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
