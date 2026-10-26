#!/usr/bin/env python3
"""Detect hardcoded infrastructure values in Rust source.

Counts occurrences of known IPs, hostnames, and ports in .rs files.
Reports per-file and total. Use as a ratchet: baseline must only shrink.

Exit code 0 always (informational, not a gate).
"""

import json
import re
import sys
from pathlib import Path

PATTERNS = [
    (re.compile(r"10\.10\.10\.\d+"), "ip_service_plane"),
    (re.compile(r"192\.168\.\d+\.\d+"), "ip_lan"),
    (re.compile(r"cynic-builder-ssh"), "ssh_alias"),
    (re.compile(r"10\.20\.0\.\d+"), "ip_overlay"),
]

EXCLUDE_DIRS = {"target", ".git", "tests"}


def scan(root: Path) -> dict:
    results = {}
    total = 0
    for rs_file in sorted(root.rglob("*.rs")):
        if any(part in EXCLUDE_DIRS for part in rs_file.parts):
            continue
        try:
            content = rs_file.read_text()
        except Exception:
            continue
        file_hits = {}
        for pat, label in PATTERNS:
            matches = pat.findall(content)
            if matches:
                file_hits[label] = len(matches)
                total += len(matches)
        if file_hits:
            rel = str(rs_file.relative_to(root))
            results[rel] = file_hits
    return {"files": results, "total": total, "file_count": len(results)}


def main():
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
    report = scan(root)
    print(f"Hardcoded infra values: {report['total']} in {report['file_count']} files")
    for f, hits in sorted(report["files"].items()):
        print(f"  {f}: {hits}")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
