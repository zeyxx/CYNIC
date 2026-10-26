#!/usr/bin/env python3
"""CYNIC audit churn — measures the 8 dimensions of development health.

Computes metrics from git history and crate structure, compares against
thresholds in infra/churn-thresholds.json, reports OK/WARNING/ALARM per
dimension.

Usage: python3 scripts/audit_churn.py [--json]

TODO (post-freeze): port to Rust. Python is pragmatic for now.
"""

import json
import os
import re
import subprocess
import sys
from collections import Counter
from datetime import datetime, timedelta, timezone

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
THRESHOLDS_PATH = os.path.join(ROOT, "infra", "churn-thresholds.json")


def git(*args):
    r = subprocess.run(["git"] + list(args), capture_output=True, text=True, cwd=ROOT)
    return r.stdout


def metric_commits_per_week():
    out = git("log", "--since=7 days ago", "--format=%H")
    return len([l for l in out.splitlines() if l.strip()])


def metric_feature_ratio_pct():
    out = git("log", "--since=30 days ago", "--format=%s")
    subjects = [l for l in out.splitlines() if l.strip()]
    if not subjects:
        return 0.0
    feats = sum(1 for s in subjects if s.startswith("feat"))
    return round(100.0 * feats / len(subjects), 1)


def metric_files_touched_per_30d():
    out = git("log", "--since=30 days ago", "--name-only", "--format=")
    files = set(l.strip() for l in out.splitlines() if l.strip().endswith(".rs"))
    return len(files)


def metric_god_object_lines():
    max_lines = 0
    for root_dir, _, files in os.walk(os.path.join(ROOT, "crates")):
        if "target" in root_dir:
            continue
        for f in files:
            if not f.endswith(".rs"):
                continue
            # group by crate
            pass
    # count per crate
    crate_lines = {}
    for root_dir, _, files in os.walk(os.path.join(ROOT, "crates")):
        if "target" in root_dir:
            continue
        parts = root_dir.split(os.sep)
        # find crate name
        crate = None
        for i, p in enumerate(parts):
            if p == "crates" and i + 1 < len(parts):
                crate = parts[i + 1]
                break
        if not crate:
            continue
        for f in files:
            if not f.endswith(".rs"):
                continue
            fp = os.path.join(root_dir, f)
            try:
                with open(fp, errors="ignore") as fh:
                    crate_lines[crate] = crate_lines.get(crate, 0) + sum(1 for _ in fh)
            except Exception:
                pass
    if not crate_lines:
        return 0
    return max(crate_lines.values())


def metric_active_branches():
    out = git("branch", "--format=%(refname:short)")
    branches = [b.strip() for b in out.splitlines() if b.strip() and b.strip() != "main"]
    return len(branches)


def metric_author_identities():
    out = git("log", "--since=30 days ago", "--format=%an")
    authors = set(l.strip() for l in out.splitlines() if l.strip())
    return len(authors)


def metric_restore_commits_per_week():
    out = git("log", "--since=7 days ago", "--format=%s")
    subjects = [l for l in out.splitlines() if l.strip()]
    pattern = re.compile(r"(?i)(restore|fix\(main\)|compile)", re.IGNORECASE)
    return sum(1 for s in subjects if pattern.search(s))


def metric_docs_ratio_pct():
    out = git("log", "--since=30 days ago", "--format=%s")
    subjects = [l for l in out.splitlines() if l.strip()]
    if not subjects:
        return 0.0
    docs = sum(1 for s in subjects if s.startswith("docs"))
    return round(100.0 * docs / len(subjects), 1)


METRIC_FUNCS = {
    "commits_per_week": metric_commits_per_week,
    "feature_ratio_pct": metric_feature_ratio_pct,
    "files_touched_per_30d": metric_files_touched_per_30d,
    "god_object_lines": metric_god_object_lines,
    "active_branches": metric_active_branches,
    "author_identities": metric_author_identities,
    "restore_commits_per_week": metric_restore_commits_per_week,
    "docs_ratio_pct": metric_docs_ratio_pct,
}


def status_for(name, value, t):
    direction = t.get("direction", "above")
    w = t["threshold_warning"]
    a = t["threshold_alarm"]
    if direction == "below":
        if value < a:
            return "ALARM"
        if value < w:
            return "WARNING"
        return "OK"
    else:
        if value >= a:
            return "ALARM"
        if value >= w:
            return "WARNING"
        return "OK"


def main():
    with open(THRESHOLDS_PATH) as f:
        thresholds = json.load(f)["metrics"]

    results = {}
    overall = "OK"
    for name, func in METRIC_FUNCS.items():
        value = func()
        t = thresholds[name]
        st = status_for(name, value, t)
        results[name] = {
            "value": value,
            "unit": t["unit"],
            "threshold_warning": t["threshold_warning"],
            "threshold_alarm": t["threshold_alarm"],
            "status": st,
            "description": t["description"],
        }
        if st == "ALARM":
            overall = "ALARM"
        elif st == "WARNING" and overall != "ALARM":
            overall = "WARNING"

    if "--json" in sys.argv:
        print(json.dumps({"overall": overall, "metrics": results}, indent=2))
    else:
        print("=" * 60)
        print("CYNIC AUDIT CHURN")
        print("=" * 60)
        for name, r in results.items():
            icon = {"OK": "  ", "WARNING": " !", "ALARM": "!!"}[r["status"]]
            print(f"  [{icon}] {name:30s} {r['value']:>8} {r['unit']:>10}  ({r['status']})")
        print("-" * 60)
        print(f"  OVERALL: {overall}")
        print("=" * 60)
        if overall != "OK":
            print("\n  Proposals:")
            for name, r in results.items():
                if r["status"] in ("WARNING", "ALARM"):
                    print(f"    - {name}: {r['value']} {r['unit']} crosses {r['status']} threshold ({r['threshold_alarm'] if r['status']=='ALARM' else r['threshold_warning']})")
                    print(f"      Rule should evolve: see docs/policy/FREEZE.md")

    # exit code: 0=OK, 1=WARNING, 2=ALARM
    sys.exit({"OK": 0, "WARNING": 1, "ALARM": 2}[overall])


if __name__ == "__main__":
    main()
