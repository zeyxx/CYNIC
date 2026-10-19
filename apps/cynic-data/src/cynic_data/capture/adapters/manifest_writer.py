"""File manifest writer — the I/O adapter for capture manifests.

The use case stays pure; this adapter owns the filesystem write.
"""

from __future__ import annotations

import json
import os

from ..domain.records import CaptureManifest
from ..ports import ManifestWriterPort


class FileManifestWriter:
    """Write a CaptureManifest to a JSON file on disk."""

    def write(self, manifest: CaptureManifest, path: str) -> None:
        payload = {
            "schema": "cynic.capture.manifest.v1",
            "manifest_uid": manifest.manifest_uid,
            "created_at_iso": manifest.created_at_iso,
            "records": [
                {
                    "capture_uid": r.capture_uid,
                    "harness": r.harness,
                    "source_kind": r.source_kind,
                    "source_path": r.source_path,
                    "captured_path": r.captured_path,
                    "content_hash": r.content_hash,
                    "size_bytes": r.size_bytes,
                    "captured_at_iso": r.captured_at_iso,
                    "method": r.method.value,
                    "entry_count": r.entry_count,
                    "native_meta": r.native_meta,
                }
                for r in manifest.records
            ],
        }
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            json.dump(payload, f, indent=2, sort_keys=True)
