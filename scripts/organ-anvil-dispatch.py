#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-anvil dispatch script.
"""Dispatch one organ-anvil repo lifecycle task to the Hermes Agent executor."""

from __future__ import annotations

import json
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

# Ensure the scripts directory is in sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from hermes_dispatch import HermesDispatcher

PROJECT_DIR = Path(__file__).resolve().parent.parent
PROPOSAL_STALE_AFTER = timedelta(minutes=45)


def repo_health() -> dict | None:
    try:
        result = subprocess.run(
            ["bash", "scripts/organ-anvil.sh", "repo-health"],
            cwd=PROJECT_DIR,
            check=True,
            capture_output=True,
            text=True,
            timeout=30,
        )
        return json.loads(result.stdout)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as exc:
        print(f"organ-anvil proposal generation skipped: {exc}", file=sys.stderr)
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


def existing_proposals(dispatcher: HermesDispatcher) -> int:
    tasks = dispatcher.fetch_tasks("/agent-tasks?kind=organ-anvil-proposal&domain=repo-lifecycle&limit=50")
    return sum(1 for task in tasks if _is_fresh_proposal(task))


def approved_proposals(dispatcher: HermesDispatcher) -> list[dict]:
    tasks = dispatcher.fetch_tasks("/agent-tasks/completed?kind=organ-anvil-proposal&domain=repo-lifecycle&limit=50")
    approved = []
    for task in tasks:
        payload = dispatcher.task_result_payload(task)
        if payload.get("decision") == "approved":
            approved.append(task)
    return approved


def remediation_followup_exists(dispatcher: HermesDispatcher, proposal_id: str) -> bool:
    pending = dispatcher.fetch_tasks("/agent-tasks?kind=hermes&domain=organ-anvil&limit=50")
    completed = dispatcher.fetch_tasks("/agent-tasks/completed?kind=hermes&domain=organ-anvil&limit=50")
    for task in pending + completed:
        content = task.get("content") or ""
        try:
            payload = json.loads(content)
        except json.JSONDecodeError:
            continue
        if payload.get("proposal_task_id") == proposal_id:
            return True
    return False


def build_remediation_task(dispatcher: HermesDispatcher, proposal_task: dict) -> dict | None:
    proposal = {}
    try:
        proposal = json.loads(proposal_task.get("content") or "{}")
    except json.JSONDecodeError:
        proposal = {}

    review = dispatcher.task_result_payload(proposal_task)
    if not proposal:
        return None

    dirty_count = proposal.get("dirty_count", 0)
    stash_count = proposal.get("stash_count", 0)
    current_branch = proposal.get("current_branch")
    recommendations = proposal.get("recommendations") or []
    summary_bits = []
    if dirty_count:
        summary_bits.append(f"{dirty_count} dirty paths")
    if stash_count:
        summary_bits.append(f"{stash_count} stashes")
    if current_branch:
        summary_bits.append(f"branch={current_branch}")

    return {
        "kind": "hermes",
        "domain": "organ-anvil",
        "agent_id": "organ-anvil-cron",
        "content": json.dumps(
            {
                "objective": "Organ Anvil approved remediation",
                "proposal_task_id": proposal_task.get("id"),
                "review_mode": "approval_required",
                "reviewed_at": review.get("reviewed_at") or proposal_task.get("completed_at"),
                "review_note": review.get("note"),
                "source": proposal.get("source") or "organ-anvil-proposal",
                "current_branch": current_branch,
                "dirty_count": dirty_count,
                "stash_count": stash_count,
                "open_pr_count": proposal.get("open_pr_count", 0),
                "recommendations": recommendations,
                "summary": ", ".join(summary_bits) if summary_bits else "approved repo lifecycle remediation",
                "targets": [
                    "infra/organ-anvil/state.json",
                    "infra/organ-anvil/poh.json",
                    "infra/organ-anvil/audit.jsonl",
                    ".handoff.md",
                ],
                "actions": [
                    "Read the approved proposal and choose the smallest reversible remediation.",
                    "Apply only the approved lifecycle action(s) for the current scope.",
                    "Record the remediation result in the audit and handoff trail.",
                    "Stop immediately if the action would widen scope beyond the approved proposal.",
                ],
            },
            separators=(",", ":"),
        ),
    }


def dispatch_proposal_task(dispatcher: HermesDispatcher) -> bool:
    pending = dispatcher.fetch_tasks("/agent-tasks?kind=organ-anvil-proposal&domain=repo-lifecycle&limit=50")
    fresh_pending = [task for task in pending if _is_fresh_proposal(task)]
    stale_pending = [task for task in pending if task not in fresh_pending]
    if fresh_pending:
        print("organ-anvil proposal dispatch skipped: fresh proposal already pending", file=sys.stderr)
        return False
    if stale_pending:
        print(
            f"organ-anvil proposal dispatch refreshing {len(stale_pending)} stale proposal(s)",
            file=sys.stderr,
        )

    health = repo_health()
    if not health:
        return False

    recommendations = [
        rec for rec in (health.get("recommendations") or [])
        if rec.get("severity") in {"warning", "critical"}
    ]
    if not recommendations:
        return False

    worktree = health.get("worktree") or {}
    stashes = health.get("stashes") or {}
    current = health.get("current") or {}
    branches = health.get("branches") or {}
    summary_bits = []
    dirty_count = worktree.get("dirty_count", 0)
    stash_count = stashes.get("count", 0)
    if dirty_count:
        summary_bits.append(f"{dirty_count} dirty paths")
    if stash_count:
        summary_bits.append(f"{stash_count} stashes")
    if branches.get("stale_upstreams"):
        summary_bits.append(f"{len(branches['stale_upstreams'])} stale upstreams")
    summary = ", ".join(summary_bits) if summary_bits else "repo hygiene needs review"

    proposal = {
        "title": "Organ Anvil fix proposals",
        "summary": summary,
        "source": "organ-anvil",
        "review_mode": "approval_required",
        "current_branch": current.get("branch"),
        "dirty_count": dirty_count,
        "stash_count": stash_count,
        "open_pr_count": (health.get("prs") or {}).get("open_count", 0),
        "recommendations": recommendations[:5],
    }
    payload = {
        "kind": "organ-anvil-proposal",
        "domain": "repo-lifecycle",
        "agent_id": "organ-anvil-cron",
        "content": json.dumps(proposal, separators=(",", ":")),
    }
    return dispatcher.dispatch_task(payload)


def dispatch_approved_remediations(dispatcher: HermesDispatcher) -> bool:
    dispatched = False
    for proposal_task in approved_proposals(dispatcher):
        proposal_id = proposal_task.get("id") or ""
        if not proposal_id or remediation_followup_exists(dispatcher, proposal_id):
            continue
        payload = build_remediation_task(dispatcher, proposal_task)
        if not payload:
            continue
        if dispatcher.dispatch_task(payload):
            dispatched = True
    return dispatched


def main() -> int:
    dispatcher = HermesDispatcher("organ-anvil")
    key = dispatcher.kernel_key()
    if not key:
        print("organ-anvil dispatch skipped: kernel API key missing", file=sys.stderr)
        return 1

    content = {
        "objective": "Organ Anvil repo lifecycle perception",
        "domain": "organ-anvil",
        "targets": [
            "infra/organ-anvil/state.json",
            "infra/organ-anvil/poh.json",
            "infra/organ-anvil/audit.jsonl",
        ],
        "actions": [
            "Run bash scripts/organ-anvil.sh state to refresh repo perception and persist the observation snapshot.",
            "Do not triage, commit, stash, restore, push, or edit files outside declared targets unless a later task declares them.",
            "Post a concise repo-health observation to /observe.",
        ],
    }
    payload = {
        "kind": "hermes",
        "domain": "organ-anvil",
        "agent_id": "organ-anvil-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }
    if not dispatcher.dispatch_task(payload):
        return 1

    dispatch_proposal_task(dispatcher)
    dispatch_approved_remediations(dispatcher)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
