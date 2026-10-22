#!/usr/bin/env python3
"""Capture an Antigravity source into an immutable, content-addressed snapshot.

Usage:
    python capture_antigravity.py [--source DIR] [--dest DIR]

Writes:
    <dest>/antigravity.<sha256[:16]>   — the frozen tree copy
    <dest>/manifest.json              — the capture manifest

The frozen copy is then the ONLY source the canonical layer may read.
"""
from __future__ import annotations

import argparse
import glob
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from cynic_data.capture.adapters.capturer_directory import DirectorySourceCapturer
from cynic_data.capture.usecases.capture_source import capture_sources
from cynic_data.capture.adapters.manifest_writer import FileManifestWriter

DEFAULT_SOURCE = "/var/lib/cynic/vault/archive/linux_sessions_2026/gemini_antigravity/conversations"
DEFAULT_DEST = "/var/lib/cynic/vault/corpus/captures"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", default=DEFAULT_SOURCE)
    ap.add_argument("--dest", default=DEFAULT_DEST)
    args = ap.parse_args()

    if not os.path.isdir(args.source):
        print(f"[capture] FATAL: source not found: {args.source}", file=sys.stderr)
        return 2

    print(f"[capture] source: {args.source}")
    print(f"[capture] dest:   {args.dest}")

    manifest = capture_sources(
        sources={"antigravity": args.source},
        dest_dir=args.dest,
        capturers=[DirectorySourceCapturer()],
    )
    rec = manifest.by_harness("antigravity")
    assert rec is not None

    print(f"[capture] captured: {rec.captured_path}")
    print(f"[capture]   hash:   {rec.content_hash}")
    print(f"[capture]   size:   {rec.size_bytes:,} bytes")
    print(f"[capture]   method: {rec.method.value}")
    print(f"[capture]   entries: {rec.entry_count:,}")

    dbs = sorted(glob.glob(os.path.join(rec.captured_path, "*.db")))
    print(f"[capture]   dbs:      {len(dbs):,}")
    if not dbs:
        print("[capture] FATAL: frozen copy holds no DBs", file=sys.stderr)
        return 3

    FileManifestWriter().write(manifest, os.path.join(args.dest, "manifest.json"))
    print(f"[capture] manifest: {os.path.join(args.dest, 'manifest.json')}")
    print("[capture] MANIFEST_JSON " + json.dumps({
        "manifest_uid": manifest.manifest_uid,
        "capture_uid": rec.capture_uid,
        "content_hash": rec.content_hash,
        "size_bytes": rec.size_bytes,
        "dbs": len(dbs),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
