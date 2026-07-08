#!/usr/bin/env python3
"""Tier 1 EXPERIMENTAL: Bootstrap the Quantum ECC harness without a shell entrypoint.

Status: ACTIVE (started 2026-06-02). Delete by 2026-07-02 if not promoted.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_DIR = SCRIPT_DIR / "ecdsafail-challenge"
REPO_URL = "https://github.com/ecdsafail/ecdsafail-challenge"
STATE_FILE = SCRIPT_DIR / "state.json"


class BootstrapError(RuntimeError):
    pass


def require_tool(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        raise BootstrapError(f"ERROR: {name} not found.")
    return path


def run(
    args: list[str],
    *,
    cwd: Path | None = None,
    timeout: int | None = None,
    capture: bool = False,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=cwd,
        timeout=timeout,
        check=True,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.STDOUT if capture else None,
    )


def tool_version(args: list[str], field: int) -> str:
    result = run(args, capture=True)
    parts = (result.stdout or "").strip().split()
    return parts[field] if len(parts) > field else "unknown"


def sync_challenge_repo() -> None:
    if (REPO_DIR / ".git").is_dir():
        print("Repo exists - pulling latest...")
        run(["git", "-C", str(REPO_DIR), "pull", "--ff-only"], timeout=30)
        return

    print(f"Cloning {REPO_URL}...")
    run(["git", "clone", REPO_URL, str(REPO_DIR)], timeout=120)


def parse_metric(output: str, pattern: str) -> int:
    match = re.search(pattern, output, re.MULTILINE)
    if match is None:
        print(f"WARNING: pattern {pattern!r} not found in eval output", file=sys.stderr)
        return 0
    return int(float(match.group(1)))


def update_state(eval_output: str) -> None:
    toffoli = parse_metric(eval_output, r"avg executed Toffoli\s*:\s*([\d.]+)")
    qubits = parse_metric(eval_output, r"qubits\s*:\s*([\d.]+)")
    product = toffoli * qubits

    existing_baseline = 0
    if STATE_FILE.exists():
        try:
            existing_baseline = json.loads(STATE_FILE.read_text()).get("baseline_product", 0)
        except (OSError, json.JSONDecodeError):
            existing_baseline = 0

    if existing_baseline:
        print(f"State file exists (baseline={existing_baseline:,}) - preserving progress.")
        return

    STATE_FILE.write_text(
        json.dumps(
            {
                "best_product": product,
                "best_gate_count": toffoli,
                "best_qubit_count": qubits,
                "tried_levers": [],
                "iterations": [],
                "baseline_product": product,
            },
            indent=2,
        )
        + "\n"
    )
    print(f"State initialized: product={product:,} ({toffoli:,} gates x {qubits:,} qubits)")


def main() -> int:
    print("=== Quantum ECC Harness Bootstrap ===")

    try:
        require_tool("cargo")
        require_tool("git")
    except BootstrapError as exc:
        print(exc, file=sys.stderr)
        return 1

    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("WARNING: ANTHROPIC_API_KEY not set - needed by harness.py, not by bootstrap.")

    cargo_version = tool_version(["cargo", "--version"], 1)
    git_version = tool_version(["git", "--version"], 2)
    print(f"cargo {cargo_version}, git {git_version}")

    try:
        sync_challenge_repo()
        print(f"Challenge repo at {REPO_DIR}")
        print("Running baseline evaluation (2-5 min first time)...")
        run(["cargo", "build", "--release"], cwd=REPO_DIR)
        run(["cargo", "run", "--release", "--bin", "build_circuit"], cwd=REPO_DIR)
        eval_result = run(
            ["cargo", "run", "--release", "--bin", "eval_circuit"],
            cwd=REPO_DIR,
            capture=True,
        )
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
        print(f"ERROR: bootstrap command failed: {exc}", file=sys.stderr)
        return 1

    eval_output = eval_result.stdout or ""
    print(eval_output, end="" if eval_output.endswith("\n") else "\n")
    update_state(eval_output)

    print()
    print("=== Bootstrap complete ===")
    print()
    print("To run the harness:")
    print(f"  cd {SCRIPT_DIR} && python3 harness.py --repo {REPO_DIR} --max-iters 20")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
