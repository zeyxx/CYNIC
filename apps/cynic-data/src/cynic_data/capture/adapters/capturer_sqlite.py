"""SQLite source capturer.

A live SQLite database with an active WAL cannot be copied with ``cp`` and remain
consistent. ``VACUUM INTO`` (SQLite >= 3.27) writes a transactionally-consistent
snapshot into a new file, even while writers continue. That file is the capture.

This is the fix for the defect: never read the live DB, read the frozen copy.
"""

from __future__ import annotations

import datetime
import os
import sqlite3

from ...kernel.ids import content_hash, short_hash
from ..domain.records import CaptureMethod, CaptureRecord
from ._hash import hash_file


class SqliteSourceCapturer:
    source_kind = "sqlite"

    def can_capture(self, source_path: str) -> bool:
        if not os.path.isfile(source_path):
            return False
        try:
            with open(source_path, "rb") as f:
                return f.read(16) == b"SQLite format 3\x00"
        except OSError:
            return False

    def capture(self, harness: str, source_path: str, dest_dir: str) -> CaptureRecord:
        os.makedirs(dest_dir, exist_ok=True)
        stamp = datetime.datetime.now().strftime("%Y%m%dT%H%M%S")
        tmp = os.path.join(dest_dir, f".{harness}.{stamp}.tmp")

        # Consistent snapshot of a possibly-live database.
        con = sqlite3.connect(source_path)
        try:
            con.execute(f"VACUUM INTO '{tmp}'")
        finally:
            con.close()

        digest = hash_file(tmp)
        final = os.path.join(dest_dir, f"{harness}.{digest.split(':')[1][:16]}.sqlite")
        os.replace(tmp, final)

        # Provenance: how many sessions the snapshot carries (if it has messages).
        entry_count = None
        try:
            c = sqlite3.connect(f"file:{final}?immutable=1", uri=True)
            try:
                entry_count = c.execute("SELECT count(*) FROM messages").fetchone()[0]
            finally:
                c.close()
        except sqlite3.Error:
            entry_count = None

        return CaptureRecord(
            capture_uid=f"{harness}:{short_hash(digest, 16)}",
            harness=harness,
            source_kind=self.source_kind,
            source_path=source_path,
            captured_path=final,
            content_hash=digest,
            size_bytes=os.path.getsize(final),
            captured_at_iso=datetime.datetime.now(datetime.timezone.utc).isoformat(),
            method=CaptureMethod.SQLITE_VACUUM_INTO,
            entry_count=entry_count,
            native_meta={"source_size_bytes": os.path.getsize(source_path)},
        )
