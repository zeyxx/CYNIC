"""ProfileCheckpoint — crash-safe persistence for mirror agent state.

Tier 2 INFRASTRUCTURE: Checkpoint serialization for the mirror daemon.

K15 Consumer: mirror coordinator reads this on boot to resume from cursor.
Failure mode: missing/corrupt file → fresh checkpoint returned (safe default).
Atomic write: tmp+rename guarantees no partial files on crash.
"""
from __future__ import annotations

import json
import logging
import os
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from organs.mirror.profile import BehavioralProfile

__version__ = "0.1.0"

logger = logging.getLogger(__name__)

MAX_ASKESIS_PER_DAY: int = 3


@dataclass
class ProfileCheckpoint:
    """Persistent state for the mirror agent across restarts.

    Input contract: profile is a valid BehavioralProfile; cursors maps
    source name → last processed offset; askesis_date is ISO date string.
    Output guarantee: save/load round-trips all fields without loss.
    Failure modes: corrupt file on load → fresh checkpoint (logs warning).
    Valid domains: mirror daemon restart recovery.
    """

    profile: BehavioralProfile
    cursors: dict[str, int] = field(default_factory=dict)
    askesis_sent_today: int = 0
    askesis_date: str = ""

    # ------------------------------------------------------------------ #
    # Persistence                                                          #
    # ------------------------------------------------------------------ #

    def save(self, path: Path) -> None:
        """Atomically write checkpoint to path via tmp+rename.

        Guarantees no partial file exists on crash mid-write.
        """
        data: dict[str, Any] = {
            "version": __version__,
            "profile": json.loads(self.profile.to_json()),
            "cursors": self.cursors,
            "askesis_sent_today": self.askesis_sent_today,
            "askesis_date": self.askesis_date,
        }
        tmp = path.with_suffix(".tmp")
        tmp.write_text(json.dumps(data, indent=2), encoding="utf-8")
        # Atomic rename: on POSIX this is guaranteed atomic within same FS.
        os.replace(tmp, path)

    @classmethod
    def load(cls, path: Path) -> ProfileCheckpoint:
        """Load checkpoint from path.

        Returns a fresh checkpoint if the file is missing or corrupt.
        Logs a warning on corruption so silent data loss is visible (P7).
        """
        if not path.exists():
            return cls(profile=BehavioralProfile.empty())
        try:
            data: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
            profile = BehavioralProfile.from_json(
                json.dumps(data.get("profile", {}))
            )
            return cls(
                profile=profile,
                cursors=data.get("cursors", {}),
                askesis_sent_today=int(data.get("askesis_sent_today", 0)),
                askesis_date=str(data.get("askesis_date", "")),
            )
        except Exception:  # noqa: BLE001
            logger.warning(
                "Corrupt checkpoint at %s — starting fresh", path, exc_info=True
            )
            return cls(profile=BehavioralProfile.empty())

    # ------------------------------------------------------------------ #
    # Askesis throttle                                                     #
    # ------------------------------------------------------------------ #

    def can_send_askesis(self, today: str) -> bool:
        """True if another askesis message is permitted today.

        A new calendar day resets the counter unconditionally.
        """
        if self.askesis_date != today:
            return True
        return self.askesis_sent_today < MAX_ASKESIS_PER_DAY

    def record_askesis_sent(self, today: str) -> None:
        """Increment counter, resetting if the date has rolled over."""
        if self.askesis_date != today:
            self.askesis_sent_today = 0
            self.askesis_date = today
        self.askesis_sent_today += 1
