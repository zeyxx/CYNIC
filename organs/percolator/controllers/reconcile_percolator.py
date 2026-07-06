#!/usr/bin/env python3
"""Reconcile Percolator bounty desired state into workspace generated files."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import read_json_file_or_default


def _resolve_project_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()

    start = Path(__file__).resolve()
    for candidate in [start, *start.parents]:
        if (candidate / "infra").is_dir() and (candidate / "organs").is_dir() and (candidate / ".git").exists():
            return candidate

    # Conservative fallback: one-level parent of the current module directory.
    fallback = Path(__file__).resolve()
    return fallback.parent


ROOT = _resolve_project_root()
WORKSPACE = ROOT.parent
DEFAULT_DESIRED = ROOT / "organs/percolator/desired/percolator-bounty.json"


def load_json(path: Path) -> dict[str, Any]:
    return read_json_file_or_default(path, {})


def write_json(path: Path, payload: dict[str, Any], dry_run: bool) -> bool:
    content = json.dumps(payload, indent=2, ensure_ascii=True) + "\n"
    path.parent.mkdir(parents=True, exist_ok=True)
    current = path.read_text(encoding="utf-8") if path.exists() else None
    if current == content:
        return False
    if not dry_run:
        path.write_text(content, encoding="utf-8")
    return True


def path_from_workspace(value: str) -> Path:
    return WORKSPACE / value


def latest_bounty_window(feed: dict[str, Any]) -> dict[str, Any] | None:
    windows = feed.get("active_bounty_windows", [])
    if not isinstance(windows, list) or not windows:
        return None
    valid = [item for item in windows if isinstance(item, dict) and item.get("start_utc")]
    if not valid:
        return None
    return sorted(valid, key=lambda item: str(item.get("start_utc")))[-1]


def github_summary(feed: dict[str, Any]) -> dict[str, Any]:
    refs = feed.get("github_refs", [])
    out: dict[str, Any] = {"issues": [], "pull_requests": [], "repositories": []}
    if not isinstance(refs, list):
        return out
    for ref in refs:
        if not isinstance(ref, dict):
            continue
        kind = ref.get("kind")
        if kind == "issues":
            out["issues"].append(ref)
        elif kind == "pull":
            out["pull_requests"].append(ref)
        elif kind == "repository":
            out["repositories"].append(ref)
    return out


def workspace_index(desired: dict[str, Any]) -> dict[str, Any]:
    ws = desired["workspace"]
    return {
        "schema_version": "1.0.0",
        "name": "percolator",
        "source_of_truth": desired["source_of_truth"],
        "purpose": "Generated Percolator workspace view. Edit CYNIC desired state, not this file.",
        "layout": {
            ws["upstream_source_of_truth"]: {"role": "canonical_upstream_clone", "use_for": ["code_review", "patching", "audit_reproduction"]},
            ws["audit_workspace"]: {"role": "bounty_audit_workspace"},
            ws["ops_workspace"]: {"role": "live_bounty_operations"},
            ws["duplicate_holding_area"]: {"role": "temporary_duplicate_holding_area", "source_of_truth": False}
        }
    }


def ssot_index(desired: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "1.0.0",
        "domain": "percolator",
        "source_of_truth": desired["source_of_truth"],
        "runbook": "CYNIC/organs/percolator/runbooks/percolator-bounty-ops.json",
        "controller": "CYNIC/organs/percolator/controllers/reconcile_percolator.py",
        "generated_outputs": desired["outputs"],
        "runtime_inputs": desired["inputs"],
        "rule": "Change desired state first, then run the controller. Discord and GitHub are observation inputs, not the SSOT."
    }


def bounty_rules(desired: dict[str, Any], feed: dict[str, Any]) -> dict[str, Any]:
    rules = desired["bounty_rules"]
    window = latest_bounty_window(feed)
    payload = {
        "schema_version": "1.0.0",
        "generated_from": desired["source_of_truth"],
        "scope": desired["scope"],
        "pinned_message": {
            "status": "desired_state_imported",
            "content": rules["pinned_message"],
            "source": "CYNIC desired state"
        },
        "submission_rules": {
            "preferred_submission": rules["preferred_submission"],
            "pull_request_allowed_as_separate_fix": rules["pull_request_allowed_as_separate_fix"],
            "proof_required": rules["proof_required"],
            "poc_tests_preferred": rules["poc_tests_preferred"],
            "avoid_speculation": rules["avoid_speculation"],
            "after_issue_filed": rules["after_issue_filed"],
            "payment_timing": rules["payment_timing"]
        },
        "active_priority": {
            "repository": desired["scope"]["primary_repository"],
            "severity": rules["general_severity"],
            "reason": "general_pinned_rule"
        },
        "qualification_checklist": [
            "scope_confirmed",
            "severity_matches_active_priority",
            "reproducible_against_current_upstream",
            "concrete_impact_not_theoretical",
            "proof_or_poc_included",
            "duplicate_check_completed",
            "wording_checked_for_accuracy",
            "no_speculation"
        ]
    }
    if window:
        payload["timeboxed_announcement"] = {
            "status": "observed_from_discord_feed",
            **window,
            "qualification_dependency": "pinned_message"
        }
        payload["active_priority"] = {
            "repository": window.get("repository") or desired["scope"]["primary_repository"],
            "severity": window.get("severity") or rules["timeboxed_override_severity"],
            "deadline_utc": window.get("end_utc"),
            "deadline_paris": window.get("end_paris"),
            "reason": "timeboxed_discord_announcement_observed"
        }
    return payload


def audit_plan(desired: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "1.0.0",
        "generated_from": desired["source_of_truth"],
        "primary_risk_areas": desired["risk_areas"],
        "workflow": [
            "refresh_discord_researcher_feed",
            "reconcile_percolator_desired_state",
            "snapshot_upstream_commit_per_candidate",
            "trace_one_instruction_path_at_a_time",
            "record_candidates_in_finding_board",
            "promote_only_reproducible_in_scope_findings"
        ]
    }


def anti_duplicate(desired: dict[str, Any], feed: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "1.0.0",
        "generated_from": desired["source_of_truth"],
        **desired["duplicate_policy"],
        "observed_github_summary": github_summary(feed)
    }


def finding_board(desired: dict[str, Any], existing: dict[str, Any] | None) -> dict[str, Any]:
    if existing and isinstance(existing.get("findings"), list):
        findings = existing["findings"]
    else:
        findings = []
    return {
        "schema_version": "1.0.0",
        "generated_from": desired["source_of_truth"],
        "status_values": {
            "candidate": "suspicious_behavior_not_proven",
            "repro": "reproducible_locally_or_by_transaction_sketch",
            "impact": "concrete_loss_privilege_or_state_impact_established",
            "dedupe_check": "compared_against_researcher_feed_and_known_docs",
            "ready": "report_has_enough_evidence_to_submit",
            "submitted": "sent_to_maintainers_or_bounty_channel",
            "rejected": "false_positive_duplicate_out_of_scope_or_too_low_severity"
        },
        "findings": findings
    }


def verify(desired: dict[str, Any]) -> dict[str, Any]:
    outputs = desired["outputs"]
    checks = {name: path_from_workspace(path).exists() for name, path in outputs.items()}
    checks["all_passed"] = all(checks.values())
    return checks


def main() -> int:
    parser = argparse.ArgumentParser(description="Reconcile Percolator bounty desired state")
    parser.add_argument("--desired", default=str(DEFAULT_DESIRED))
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()

    desired = load_json(Path(args.desired))
    feed = load_json(path_from_workspace(desired["inputs"]["discord_researcher_feed"]))
    outputs = desired["outputs"]
    existing_board = load_json(path_from_workspace(outputs["finding_board"]))
    writes = {
        outputs["workspace_index"]: workspace_index(desired),
        outputs["ssot_index"]: ssot_index(desired),
        outputs["bounty_rules"]: bounty_rules(desired, feed),
        outputs["audit_plan"]: audit_plan(desired),
        outputs["anti_duplicate_protocol"]: anti_duplicate(desired, feed),
        outputs["finding_board"]: finding_board(desired, existing_board),
    }
    changed = {path: write_json(path_from_workspace(path), payload, args.dry_run) for path, payload in writes.items()}
    result = {"changed": changed, "verify": verify(desired) if args.verify else None}
    print(json.dumps(result, indent=2, ensure_ascii=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
