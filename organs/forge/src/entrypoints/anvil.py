#!/usr/bin/env python3
"""Organ-forge Anvil entrypoint."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def _bootstrap_repo_path() -> Path:
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            if str(candidate) not in sys.path:
                sys.path.insert(0, str(candidate))
            return candidate
    raise RuntimeError("Could not resolve repository root for organ-forge anvil")


ROOT = _bootstrap_repo_path()

from organs.forge.src.core.anvil import (  # noqa: E402
    BranchCounts,
    DirtyCounts,
    RepoAnvilService,
    RepoAnvilSnapshot,
)


STATE_DIR = ROOT / "organs" / "forge" / "state"
STATE_FILE = STATE_DIR / "anvil-state.json"
AUDIT_FILE = STATE_DIR / "anvil-audit.jsonl"


def run_git(args: list[str]) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def gate_timestamp(name: str) -> int:
    path = ROOT / name
    if not path.exists():
        return 0
    return int(path.stat().st_mtime)


def gather_snapshot() -> RepoAnvilSnapshot:
    status_lines = run_git(["status", "--porcelain"]).splitlines()
    local_branches = [line for line in run_git(["branch"]).splitlines() if line.strip()]
    remote_branches = [line for line in run_git(["branch", "-r"]).splitlines() if line.strip()]
    return RepoAnvilSnapshot(
        repo=str(ROOT),
        branch=run_git(["branch", "--show-current"]),
        head=run_git(["log", "--oneline", "-1"]).split(" ", 1)[0],
        dirty=DirtyCounts(
            total=len(status_lines),
            modified=sum(1 for line in status_lines if line.startswith(" M") or line.startswith("M ")),
            untracked=sum(1 for line in status_lines if line.startswith("??")),
            deleted=sum(1 for line in status_lines if line.startswith(" D") or line.startswith("D ")),
            added=sum(1 for line in status_lines if line.startswith(" A") or line.startswith("A ")),
            renamed=sum(1 for line in status_lines if line.startswith("RM ")),
        ),
        branches=BranchCounts(local=len(local_branches), remote=len(remote_branches)),
        gates={
            "gate-0": gate_timestamp(".gate-0"),
            "gate-1": gate_timestamp(".gate-1"),
            "gate-2": gate_timestamp(".gate-2"),
            "gate-passed": gate_timestamp(".gate-passed"),
        },
        timestamp=utc_now(),
    )


def print_json(payload: dict[str, Any]) -> None:
    print(json.dumps(payload, indent=2, sort_keys=True))


def command_perceive(args: argparse.Namespace) -> int:
    service = RepoAnvilService()
    payload = service.perception_payload(gather_snapshot())
    if args.save:
        STATE_DIR.mkdir(parents=True, exist_ok=True)
        STATE_FILE.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print_json(payload)
    return 0


def command_state(_: argparse.Namespace) -> int:
    if not STATE_FILE.exists():
        print("No state file found.", file=sys.stderr)
        return 1
    print(STATE_FILE.read_text(encoding="utf-8"), end="")
    return 0


def command_audit(args: argparse.Namespace) -> int:
    try:
        payload = json.loads(args.payload)
    except json.JSONDecodeError as exc:
        print(f"invalid audit payload JSON: {exc}", file=sys.stderr)
        return 2
    entry = RepoAnvilService().audit_entry(
        timestamp=utc_now(),
        action=args.action,
        payload=payload,
        result=args.result,
    )
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    with AUDIT_FILE.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(entry, sort_keys=True) + "\n")
    print(json.dumps(entry, sort_keys=True))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Organ-forge Anvil repo-health entrypoint.")
    subparsers = parser.add_subparsers(dest="command", required=True)
    perceive = subparsers.add_parser("perceive")
    perceive.add_argument("--save", action="store_true")
    subparsers.add_parser("state")
    audit = subparsers.add_parser("audit")
    audit.add_argument("action")
    audit.add_argument("payload")
    audit.add_argument("result", nargs="?", default="success")
    args = parser.parse_args()

    if args.command == "perceive":
        return command_perceive(args)
    if args.command == "state":
        return command_state(args)
    if args.command == "audit":
        return command_audit(args)
    parser.error(f"unsupported command {args.command!r}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
