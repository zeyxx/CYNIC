#!/usr/bin/env python3
"""Static markdown fragmentation audit for CYNIC.

This emits JSON to help prove SSOT compliance around markdown placement.

Usage:
  python3 scripts/markdown-fragmentation-audit.py
  python3 scripts/markdown-fragmentation-audit.py --staged
  python3 scripts/markdown-fragmentation-audit.py --json
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List

SCRIPT_DIR = Path(__file__).resolve().parent
POLICY_PATH = SCRIPT_DIR / "markdown-fragmentation-policy.json"

DEFAULT_POLICY = {
    "special_allowed": [
        "scripts/README-token-screening.md",
        "scripts/hermes-x/HERMES_LOGIN_SETUP.md",
        "organs/*/state/README.md",
        "organs/*/state/SKILL.md",
        "apps/kyon-mobile/README.md",
    ],
    "canonical_prefixes": [
        "docs/",
        "packages/",
        "crates/",
        "organs/",
        "cynic/",
        ".cortex/",
        ".cynic/",
        "data/",
        ".github/",
    ],
    "canonical_patterns": [
        "organs/*/README.md",
        "organs/*/SKILL.md",
        "organs/twitter/*.md",
        "organs/twitter/src/*.md",
        "organs/organ-cloud/*.md",
        "packages/*/README.md",
        "packages/*/CLAUDE.md",
        "crates/*/README.md",
        "crates/*/BENCHMARK_ROUTING.md",
        "crates/*/domains/*.md",
        "crates/percolator-cli/*.md",
        "crates/percolator-cli/DEPLOY_CHECKLIST.md",
    ],
    "canonical_roots": [
        "CLAUDE.md",
        "AGENTS.md",
        "CODEX.md",
        "ANTIGRAVITY.md",
        "README.md",
        "CHANGELOG.md",
        "API.md",
        "VERSION.md",
        "TOPOLOGY.md",
        ".codex/CODEX-PROTOCOL.md",
        "packages/cynic-ui/CLAUDE.md",
        "packages/talaria-demo/README.md",
        "packages/talaria-landing/README.md",
        "crates/cynic-askesis/README.md",
        "apps/kyon-mobile/README.md",
    ],
    "runtime_prefixes": [
        "services/",
        "infra/",
        "scripts/",
        "cynic-python/",
        "apps/",
        "organs/*/state/",
    ],
}


def _run(cmd: List[str]) -> List[str]:
    return subprocess.check_output(cmd, text=True).strip().splitlines()


def _load_policy() -> Dict[str, List[str]]:
    if POLICY_PATH.exists():
        with POLICY_PATH.open("r", encoding="utf-8") as fh:
            payload = json.load(fh)
        if isinstance(payload, dict):
            return payload
    return DEFAULT_POLICY


def _matches(path: str, patterns: Iterable[str]) -> bool:
    for pattern in patterns:
        if fnmatch.fnmatch(path, pattern):
            return True
    return False


def _has_prefix(path: str, prefixes: Iterable[str]) -> bool:
    for prefix in prefixes:
        if path.startswith(prefix):
            return True
    return False


def _is_md(path: str) -> bool:
    return path.endswith(".md")


def _classify_markdowns(paths: Iterable[str], policy: Dict[str, List[str]]) -> Dict[str, List[str]]:
    special = policy.get("special_allowed", [])
    prefixes = policy.get("canonical_prefixes", [])
    patterns = policy.get("canonical_patterns", [])
    roots = set(policy.get("canonical_roots", []))
    runtime = policy.get("runtime_prefixes", [])

    out = {
        "allowed": [],
        "forbidden": [],
        "forbidden_runtime": [],
        "forbidden_unanchored": [],
    }

    for path in paths:
        if not _is_md(path):
            continue

        if _matches(path, special) or path in roots or path in {
            "scripts/README-token-screening.md",
            "scripts/hermes-x/HERMES_LOGIN_SETUP.md",
            "apps/kyon-mobile/README.md",
        }:
            out["allowed"].append(path)
            continue

        if _has_prefix(path, prefixes) or _matches(path, patterns) or path in roots:
            out["allowed"].append(path)
            continue

        if _matches(path, [p + "*" for p in runtime] if runtime else []):
            out["forbidden_runtime"].append(path)
            out["forbidden"].append(path)
            continue

        # Non-runtime but non-canonical docs are treated as forbidden for audit traceability.
        out["forbidden_unanchored"].append(path)
        out["forbidden"].append(path)

    return out


def _candidate_files(staged: bool) -> List[str]:
    if staged:
        return _run(["git", "diff", "--cached", "--name-only", "--diff-filter=ACM"])
    return _run(["git", "ls-files", "*.md"])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--staged", action="store_true", help="Audit only staged markdown files")
    parser.add_argument("--json", action="store_true", help="Force JSON output (default)")
    args = parser.parse_args()

    policy = _load_policy()
    files = sorted(_candidate_files(args.staged))
    result = _classify_markdowns(files, policy)

    payload = {
        "audit": "markdown-fragmentation",
        "scope": "staged" if args.staged else "tracked",
        "checked_at": datetime.now(timezone.utc).isoformat(),
        "policy": {
            "path": str(POLICY_PATH),
            "exists": POLICY_PATH.exists(),
        },
        "totals": {
            "markdown_files": len([x for x in files if x.endswith(".md")]),
            "allowed": len(result["allowed"]),
            "forbidden": len(result["forbidden"]),
        },
        "allowed": sorted(result["allowed"]),
        "forbidden": sorted(result["forbidden"]),
        "forbidden_runtime": sorted(result["forbidden_runtime"]),
        "forbidden_unanchored": sorted(result["forbidden_unanchored"]),
        "status": "ok" if not result["forbidden"] else "violation",
    }

    print(json.dumps(payload, indent=2, sort_keys=True))

    return 0 if payload["status"] == "ok" else 1


if __name__ == "__main__":
    raise SystemExit(main())
