#!/usr/bin/env python3
"""Conservation proof: canonical store vs source state.db, and vs old Silver.

Read-only. No mutation.
"""
from __future__ import annotations

import sqlite3

SRC = "/home/user/.hermes/state.db"
CANON = "/home/user/.hermes/cache/scratch/canonical_hermes.sqlite"
SILVER = "/var/lib/cynic/vault/corpus/silver/corpus_master.sqlite"

print("=" * 78)
print("CONSERVATION: canonical vs source")
print("=" * 78)
src = sqlite3.connect(f"file:{SRC}?immutable=1", uri=True)
n_src_sessions = src.execute("SELECT count(DISTINCT session_id) FROM messages").fetchone()[0]
n_src_msgs = src.execute("SELECT count(*) FROM messages").fetchone()[0]
n_src_user = src.execute("SELECT count(*) FROM messages WHERE role='user'").fetchone()[0]
n_src_reason = src.execute("SELECT count(*) FROM messages WHERE reasoning_content IS NOT NULL AND reasoning_content != ''").fetchone()[0]
n_src_compact = src.execute("SELECT count(*) FROM messages WHERE compacted=1").fetchone()[0]
src.close()

can = sqlite3.connect(f"file:{CANON}?immutable=1", uri=True)
n_can_sessions = can.execute("SELECT count(*) FROM sessions").fetchone()[0]
n_can_turns = can.execute("SELECT count(*) FROM turns").fetchone()[0]
n_can_human = can.execute("SELECT count(*) FROM human_signals").fetchone()[0]
n_can_reason = can.execute("SELECT count(*) FROM reasoning").fetchone()[0]
n_can_states = can.execute("SELECT count(*) FROM states").fetchone()[0]
n_can_edges = can.execute("SELECT count(*) FROM edges").fetchone()[0]

def cmp(label, a, b):
    ok = "OK " if a == b else "DIFF"
    print(f"  [{ok}] {label:<24} source={a:>9,}  canonical={b:>9,}")

cmp("sessions", n_src_sessions, n_can_sessions)
cmp("messages -> turns", n_src_msgs, n_can_turns)
cmp("user msgs -> human", n_src_user, n_can_human)
cmp("reasoning_content", n_src_reason, n_can_reason)
cmp("compacted -> states", n_src_compact, n_can_states)
print(f"  [ADD] edges (was 0 in old pipeline)      canonical={n_can_edges:>9,}")

print()
print("=" * 78)
print("OLD SILVER vs NEW CANONICAL (hermes)")
print("=" * 78)
sil = sqlite3.connect(f"file:{SILVER}?immutable=1", uri=True)
try:
    s_turns = sil.execute("SELECT count(*) FROM dialogue_turns WHERE agent_family='hermes'").fetchone()[0]
    s_parents = sil.execute("SELECT count(*) FROM dialogue_turns WHERE agent_family='hermes' AND parent_turn_uid IS NOT NULL").fetchone()[0]
    s_reason = sil.execute("SELECT count(*) FROM reasoning_traces WHERE session_uid LIKE 'hermes:%'").fetchone()[0]
    s_compact = sil.execute("SELECT count(*) FROM context_compactions WHERE session_uid LIKE 'hermes:%'").fetchone()[0]
    print(f"  {'metric':<26}{'old silver':>13}{'new canonical':>15}")
    print(f"  {'turns':<26}{s_turns:>13,}{n_can_turns:>15,}")
    print(f"  {'edges (parent_turn_uid)':<26}{s_parents:>13,}{n_can_edges:>15,}")
    print(f"  {'reasoning':<26}{s_reason:>13,}{n_can_reason:>15,}")
    print(f"  {'compactions':<26}{s_compact:>13,}{n_can_states:>15,}")
    print(f"  {'human signal (tagged)':<26}{'0':>13}{n_can_human:>15,}")
except Exception as e:
    print("  silver query failed:", e)
sil.close()
can.close()
