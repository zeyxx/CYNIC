#!/usr/bin/env python3
"""Non-vacuity harness — a regression fix is only a guard if it can fail.

A test that passes both BEFORE and AFTER its fix is vacuous: it guards nothing,
and the invariant it claims to protect can regress silently. This harness makes
the check mechanical. For each commit it reconstructs the PRE-fix source (via
`git show <sha>~1:<path>`), reruns the commit's tests against that source, and
FAILS LOUDLY if any of them still passes — a vacuous guard.

Two tables in one runner: the Python pipeline guards (paths re-anchored after
the infra/data_system -> apps/cynic-data move; pre-move paths kept as
fallbacks) and the Rust rewrite guards (tranche commits T2-T4; files created
by the commit itself are DELETED for the pre-fix run — a genesis file cannot
be reverted, only removed).

Mutation happens in the working tree and is restored in a finally block; the
run refuses to start on a dirty tree. Pass --python for the pytest rows and
--cargo for the cargo rows (both default to the current PATH).

Exit code 0 = every guard is non-vacuous (or documented known-vacuous);
1 = at least one guard is vacuous, unexpectedly failed, or could not be
evaluated.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
# apps/cynic-data lives at <repo>/apps/cynic-data (moved from infra/data_system).
REPO = os.path.dirname(os.path.dirname(HERE))

OLD_SRC = "infra/data_system/src/"
NEW_SRC = "apps/cynic-data/src/"

# sha -> guard. files: repo-relative paths to revert (tests stay fixed).
# fallbacks: pre-move paths tried when the repo-relative one is absent.
# run: command proving the guard (must FAIL on pre-fix source).
# cwd: repo-relative working dir. expect: "fail", or "known-vacuous" with note.
GUARDS: dict[str, dict] = {
    # --- Python pipeline (re-anchored post-move) ---
    "9f76aa88": {
        "files": [
            NEW_SRC + "cynic_data/canonical/adapters/normalizers/hermes.py",
            NEW_SRC + "cynic_data/canonical/adapters/normalizers/hermes_contract.py",
        ],
        "run": [sys.executable, "-m", "pytest", "-o", "pythonpath=src",
                "-q", "-p", "no:cacheprovider", "tests/test_hermes_conservation.py"],
        "cwd": "apps/cynic-data",
    },
    "eb3c43b6": {
        "files": [NEW_SRC + "cynic_data/canonical/adapters/normalizers/hermes.py"],
        "run": [sys.executable, "-m", "pytest", "-o", "pythonpath=src",
                "-q", "-p", "no:cacheprovider", "tests/test_hermes_conservation.py"],
        "cwd": "apps/cynic-data",
    },
    "311058ac": {
        "files": [
            NEW_SRC + "cynic_data/canonical/adapters/normalizers/hermes.py",
            NEW_SRC + "cynic_data/canonical/adapters/store_sqlite.py",
            NEW_SRC + "cynic_data/canonical/domain/entities.py",
        ],
        "run": [sys.executable, "-m", "pytest", "-o", "pythonpath=src",
                "-q", "-p", "no:cacheprovider",
                "tests/test_hermes_conservation.py", "tests/test_architecture_boundary.py"],
        "cwd": "apps/cynic-data",
    },
    "7c64d2e2": {
        "files": [
            NEW_SRC + "cynic_data/canonical/domain/conservation.py",
            NEW_SRC + "cynic_data/canonical/adapters/normalizers/hermes_contract.py",
            NEW_SRC + "cynic_data/canonical/adapters/normalizers/hermes.py",
        ],
        "run": [sys.executable, "-m", "pytest", "-o", "pythonpath=src",
                "-q", "-p", "no:cacheprovider", "tests/test_hermes_conservation.py"],
        "cwd": "apps/cynic-data",
    },
    # --- Rust rewrite (tranches T2-T4) ---
    "f70d6645": {
        "files": [
            "crates/cynic-traces/src/conservation.rs",
            "crates/cynic-traces/src/kernel/contract.rs",
            "crates/cynic-traces/src/kernel/errors.rs",
            "crates/cynic-traces/src/kernel/ids.rs",
            "crates/cynic-traces/src/kernel/mod.rs",
            "crates/cynic-traces/src/lib.rs",
            "crates/cynic-traces/src/ports.rs",
        ],
        "run": ["cargo", "test", "-p", "cynic-traces", "--lib", "-q"],
        "cwd": ".",
        "env": {"RUSTFLAGS": "-C debuginfo=0"},
    },
    "44a109c0": {
        "files": [
            "crates/cynic-traces/src/adapters/hermes/contract.rs",
            "crates/cynic-traces/src/adapters/hermes/human.rs",
            "crates/cynic-traces/src/adapters/hermes/mod.rs",
            "crates/cynic-traces/src/adapters/hermes/normalizer.rs",
            "crates/cynic-traces/src/adapters/mod.rs",
            "crates/cynic-traces/src/kernel/ids.rs",
            "crates/cynic-traces/src/lib.rs",
        ],
        "run": ["cargo", "test", "-p", "cynic-traces", "--test", "hermes_parity", "-q"],
        "cwd": ".",
        "env": {"RUSTFLAGS": "-C debuginfo=0"},
    },
    "ef91f1af": {
        "files": [
            "crates/cynic-traces/src/adapters/content_store.rs",
            "crates/cynic-traces/src/adapters/mod.rs",
            "crates/cynic-traces/src/adapters/store_schema.rs",
            "crates/cynic-traces/src/adapters/store_sqlite.rs",
            "crates/cynic-traces/src/conservation.rs",
            "crates/cynic-traces/src/lib.rs",
            "crates/cynic-traces/src/usecases.rs",
        ],
        "run": ["cargo", "test", "-p", "cynic-traces", "--test", "conservation_proof", "-q"],
        "cwd": ".",
        "env": {"RUSTFLAGS": "-C debuginfo=0"},
    },
    "f3767cff": {
        "files": [
            "crates/cynic-traces/src/adapters/hermes/contract.rs",
            "crates/cynic-traces/src/adapters/store_sqlite.rs",
            "crates/cynic-traces/src/lib.rs",
        ],
        "run": ["cargo", "test", "-p", "cynic-traces", "--test", "conservation_proof", "-q"],
        "cwd": ".",
        "env": {"RUSTFLAGS": "-C debuginfo=0"},
        "expect": "known-vacuous",
        "note": "E7: transactional hardening has no fault-injection test; "
                "success-path behavior is identical pre/post. Guarded by review only.",
    },
}


def _git(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git", "-C", REPO, *args], capture_output=True, text=True)


def _resolve(sha: str, path: str) -> tuple[str | None, str]:
    """Return (pre-fix content or None if genesis, action description)."""
    if path.startswith(NEW_SRC):
        for cand in (path, OLD_SRC + path[len(NEW_SRC):]):
            r = _git("show", f"{sha}~1:{cand}")
            if r.returncode == 0:
                return r.stdout, f"revert {cand}"
    else:
        r = _git("show", f"{sha}~1:{path}")
        if r.returncode == 0:
            return r.stdout, f"revert {path}"
    if _git("cat-file", "-e", f"{sha}:{path}").returncode == 0:
        return None, f"delete {path} (genesis in {sha})"
    raise LookupError(f"{sha}: {path} exists neither pre-fix nor at fix")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--python", default=sys.executable)
    ap.add_argument("--cargo", default="cargo")
    ap.add_argument("--only", default=None)
    args = ap.parse_args()

    guards = {k: v for k, v in GUARDS.items() if args.only in (None, k)}
    if not guards:
        print(f"no guard matches --only {args.only!r}", file=sys.stderr)
        return 1
    if _git("status", "--short").stdout.strip():
        print("refusing: working tree is dirty", file=sys.stderr)
        return 1

    vacuous: list[str] = []
    unexpected: list[str] = []
    errors: list[str] = []
    for sha, g in guards.items():
        run = [args.python if c == sys.executable else
               args.cargo if c == "cargo" else c for c in g["run"]]
        saved: dict[str, str | None] = {}
        deleted: list[str] = []
        ok = True
        try:
            for rel in g["files"]:
                dest = os.path.join(REPO, rel)
                try:
                    pre, action = _resolve(sha, rel)
                except LookupError as e:
                    errors.append(str(e))
                    ok = False
                    break
                saved[rel] = open(dest, encoding="utf-8").read() if os.path.exists(dest) else None
                if pre is None:
                    os.remove(dest)
                    deleted.append(rel)
                else:
                    open(dest, "w", encoding="utf-8").write(pre)
            if not ok:
                continue
            env = dict(os.environ)
            env.update(g.get("env", {}))
            r = subprocess.run(run, cwd=os.path.join(REPO, g["cwd"]),
                               capture_output=True, text=True, env=env)
            out = ((r.stdout or "") + (r.stderr or "")).strip().splitlines()
            last = out[-1] if out else "(no output)"
            failed = r.returncode != 0
            expect = g.get("expect", "fail")
            if expect == "known-vacuous":
                if failed:
                    unexpected.append(f"{sha}: known-vacuous guard FAILED ({last})")
                    print(f"  revert {sha}: {last}  -> UNEXPECTED FAIL")
                else:
                    print(f"  revert {sha}: {last}  -> known-vacuous ({g.get('note', '')})")
            elif failed:
                print(f"  revert {sha}: {last}  -> non-vacuous")
            else:
                vacuous.append(sha)
                print(f"  revert {sha}: {last}  -> VACUOUS")
        finally:
            for rel, text in saved.items():
                dest = os.path.join(REPO, rel)
                if text is None:
                    if os.path.exists(dest):
                        os.remove(dest)
                else:
                    open(dest, "w", encoding="utf-8").write(text)
        _ = deleted

    print()
    if unexpected:
        for u in unexpected:
            print(f"  UNEXPECTED {u}", file=sys.stderr)
    if errors:
        for e in errors:
            print(f"  ERROR {e}", file=sys.stderr)
    if vacuous:
        print(f"FAIL: vacuous guards (tests pass on pre-fix source): {vacuous}")
        return 1
    if unexpected or errors:
        return 1
    n = len(guards)
    kv = sum(1 for g in guards.values() if g.get("expect") == "known-vacuous")
    print(f"OK: {n - kv} guards are non-vacuous, {kv} documented known-vacuous")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
