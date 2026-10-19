#!/usr/bin/env python3
"""Capture an OMP source into an immutable, content-addressed snapshot.

Usage:
    python capture_omp.py [--source DIR] [--dest DIR]

Writes:
    <dest>/omp.<sha256[:16]>            — the frozen tree copy
    <dest>/manifest.json                — the capture manifest

The frozen copy is then the ONLY source the canonical layer may read.
Live ``~/.omp/agent/sessions`` is for tests and probes, never for ingestion.
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

DEFAULT_SOURCE = "/home/user/.omp/agent/sessions"
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
        sources={"omp": args.source},
        dest_dir=args.dest,
        capturers=[DirectorySourceCapturer()],
    )
    rec = manifest.by_harness("omp")
    assert rec is not None

    print(f"[capture] captured: {rec.captured_path}")
    print(f"[capture]   hash:   {rec.content_hash}")
    print(f"[capture]   size:   {rec.size_bytes:,} bytes")
    print(f"[capture]   method: {rec.method.value}")
    print(f"[capture]   entries: {rec.entry_count:,}")

    # Prove the frozen copy holds the same sessions: count files + records.
    files = sorted(glob.glob(os.path.join(rec.captured_path, "**/*.jsonl"), recursive=True))
    n_records = 0
    for f in files:
        with open(f, errors="replace") as fh:
            for line in fh:
                if line.strip():
                    n_records += 1
    print(f"[capture]   sessions: {len(files):,}")
    print(f"[capture]   records:  {n_records:,}")
    if not files or not n_records:
        print("[capture] FATAL: frozen copy holds no sessions", file=sys.stderr)
        return 3

    FileManifestWriter().write(manifest, os.path.join(args.dest, "manifest.json"))
    print(f"[capture] manifest: {os.path.join(args.dest, 'manifest.json')}")
    print(f"[capture] MANIFEST_JSON " + json.dumps({
        "manifest_uid": manifest.manifest_uid,
        "capture_uid": rec.capture_uid,
        "content_hash": rec.content_hash,
        "size_bytes": rec.size_bytes,
        "sessions": len(files),
        "records": n_records,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
