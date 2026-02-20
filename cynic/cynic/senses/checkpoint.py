"""
CYNIC SessionCheckpointer — Cross-crash context persistence (γ2)

Saves ContextCompressor state to ~/.cynic/session-latest.json so the session
context survives process crashes and disk-pressure events.

Strategy:
  - Atomic write: JSON → .tmp file → os.replace() (crash-safe)
  - Checkpoint every CHECKPOINT_EVERY (F(8)=21) judgments
  - Skip restore if checkpoint is older than MAX_AGE_H hours (stale)
  - Skip save if disk is critically full (avoid making things worse)

φ-derived constants:
  CHECKPOINT_EVERY = F(8) = 21 judgments
  MAX_AGE_H        = 24 hours (one full day)
"""
from __future__ import annotations

import json
import logging
import os
import time
from typing import TYPE_CHECKING

from cynic.core.phi import fibonacci

if TYPE_CHECKING:
    from cynic.senses.compressor import ContextCompressor

logger = logging.getLogger("cynic.perceive.checkpoint")

# Save every F(8)=21 judgments
CHECKPOINT_EVERY: int = fibonacci(8)  # 21

# Discard checkpoints older than this (stale session)
MAX_AGE_H: float = 24.0

# Checkpoint file location (follows ~/.cynic/ convention)
_CHECKPOINT_PATH = os.path.join(
    os.path.expanduser("~"), ".cynic", "session-latest.json"
)


def save(compressor: ContextCompressor) -> bool:
    """
    Serialize ContextCompressor state to ~/.cynic/session-latest.json.

    Uses atomic write (write to .tmp then os.replace) so a crash during
    save never corrupts the existing checkpoint.

    Returns True on success, False if save was skipped or failed.
    """
    try:
        os.makedirs(os.path.dirname(_CHECKPOINT_PATH), exist_ok=True)

        # Abort if disk critically full (avoid writing to a full disk)
        import shutil
        usage = shutil.disk_usage(os.path.dirname(_CHECKPOINT_PATH))
        disk_free_pct = usage.free / usage.total
        if disk_free_pct < 0.05:
            logger.warning(
                "SessionCheckpoint: disk %.1f%% free — skipping save",
                disk_free_pct * 100,
            )
            return False

        data = compressor.to_dict()
        data["saved_at"] = time.time()

        tmp_path = _CHECKPOINT_PATH + ".tmp"
        with open(tmp_path, "w", encoding="utf-8") as fh:
            json.dump(data, fh, ensure_ascii=False)

        os.replace(tmp_path, _CHECKPOINT_PATH)  # atomic on POSIX and Windows

        logger.debug(
            "SessionCheckpoint: saved %d chunks to %s",
            len(data["chunks"]), _CHECKPOINT_PATH,
        )
        return True

    except Exception as exc:
        logger.warning("SessionCheckpoint save failed: %s", exc)
        return False


def restore(compressor: ContextCompressor) -> int:
    """
    Restore ContextCompressor state from the last checkpoint.

    Skips silently if:
      - No checkpoint file exists (first run)
      - Checkpoint is older than MAX_AGE_H (stale session)
      - File is corrupted (JSON error)

    Returns number of chunks restored (0 on skip or error).
    """
    if not os.path.exists(_CHECKPOINT_PATH):
        logger.debug("SessionCheckpoint: no checkpoint found at %s", _CHECKPOINT_PATH)
        return 0

    try:
        with open(_CHECKPOINT_PATH, encoding="utf-8") as fh:
            data = json.load(fh)

        saved_at = float(data.get("saved_at", 0))
        age_h = (time.time() - saved_at) / 3600.0

        if age_h > MAX_AGE_H:
            logger.info(
                "SessionCheckpoint: skipping stale checkpoint (%.1fh > %.0fh limit)",
                age_h, MAX_AGE_H,
            )
            return 0

        n = compressor.restore_from_dict(data)
        logger.info(
            "SessionCheckpoint: restored %d chunks (checkpoint %.1fh old)",
            n, age_h,
        )
        return n

    except Exception as exc:
        logger.warning("SessionCheckpoint restore failed: %s", exc)
        return 0
