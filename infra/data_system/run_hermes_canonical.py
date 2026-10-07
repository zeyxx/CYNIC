#!/usr/bin/env python3
"""Scale run: normalize ALL hermes sessions into the canonical store, then read back.

Read-only against the source state.db (immutable). Writes only to the new
canonical store. Prints a machine-readable summary.
"""

from __future__ import annotations

import json
import os
import sqlite3
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from cynic_data.canonical.adapters.normalizers.hermes import HermesNormalizer
from cynic_data.canonical.adapters.store_sqlite import SqliteCanonicalStore
from cynic_data.canonical.usecases.normalize import normalize_many

HERMES_DB = os.environ.get("CYNIC_HERMES_DB", "/home/user/.hermes/state.db")
OUT_DB = os.environ.get("CYNIC_CANONICAL_DB", "/home/user/.hermes/cache/scratch/canonical_hermes.sqlite")


def hermes_session_ids(db: str) -> list[str]:
    con = sqlite3.connect(f"file:{db}?immutable=1", uri=True)
    try:
        rows = con.execute(
            "SELECT DISTINCT session_id FROM messages WHERE session_id IS NOT NULL ORDER BY session_id"
        ).fetchall()
    finally:
        con.close()
    return [r[0] for r in rows]


def main() -> int:
    if os.path.exists(OUT_DB):
        os.remove(OUT_DB)
    os.makedirs(os.path.dirname(OUT_DB), exist_ok=True)

    ids = hermes_session_ids(HERMES_DB)
    print(f"[canonical] hermes sessions to normalize: {len(ids)}")

    store = SqliteCanonicalStore(OUT_DB, blob_root=OUT_DB + ".blobs", bulk_load=True)
    t0 = time.time()
    report = normalize_many(HermesNormalizer(HERMES_DB), store, ids, progress_every=500)
    elapsed = time.time() - t0

    print()
    print(f"[canonical] normalized {report.sessions_ok}/{len(ids)} in {elapsed:.1f}s "
          f"({report.sessions_failed} failed)")
    print("[canonical] totals:")
    for k, v in sorted(report.totals.items()):
        print(f"    {k:<14} {v:>12,}")

    print()
    print("[canonical] store row counts (read-back):")
    for table in ("sessions", "turns", "tools", "reasoning", "edges", "states", "human_signals"):
        print(f"    {table:<14} {store.count(table):>12,}")

    # read-back proof: reload the largest session and verify it matches
    print()
    print("[canonical] read-back proof:")
    biggest = store._con.execute(
        "SELECT session_uid, count(*) n FROM turns GROUP BY session_uid ORDER BY n DESC LIMIT 1"
    ).fetchone()
    if biggest:
        uid = biggest[0]
        reloaded = store.read(uid)
        reloaded.check()
        st = reloaded.stats()
        print(f"    session {uid} (source turns={biggest[1]})")
        for k, v in st.items():
            print(f"      {k:<14} {v:>12,}")

    if report.failures:
        print()
        print("[canonical] failures (first 10):")
        for rid, err in report.failures[:10]:
            print(f"    {rid}: {err}")

    store.close()
    summary = {
        "harness": "hermes",
        "sessions": len(ids),
        "ok": report.sessions_ok,
        "failed": report.sessions_failed,
        "elapsed_s": round(elapsed, 1),
        "totals": report.totals,
    }
    print()
    print("[canonical] SUMMARY_JSON " + json.dumps(summary, sort_keys=True))
    return 0 if report.sessions_failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
