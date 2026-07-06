#!/usr/bin/env python3
"""Build durable inference tasks for Percolator dedupe triage.

This script does not require the CYNIC kernel. It converts the structured
Percolator researcher feed into JSONL tasks that can be consumed later by any
available inference worker: remote CYNIC, laptop, server, or manual batch.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import read_json_file_or_default

_DEFAULT_FEED = Path("percolator/ops/researcher-feed.json")
_DEFAULT_QUEUE = Path("percolator/ops/infer-queue.jsonl")
_DEFAULT_STATE = Path("percolator/ops/infer-queue-state.json")


def stable_hash(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    return read_json_file_or_default(path, {})


def load_seen(path: Path) -> set[str]:
    state = load_json(path)
    seen = state.get("seen_task_ids", [])
    return {str(item) for item in seen if item}


def write_state(path: Path, seen: set[str], emitted: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "updated_at": datetime.now(timezone.utc).isoformat(),
        "emitted_last_run": emitted,
        "seen_task_ids": sorted(seen),
    }
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")


def task_from_item(item: dict[str, Any]) -> dict[str, Any] | None:
    classification = item.get("classification")
    if classification not in {
        "github_issue_submission",
        "github_pull_request",
        "external_audit_dossier",
        "bounty_announcement",
        "duplicate_signal",
        "proof_or_submission_guidance",
    }:
        return None

    task_input = {
        "source": item.get("source"),
        "time_utc": item.get("time_utc"),
        "researcher": item.get("researcher"),
        "classification": classification,
        "repo_scope": item.get("repo_scope", []),
        "severity_signal": item.get("severity_signal"),
        "claim_or_finding": item.get("claim_or_finding"),
        "evidence_links": item.get("evidence_links", []),
        "github_refs": item.get("github_refs", []),
    }
    task_id = "percolator-dedupe-" + stable_hash(task_input)[:24]
    return {
        "task_id": task_id,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "kind": "local_or_remote_infer",
        "domain": "percolator_dedupe",
        "priority": "normal",
        "status": "queued",
        "objective": "Summarize duplicate risk and likely affected area for a Percolator bounty claim. Do not invent facts.",
        "input": task_input,
        "expected_output_schema": {
            "likely_area": "string_or_null",
            "duplicate_risk": "none|low|medium|high|duplicate|unknown",
            "summary": "string",
            "needs_human_review": "boolean",
            "reasoning_limits": "string"
        },
    }


def build_queue(feed_path: Path, queue_path: Path, state_path: Path) -> int:
    feed = load_json(feed_path)
    seen = load_seen(state_path)
    items = feed.get("live_log", [])
    emitted = 0
    queue_path.parent.mkdir(parents=True, exist_ok=True)
    with queue_path.open("a", encoding="utf-8") as out:
        for item in items:
            if not isinstance(item, dict):
                continue
            task = task_from_item(item)
            if not task:
                continue
            task_id = task["task_id"]
            if task_id in seen:
                continue
            out.write(json.dumps(task, separators=(",", ":"), ensure_ascii=True) + "\n")
            seen.add(task_id)
            emitted += 1
    write_state(state_path, seen, emitted)
    return emitted


def main() -> int:
    parser = argparse.ArgumentParser(description="Build Percolator dedupe inference queue")
    parser.add_argument("--feed", default=str(_DEFAULT_FEED))
    parser.add_argument("--queue", default=str(_DEFAULT_QUEUE))
    parser.add_argument("--state", default=str(_DEFAULT_STATE))
    args = parser.parse_args()
    emitted = build_queue(Path(args.feed), Path(args.queue), Path(args.state))
    print(f"percolator dedupe queue updated: emitted={emitted}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
