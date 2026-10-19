"""Content-addressed blob store for large payloads.

SQLite stores TEXT without a practical limit, but the Python sqlite3 binding
refuses parameters larger than INT_MAX (2^31-1) bytes. Rather than truncate
(a semantic loss), large payloads are written to content-addressed files and
the database stores only the hash. Nothing is lost; the content is retrievable
by its hash.
"""

from __future__ import annotations

import hashlib
import os

from ...kernel.ids import content_hash

# Payloads up to this size stay in the database. Above it, they go to the blob
# store. 1 GiB leaves a wide margin under the 2 GiB binding limit.
INLINE_LIMIT = 1 << 30


def _blob_path(root: str, digest: str) -> str:
    """Shard by the first two hex chars so no directory holds too many files."""
    hexpart = digest.split(":", 1)[1]
    return os.path.join(root, "blobs", hexpart[:2], hexpart[2:])


class ContentBlobStore:
    def __init__(self, root: str) -> None:
        self.root = root
        os.makedirs(os.path.join(root, "blobs"), exist_ok=True)

    def put(self, data: str) -> tuple[str, bool]:
        """Store a payload. Returns (content_hash, inlined).

        ``inlined`` is True when the payload is small enough to keep in the
        database; False when it was written to the blob store.
        """
        raw = data.encode("utf-8")
        digest = content_hash(raw)
        if len(raw) <= INLINE_LIMIT:
            return digest, True
        path = _blob_path(self.root, digest)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        if not os.path.exists(path):
            with open(path, "wb") as f:
                f.write(raw)
        return digest, False

    def get(self, digest: str) -> str | None:
        """Retrieve a payload by hash. Returns None if absent."""
        path = _blob_path(self.root, digest)
        if not os.path.exists(path):
            return None
        with open(path, "rb") as f:
            return f.read().decode("utf-8", errors="replace")
