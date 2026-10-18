#!/usr/bin/env python3
"""Read-only: compare the LIVE hermes state.db vs the ARCHIVED copy."""
from __future__ import annotations

import os
import sqlite3

LIVE = "/home/user/.hermes/state.db"
ARCH = "/var/lib/cynic/vault/archive/hermes_training_corpus/hermes_state_corrupted_source_20260924.db"


def probe(path, label):
    print(f"--- {label}: {path}")
    if not os.path.exists(path):
        print("    MISSING")
        return
    print(f"    size: {os.path.getsize(path):,} bytes")
    try:
        con = sqlite3.connect(f"file:{path}?immutable=1", uri=True)
        cur = con.cursor()
        integrity = cur.execute("PRAGMA integrity_check").fetchone()[0]
        print(f"    integrity_check: {integrity}")
        n_sessions = cur.execute("SELECT count(DISTINCT session_id) FROM messages").fetchone()[0]
        n_msgs = cur.execute("SELECT count(*) FROM messages").fetchone()[0]
        print(f"    sessions: {n_sessions:,}  messages: {n_msgs:,}")
        # latest timestamp
        try:
            mx = cur.execute("SELECT max(timestamp) FROM messages").fetchone()[0]
            print(f"    latest message ts: {mx}")
        except Exception:
            pass
        con.close()
    except Exception as e:
        print(f"    OPEN FAILED: {type(e).__name__}: {e}")


probe(LIVE, "LIVE")
print()
probe(ARCH, "ARCHIVED (2026-09-24)")

print()
print("=== WAL state of LIVE (proof it is mutating) ===")
for suffix in ("", "-wal", "-shm"):
    p = LIVE + suffix
    if os.path.exists(p):
        print(f"  {os.path.basename(p):<20} {os.path.getsize(p):>14,} bytes  mtime={os.path.getmtime(p):.0f}")

print()
print("=== archive/linux_sessions_2026 contents (are they full copies?) ===")
root = "/var/lib/cynic/vault/archive/linux_sessions_2026"
for d in sorted(os.listdir(root)):
    p = os.path.join(root, d)
    if os.path.isdir(p):
        n = sum(len(f) for _, _, f in os.walk(p))
        sz = sum(os.path.getsize(os.path.join(r, f)) for r, _, fs in os.walk(p) for f in fs)
        print(f"  {d:<24} {n:>6} files  {sz/1e6:>10.1f} MB")
