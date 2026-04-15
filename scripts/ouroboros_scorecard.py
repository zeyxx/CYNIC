#!/usr/bin/env python3
"""Emit a minimal Ouroboros nightly run record from the corpus fixture.

This is intentionally small. The foundation is:
1. a machine-readable corpus
2. a stable run-record envelope
3. a deterministic nightly rotation policy

Usage:
  python3 scripts/ouroboros_scorecard.py
  python3 scripts/ouroboros_scorecard.py --date 2026-04-15
  python3 scripts/ouroboros_scorecard.py --all
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
CORPUS_PATH = ROOT / "cynic-kernel" / "tests" / "fixtures" / "ouroboros_corpus.json"


def load_corpus() -> dict:
    with CORPUS_PATH.open() as fh:
        return json.load(fh)


def select_repos(corpus: dict, run_date: dt.date, include_all: bool) -> tuple[int, list[dict]]:
    repos = corpus["repos"]
    if include_all:
        return 0, repos

    created = dt.date.fromisoformat(corpus["created"])
    offset = max((run_date - created).days, 0)

    selected = []
    tracks = [track["id"] for track in corpus["tracks"]]
    for track in tracks:
        track_repos = [repo for repo in repos if repo["track"] == track]
        idx = offset % len(track_repos)
        selected.append(track_repos[idx])
    return offset, selected


def blank_repo_eval(repo: dict, task_profiles: list[dict]) -> dict:
    return {
        "repo_id": repo["id"],
        "full_name": repo["full_name"],
        "track": repo["track"],
        "language": repo["language"],
        "role": repo["role"],
        "nightly_question": repo["nightly_question"],
        "tasks": [
            {
                "task_profile": task["id"],
                "status": "pending",
                "decision": None,
                "effort": None,
                "confidence": None,
                "evidence_count": 0,
                "exact_files_cited": 0,
                "notes": "",
            }
            for task in task_profiles
        ],
    }


def build_run_record(corpus: dict, run_date: dt.date, include_all: bool) -> dict:
    offset, selected = select_repos(corpus, run_date, include_all)
    run_id = f"ouroboros-{run_date.isoformat()}"
    return {
        "run_id": run_id,
        "date": run_date.isoformat(),
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "mode": "landscape",
        "trigger": "nightly",
        "status": "planned",
        "corpus_version": corpus["version"],
        "rotation_offset_days": offset,
        "guardrails": {
            "external_repo_writes": False,
            "max_internal_patches": corpus["rotation_policy"]["mutation_budget"]["max_internal_patches"],
            "selected_repo_count": len(selected),
        },
        "selected_repos": [
            {
                "repo_id": repo["id"],
                "full_name": repo["full_name"],
                "track": repo["track"],
                "language": repo["language"],
                "why_it_matters": repo["why_it_matters"],
            }
            for repo in selected
        ],
        "scorecard": {
            "reliability": {
                "bootstrap_ok": None,
                "auth_ok": None,
                "repos_attempted": len(selected),
                "repos_completed": 0,
                "tool_failures": 0,
            },
            "learning": {
                "primary_sources_read": 0,
                "adopt_candidates": 0,
                "reject_candidates": 0,
                "followup_candidates": 0,
                "stale_repos_flagged": 0,
            },
            "actionability": {
                "internal_patch_candidates": 0,
                "internal_patches_authored": 0,
                "tests_run": 0,
                "tests_passed": 0,
            },
            "efficiency": {
                "duration_s": None,
                "prompt_tokens": 0,
                "completion_tokens": 0,
            },
        },
        "repo_evals": [blank_repo_eval(repo, corpus["task_profiles"]) for repo in selected],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--date", help="Run date in YYYY-MM-DD. Defaults to today.")
    parser.add_argument("--all", action="store_true", help="Emit all corpus repos instead of nightly rotation.")
    args = parser.parse_args()

    run_date = dt.date.fromisoformat(args.date) if args.date else dt.date.today()
    corpus = load_corpus()
    record = build_run_record(corpus, run_date, args.all)
    print(json.dumps(record, indent=2, ensure_ascii=True))


if __name__ == "__main__":
    main()
