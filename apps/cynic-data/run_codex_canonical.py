#!/usr/bin/env python3
"""Scale run: normalize ALL codex sessions FROM A FROZEN CAPTURE.

Fails closed: the capture path is required (CYNIC_CODEX_CAPTURE or --source).
No live fallback. Writes only to the new canonical store.
"""

from __future__ import annotations

import json
import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from cynic_data.canonical.adapters.normalizers.codex import CodexNormalizer
from cynic_data.canonical.adapters.store_sqlite import SqliteCanonicalStore
from cynic_data.canonical.usecases.normalize import normalize_source

OUT_DB = os.environ.get("CYNIC_CANONICAL_DB", "/home/user/.hermes/cache/scratch/canonical_codex.sqlite")


def main() -> int:
    import argparse

    ap = argparse.ArgumentParser()
    ap.add_argument("--source", default=os.environ.get("CYNIC_CODEX_CAPTURE", ""))
    ap.add_argument("--out", default=OUT_DB)
    args = ap.parse_args()

    if not args.source or not os.path.isdir(args.source):
        print(
            "[canonical] FATAL: a frozen codex capture is required "
            "(--source or CYNIC_CODEX_CAPTURE). Live reads are refused.",
            file=sys.stderr,
        )
        return 2

    if os.path.exists(args.out):
        os.remove(args.out)
    os.makedirs(os.path.dirname(args.out), exist_ok=True)

    normalizer = CodexNormalizer(args.source)
    refs = normalizer.session_refs()
    print(f"[canonical] codex sessions to normalize: {len(refs)}")
    unreadable = [r for r in refs if not normalizer.can_read(r)]
    if unreadable:
        print(f"[canonical] FATAL: {len(unreadable)} sessions fail can_read", file=sys.stderr)
        return 3

    store = SqliteCanonicalStore(args.out, blob_root=args.out + ".blobs", bulk_load=True)
    t0 = time.time()
    report = normalize_source(normalizer, store, progress_every=50)
    elapsed = time.time() - t0

    print()
    print(f"[canonical] normalized {report.sessions_ok}/{len(refs)} in {elapsed:.1f}s "
          f"({report.sessions_failed} failed)")
    print("[canonical] totals:")
    for k, v in sorted(report.totals.items()):
        print(f"    {k:<14} {v:>12,}")

    print()
    print("[canonical] store row counts (read-back):")
    for table in ("sessions", "turns", "tools", "reasoning", "edges", "states", "human_signals"):
        print(f"    {table:<14} {store.count(table):>12,}")

    print()
    print("[canonical] read-back proof:")
    biggest = store._con.execute(
        "SELECT session_uid, count(*) n FROM turns GROUP BY session_uid ORDER BY n DESC LIMIT 1"
    ).fetchone()
    if biggest:
        uid = biggest[0]
        reloaded = store.read(uid)
        assert reloaded is not None
        reloaded.check()
        print(f"    {uid} turns={len(reloaded.turns)} tools={len(reloaded.tools)} check=OK")
        print("[canonical] REPORT_JSON " + json.dumps({
            "sessions_ok": report.sessions_ok,
            "sessions_failed": report.sessions_failed,
            "totals": report.totals,
            "read_back": uid,
        }, sort_keys=True))
    if report.failures:
        print("[canonical] failures:")
        for ref, err in report.failures[:10]:
            print(f"    {ref[:60]}: {err}")
    store.close()
    return 0 if report.sessions_failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
