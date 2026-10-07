#!/usr/bin/env python3
"""Capture a hermes source into an immutable, content-addressed snapshot.

Usage:
    python capture_hermes.py [--dest DIR]

Writes:
    <dest>/hermes.<sha256[:16]>.sqlite   — the frozen copy
    <dest>/manifest.json                  — the capture manifest

The frozen copy is then the ONLY source the canonical layer may read.
"""
from __future__ import annotations

import argparse
import json
import os
import sqlite3
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from cynic_data.capture.adapters.capturer_sqlite import SqliteSourceCapturer
from cynic_data.capture.usecases.capture_source import capture_sources, write_manifest

DEFAULT_SOURCE = "/home/user/.hermes/state.db"
DEFAULT_DEST = "/var/lib/cynic/vault/corpus/captures"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", default=DEFAULT_SOURCE)
    ap.add_argument("--dest", default=DEFAULT_DEST)
    args = ap.parse_args()

    if not os.path.exists(args.source):
        print(f"[capture] FATAL: source not found: {args.source}", file=sys.stderr)
        return 2

    print(f"[capture] source: {args.source}")
    print(f"[capture] dest:   {args.dest}")

    manifest = capture_sources(
        sources={"hermes": args.source},
        dest_dir=args.dest,
        capturers=[SqliteSourceCapturer()],
    )
    rec = manifest.by_harness("hermes")
    assert rec is not None

    print(f"[capture] captured: {rec.captured_path}")
    print(f"[capture]   hash:   {rec.content_hash}")
    print(f"[capture]   size:   {rec.size_bytes:,} bytes")
    print(f"[capture]   method: {rec.method.value}")
    print(f"[capture]   entries: {rec.entry_count:,}")

    # Prove the frozen copy is consistent and readable.
    con = sqlite3.connect(f"file:{rec.captured_path}?immutable=1", uri=True)
    try:
        integrity = con.execute("PRAGMA integrity_check").fetchone()[0]
        n_sessions = con.execute("SELECT count(DISTINCT session_id) FROM messages").fetchone()[0]
        n_msgs = con.execute("SELECT count(*) FROM messages").fetchone()[0]
    finally:
        con.close()
    print(f"[capture]   integrity: {integrity}")
    print(f"[capture]   sessions:  {n_sessions:,}")
    print(f"[capture]   messages:  {n_msgs:,}")
    if integrity != "ok":
        print("[capture] FATAL: snapshot is not consistent", file=sys.stderr)
        return 3

    write_manifest(manifest, os.path.join(args.dest, "manifest.json"))
    print(f"[capture] manifest: {os.path.join(args.dest, 'manifest.json')}")
    print(f"[capture] MANIFEST_JSON " + json.dumps({
        "manifest_uid": manifest.manifest_uid,
        "capture_uid": rec.capture_uid,
        "content_hash": rec.content_hash,
        "size_bytes": rec.size_bytes,
        "sessions": n_sessions,
        "messages": n_msgs,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
