"""CYNIC SocialWatcher — SOCIAL×PERCEIVE/MICRO every F(11)=89s."""
from __future__ import annotations

import asyncio
import json
import os
import time
from typing import Any, Dict, Optional

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci
from cynic.perceive.workers.base import PerceiveWorker

_SOCIAL_SIGNAL_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "social.json")


class SocialWatcher(PerceiveWorker):
    """
    Monitors social signals from the ~/.cynic/social.json feed.

    JS hooks (observe.js) or external scripts write social signals here.
    SocialWatcher reads them and submits SOCIAL×PERCEIVE at MICRO level.

    signal.json schema:
      {"ts": 1234567890, "source": "twitter|discord|reddit", "sentiment": -1.0..1.0,
       "volume": 0..100, "topic": "...", "signal_type": "...", "read": false}

    Only processes signals where read=false (marks them as read after submission).

    interval: F(11)=89s — social signals are slow-moving.
    API-key-free: reads a local file written by any JS/Python hook.
    """

    level = ConsciousnessLevel.MICRO
    interval_s = float(fibonacci(11))  # 89.0s
    name = "social_watcher"

    def __init__(self, signal_path: Optional[str] = None) -> None:
        self._path = signal_path or _SOCIAL_SIGNAL_PATH
        self._last_ts: float = 0.0

    def _read_signal(self) -> Optional[Dict[str, Any]]:
        """Blocking read — called via run_in_executor."""
        try:
            if not os.path.exists(self._path):
                return None
            with open(self._path, encoding="utf-8") as fh:
                data = json.load(fh)
            # Accept both single signal and list; take first unread
            signals = data if isinstance(data, list) else [data]
            for sig in signals:
                if not sig.get("read", False) and sig.get("ts", 0) > self._last_ts:
                    return sig
            return None
        except Exception:
            return None

    def _mark_read(self, ts: float) -> None:
        """Mark the signal as read so we don't re-submit it."""
        try:
            if not os.path.exists(self._path):
                return
            with open(self._path, encoding="utf-8") as fh:
                data = json.load(fh)
            signals = data if isinstance(data, list) else [data]
            for sig in signals:
                if sig.get("ts") == ts:
                    sig["read"] = True
            with open(self._path, "w", encoding="utf-8") as fh:
                json.dump(signals if isinstance(data, list) else signals[0], fh)
        except Exception:
            pass

    async def sense(self) -> Optional[Cell]:
        loop = asyncio.get_running_loop()
        sig = await loop.run_in_executor(None, self._read_signal)
        if sig is None:
            return None

        ts = float(sig.get("ts", time.time()))
        self._last_ts = ts
        await loop.run_in_executor(None, self._mark_read, ts)

        sentiment = float(sig.get("sentiment", 0.0))
        volume = float(sig.get("volume", 0.0))
        source = sig.get("source", "unknown")

        # Positive sentiment → low risk; negative → higher risk
        risk = max(0.0, min(1.0, (0.5 - sentiment * 0.5)))

        return Cell(
            reality="SOCIAL",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "source": source,
                "sentiment": round(sentiment, 3),
                "volume": round(volume, 1),
                "topic": sig.get("topic", ""),
                "signal_type": sig.get("signal_type", "mention"),
            },
            context=(
                f"Social watcher: {source} sentiment={sentiment:+.2f} "
                f"volume={volume:.0f} topic={sig.get('topic', '')}"
            ),
            risk=risk,
            complexity=0.3,
            budget_usd=0.002,
            metadata={"source": "social_watcher", "social_source": source, "ts": ts},
        )
