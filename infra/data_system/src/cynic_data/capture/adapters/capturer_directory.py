"""Directory source capturer.

Copies a tree of files into a frozen capture, then hashes a deterministic manifest
of (relative path, size, sha256) so the capture is content-addressed as a whole.
Skips symlinks (a capture must never re-introduce a link to live data).
"""

from __future__ import annotations

import datetime
import os
import shutil
import tempfile

from ...kernel.ids import short_hash
from ..domain.records import CaptureMethod, CaptureRecord
from ._hash import hash_tree


class DirectorySourceCapturer:
    source_kind = "directory"

    def can_capture(self, source_path: str) -> bool:
        return os.path.isdir(source_path)

    def capture(self, harness: str, source_path: str, dest_dir: str) -> CaptureRecord:
        os.makedirs(dest_dir, exist_ok=True)
        # staging MUST live outside dest_dir, otherwise shutil.move() sees the
        # final path as a subdirectory of staging and refuses.
        staging = tempfile.mkdtemp(prefix=f".cynic-cap-{harness}-")
        # Copy manually, skipping symlinks: a capture must never re-introduce a
        # link to live data. shutil.copytree(symlinks=False) would copy the
        # TARGET's bytes, silently turning a link into a real file.
        for dirpath, dirnames, filenames in os.walk(source_path):
            rel_dir = os.path.relpath(dirpath, source_path)
            target_dir = staging if rel_dir == "." else os.path.join(staging, rel_dir)
            os.makedirs(target_dir, exist_ok=True)
            # prune symlinked directories so os.walk does not descend into them
            dirnames[:] = [d for d in dirnames if not os.path.islink(os.path.join(dirpath, d))]
            for name in filenames:
                src_file = os.path.join(dirpath, name)
                if os.path.islink(src_file):
                    continue
                shutil.copy2(src_file, os.path.join(target_dir, name))

        digest, count, total = hash_tree(staging)
        final = os.path.join(dest_dir, f"{harness}.{digest.split(':')[1][:16]}")
        if os.path.exists(final):
            shutil.rmtree(final)
        shutil.move(staging, final)

        return CaptureRecord(
            capture_uid=f"{harness}:{short_hash(digest, 16)}",
            harness=harness,
            source_kind=self.source_kind,
            source_path=source_path,
            captured_path=final,
            content_hash=digest,
            size_bytes=total,
            captured_at_iso=datetime.datetime.now(datetime.timezone.utc).isoformat(),
            method=CaptureMethod.DIRECTORY_COPY,
            entry_count=count,
            native_meta={"tree_manifest_digest": digest},
        )
