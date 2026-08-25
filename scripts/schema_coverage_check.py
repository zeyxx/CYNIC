#!/usr/bin/env python3
"""
Python mirror of crates/cynic-kernel/tests/schema_coverage.rs.

Same three checks against the real storage sources:
  C1 manifest covers every runtime table
  C2 sql declarations are unique and covered by the manifest
  C3 declared tables have runtime consumers

Run from repo root or anywhere: paths are resolved relative to this file.
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
KERNEL = os.path.normpath(os.path.join(HERE, "../crates/cynic-kernel"))
MANIFEST = os.path.join(KERNEL, "src/storage/schema_manifest.json")
SQL_DIR = os.path.join(KERNEL, "src/storage")
ADAPTER_DIRS = [os.path.join(KERNEL, "src/storage/surreal"), os.path.join(KERNEL, "src/storage/memory")]
ADAPTER_FILES = [os.path.join(KERNEL, "src/storage/memory.rs"), os.path.join(KERNEL, "src/storage/surreal.rs")]

VERB_RE = re.compile(r"(?:FROM|INTO|UPDATE|UPSERT ONLY|CREATE ONLY|CREATE|UPSERT) ([a-z_]+)")
DEFINE_RE = re.compile(r"DEFINE TABLE (?:IF NOT EXISTS |OVERWRITE )?([a-z_]+)")
STOP = {"on", "only", "verdict_id", "now"}


def load_manifest():
    data = json.load(open(MANIFEST))
    assert data["schema"] == "cynic.storage.schema_manifest.v1"
    manifest = {t["name"]: t for t in data["tables"]}
    manifest.update(
        {k: {"mode": "sqlite_plane"} for k in data.get("sqlite_plane_tables", {})}
    )
    return manifest


def collect_runtime_tables():
    """Whole-crate scan: tables are touched outside storage/ too (sqlite
    planes, external sensors) — the 2026-08-24 live reconciliation proved it."""
    used = set()
    files = []
    src_root = os.path.join(KERNEL, "src")
    for root, _, names in os.walk(src_root):
        files.extend(os.path.join(root, n) for n in names if n.endswith(".rs"))
    for f in files:
        src = open(f, encoding="utf-8", errors="replace").read()
        used.update(m.group(1) for m in VERB_RE.finditer(src))
    return sorted(u for u in used if u not in STOP)


def collect_sql_declarations():
    out = {}
    for f in sorted(os.listdir(SQL_DIR)):
        if not f.endswith(".sql"):
            continue
        src = open(os.path.join(SQL_DIR, f)).read()
        for m in DEFINE_RE.finditer(src):
            out.setdefault(m.group(1), []).append(f)
    return out


def main():
    manifest = load_manifest()
    used = collect_runtime_tables()
    sql = collect_sql_declarations()

    c1_missing = [t for t in used if t not in manifest]
    c2_dupes = {k: v for k, v in sql.items() if len(v) > 1}
    c2_undeclared = [t for t in sql if t not in manifest]
    c3_orphans = [t for t in manifest if t not in used]

    ok1, ok2, ok3 = not c1_missing, not c2_dupes and not c2_undeclared, not c3_orphans
    print(f"C1 manifest_covers_every_runtime_table : {'PASS' if ok1 else 'FAIL'}")
    if not ok1:
        print(f"   missing: {c1_missing}")
    print(f"C2 sql_unique_and_covered              : {'PASS' if ok2 else 'FAIL'}")
    if not ok2:
        print(f"   dupes: {c2_dupes}  undeclared-in-manifest: {c2_undeclared}")
    print(f"C3 no_orphan_declarations              : {'PASS' if ok3 else 'FAIL'}")
    if not ok3:
        print(f"   orphans: {c3_orphans}")

    print(f"\nmanifest tables={len(manifest)} runtime tables={len(used)} sql-declared={len(sql)}")
    verdict = "ALL SCHEMA COVERAGE CHECKS PASS" if (ok1 and ok2 and ok3) else "FAILURES PRESENT"
    print(verdict)
    sys.exit(0 if (ok1 and ok2 and ok3) else 1)


if __name__ == "__main__":
    main()
