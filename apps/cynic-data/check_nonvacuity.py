#!/usr/bin/env python3
"""Non-vacuity harness — a regression fix is only a guard if it can fail.

A test that passes both BEFORE and AFTER its fix is vacuous: it guards nothing,
and the invariant it claims to protect can regress silently. This harness makes
the check mechanical. For each commit it reconstructs the PRE-fix source (via
`git show <sha>~1:<path>`), reruns the tests the commit added against that
source, and FAILS LOUDLY if any of them still passes — a vacuous guard.

Read-only on the repository: all mutation happens in a scratch copy of `src/`,
never in the working tree.

Run with the project venv:

    /home/user/.hermes/cache/scratch/dsvenv/bin/python check_nonvacuity.py

Exit code 0 = every guard is non-vacuous; 1 = at least one guard is vacuous or
could not be evaluated.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
# data_system lives at <repo>/infra/data_system
# data_system lives at <repo>/apps/cynic-data (moved from infra/data_system).
# GUARDS entries predating the move cannot resolve and report an error;
# re-anchor the table to post-move commits (see Rust-rewrite spec).
REPO = os.path.dirname(os.path.dirname(HERE))

# commit -> (src files to revert, relative to src/, tests that MUST fail on the
# pre-fix source). Keep this table in sync with the commits that changed src/.
GUARDS: dict[str, tuple[list[str], list[str]]] = {
    # BLOB display_identity, logical order, model_usage, n-ary compaction
    "9f76aa88": (
        [
            "cynic_data/canonical/adapters/normalizers/hermes.py",
            "cynic_data/canonical/adapters/normalizers/hermes_contract.py",
        ],
        ["tests/test_hermes_conservation.py"],
    ),
    # session enumeration — 366 message-less sessions were invisible
    "eb3c43b6": (
        ["cynic_data/canonical/adapters/normalizers/hermes.py"],
        ["tests/test_hermes_conservation.py"],
    ),
    # NULL != "" enforcement + architecture boundary hardening
    "311058ac": (
        [
            "cynic_data/canonical/adapters/normalizers/hermes.py",
            "cynic_data/canonical/adapters/store_sqlite.py",
            "cynic_data/canonical/domain/entities.py",
        ],
        ["tests/test_hermes_conservation.py", "tests/test_architecture_boundary.py"],
    ),
    # closure over the full producer DDL — every table decided
    "7c64d2e2": (
        [
            "cynic_data/canonical/domain/conservation.py",
            "cynic_data/canonical/adapters/normalizers/hermes_contract.py",
            "cynic_data/canonical/adapters/normalizers/hermes.py",
        ],
        ["tests/test_hermes_conservation.py"],
    ),
}


def _git_show(sha: str, path: str) -> str | None:
    r = subprocess.run(
        ["git", "-C", REPO, "show", f"{sha}~1:infra/data_system/src/{path}"],
        ["git", "-C", REPO, "show", f"{sha}~1:apps/cynic-data/src/{path}"],
        capture_output=True,
        text=True,
    )
    return r.stdout if r.returncode == 0 else None


def _run_tests(python: str, scratch_src: str, targets: list[str]) -> str:
    # `-o pythonpath=` overrides pyproject's pinned `pythonpath = ["src"]`, which
    # would otherwise shadow the scratch source and make the revert a no-op.
    cmd = [
        python, "-m", "pytest", "-o", f"pythonpath={scratch_src}",
        "-q", "-p", "no:cacheprovider", *targets,
    ]
    r = subprocess.run(cmd, cwd=HERE, capture_output=True, text=True)
    return (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout or r.stderr) else ""


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--python", default=sys.executable, help="interpreter for pytest")
    ap.add_argument("--only", default=None, help="evaluate a single commit sha")
    args = ap.parse_args()

    guards = {k: v for k, v in GUARDS.items() if args.only in (None, k)}
    if not guards:
        print(f"no guard matches --only {args.only!r}", file=sys.stderr)
        return 1

    scratch = tempfile.mkdtemp(prefix="nonvacuity-")
    scratch_src = os.path.join(scratch, "src")
    shutil.copytree(os.path.join(HERE, "src"), scratch_src)

    vacuous: list[str] = []
    errors: list[str] = []
    try:
        for sha, (files, targets) in guards.items():
            # snapshot the fixed sources, revert them, run, restore
            saved: dict[str, str] = {}
            ok = True
            for rel in files:
                dest = os.path.join(scratch_src, rel)
                saved[rel] = open(dest, encoding="utf-8").read()
                pre = _git_show(sha, rel)
                if pre is None:
                    errors.append(f"{sha}: cannot read {sha}~1:{rel}")
                    ok = False
                    break
                open(dest, "w", encoding="utf-8").write(pre)
            if ok:
                last = _run_tests(args.python, scratch_src, targets)
                failed = ("failed" in last) or ("error" in last.lower())
                verdict = "non-vacuous" if failed else "VACUOUS"
                if not failed:
                    vacuous.append(sha)
                print(f"  revert {sha}: {last}  -> {verdict}")
            for rel, text in saved.items():
                open(os.path.join(scratch_src, rel), "w", encoding="utf-8").write(text)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)

    print()
    if errors:
        for e in errors:
            print(f"  ERROR {e}", file=sys.stderr)
    if vacuous:
        print(f"FAIL: vacuous guards (tests pass on pre-fix source): {vacuous}")
        return 1
    if errors:
        return 1
    print(f"OK: {len(guards)} guards are non-vacuous")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
