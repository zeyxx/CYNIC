#!/usr/bin/env python3
"""Prompt enricher invocation membrane."""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path


def _bootstrap_repo_path() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return start.parent


ROOT = _bootstrap_repo_path()
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
ORGAN_SRC = ROOT / "organs" / "prompt-enricher" / "src"
if str(ORGAN_SRC) not in sys.path:
    sys.path.insert(0, str(ORGAN_SRC))

from adapters.kernel_tasks import KernelTaskAdapter
from core.enrichment import enrich_prompt


def run_once(kernel_url: str) -> dict[str, object]:
    adapter = KernelTaskAdapter(kernel_url)
    tasks = adapter.fetch_raw_tasks()
    for task in tasks:
        adapter.mark_enriched(task, enrich_prompt(task.prompt))
    return {"schema": "cynic.prompt_enricher.run.v1", "processed": len(tasks), "mutates": False}


def run_loop(kernel_url: str, interval_seconds: float) -> int:
    while True:
        print(json.dumps(run_once(kernel_url), sort_keys=True))
        time.sleep(interval_seconds)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="organ-prompt-enricher control surface")
    parser.add_argument("--kernel-url", default=os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030"))
    parser.add_argument("--loop", action="store_true")
    parser.add_argument("--interval", type=float, default=5.0)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.loop:
        return run_loop(args.kernel_url, args.interval)
    print(json.dumps(run_once(args.kernel_url), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
