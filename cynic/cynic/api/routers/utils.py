"""
Shared helpers used across multiple CYNIC API routers.
"""
from __future__ import annotations

import json
import logging
import os

logger = logging.getLogger("cynic.api.server")

# Path for social signals — SocialWatcher reads; human interactions write.
_SOCIAL_SIGNAL_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "social.json")
# Rolling cap: F(8)=21 signals max (prevent unbounded growth)
_SOCIAL_SIGNAL_CAP = 21


def _append_social_signal(
    source: str,
    sentiment: float,
    volume: float,
    topic: str,
    signal_type: str,
) -> None:
    """
    Append one social signal to ~/.cynic/social.json (fire-and-forget).

    SocialWatcher reads this file every 89s and submits SOCIAL×PERCEIVE
    cells. The read=False flag ensures each signal is processed exactly once.

    Closes the Social loop: human interactions → sentiment → SocialWatcher →
    MICRO judgment → QTable + SYMBIOSIS axiom signal.
    """
    try:
        import time as _t
        os.makedirs(os.path.dirname(_SOCIAL_SIGNAL_PATH), exist_ok=True)
        signal = {
            "ts": _t.time(),
            "source": source,
            "sentiment": round(max(-1.0, min(1.0, sentiment)), 3),
            "volume": round(max(0.0, min(100.0, volume)), 1),
            "topic": topic,
            "signal_type": signal_type,
            "read": False,
        }
        if os.path.exists(_SOCIAL_SIGNAL_PATH):
            with open(_SOCIAL_SIGNAL_PATH, encoding="utf-8") as fh:
                existing = json.load(fh)
            if not isinstance(existing, list):
                existing = [existing]
        else:
            existing = []
        existing.append(signal)
        if len(existing) > _SOCIAL_SIGNAL_CAP:
            existing = existing[-_SOCIAL_SIGNAL_CAP:]
        with open(_SOCIAL_SIGNAL_PATH, "w", encoding="utf-8") as fh:
            json.dump(existing, fh)
    except json.JSONDecodeError as exc:
        logger.debug("social.json append skipped: %s", exc)
