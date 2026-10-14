"""Shared tree hashing — used by both the directory capturer and the
windows_gemini tool-outputs manifest. Single source of truth for the
content-addressed tree digest.
"""

from __future__ import annotations

import hashlib
import json
import os


def hash_file(path: str, chunk: int = 1 << 20) -> str:
    """Return the canonical sha256 digest of a file, prefixed with its algorithm."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            b = f.read(chunk)
            if not b:
                break
            h.update(b)
    return "sha256:" + h.hexdigest()


def hash_tree(root: str) -> tuple[str, int, int]:
    """Return (manifest_digest, file_count, total_bytes) for a directory tree.

    Skips symlinks. Deterministic: sorted entries, canonical JSON.
    """
    entries = []
    total = 0
    for dirpath, _dirnames, filenames in os.walk(root):
        for name in sorted(filenames):
            full = os.path.join(dirpath, name)
            if os.path.islink(full):
                continue
            rel = os.path.relpath(full, root)
            size = os.path.getsize(full)
            h = hashlib.sha256()
            with open(full, "rb") as f:
                for chunk in iter(lambda: f.read(1 << 20), b""):
                    h.update(chunk)
            entries.append({"path": rel, "size": size, "sha256": h.hexdigest()})
            total += size
    blob = json.dumps(entries, sort_keys=True, separators=(",", ":")).encode()
    return "sha256:" + hashlib.sha256(blob).hexdigest(), len(entries), total
