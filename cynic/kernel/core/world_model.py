"""
CYNIC WorldModelUpdater " Unified cross-reality state aggregator.

Subscribes to JUDGMENT_CREATED, maintains a live WorldState snapshot
indexed by reality. Computes composite_risk (phi-weighted geo mean).
Detects conflicts (HOWL vs BARK in different realities).
"""

from __future__ import annotations

import logging
import math
import time
from dataclasses import dataclass, field
from typing import Any, Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.phi import phi_bound_score

logger = logging.getLogger("cynic.kernel.core.world_model")

# Realities in priority order for conflict detection
REALITIES = ["CODE", "CYNIC", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "COSMOS"]


@dataclass
class RealitySnapshot:
    reality: str
    verdict: str = "WAG"
    q_score: float = 50.0
    judgment_id: str = ""
    updated_at: float = field(default_factory=time.time)


@dataclass
class WorldState:
    snapshots: dict[str, RealitySnapshot] = field(default_factory=dict)
    conflicts: list[str] = field(default_factory=list)
    composite_risk: float = 50.0  # phi-weighted geometric mean [0, 100]
    dominant_reality: str = "CODE"  # reality with highest risk (lowest q_score)
    last_updated: float = field(default_factory=time.time)


class WorldModelUpdater:
    """
    Aggregates JUDGMENT_CREATED events into a live WorldState.

    Usage:
        updater = WorldModelUpdater()
        updater.start()  # subscribes to event bus
        state = updater.world_state()  # read current snapshot
    """

    def __init__(self, bus: Optional[EventBus] = None) -> None:
        self._state = WorldState()
        self._judgment_count = 0
        self._conflict_count = 0
        self._started = False
        self._bus = bus

    def start(self, bus=None) -> None:
        if self._started:
            return
        target = bus or self._bus
        if target is None:
             raise RuntimeError("WorldModelUpdater started without a bus instance")
        target.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment)
        self._started = True
        logger.info("WorldModelUpdater subscribed to JUDGMENT_CREATED")

    def stop(self) -> None:
        """Unregister from bus judgment events."""
        if self._started and self._bus:
            try:
                self._bus.off(CoreEvent.JUDGMENT_CREATED, self._on_judgment)
            except Exception as e:
                logger.debug(f"Error unregistering WorldModelUpdater listener: {e}")
            self._started = False
            logger.info("WorldModelUpdater stopped")

    async def _on_judgment(self, event: Event) -> None:
        try:
            p = event.dict_payload or {}
            reality = p.get("reality", "CODE")
            verdict = p.get("verdict", "WAG")
            q_score = float(p.get("q_score", 50.0))
            judgment_id = p.get("judgment_id", "")

            self._state.snapshots[reality] = RealitySnapshot(
                reality=reality,
                verdict=verdict,
                q_score=q_score,
                judgment_id=judgment_id,
            )
            self._judgment_count += 1
            self._recompute()
        except Exception as exc:
            logger.debug("WorldModelUpdater._on_judgment error: %s", exc)

    def _recompute(self) -> None:
        snaps = self._state.snapshots
        if not snaps:
            return

        # Composite risk = geometric mean of (100 - q_score) for each reality
        # High risk = high (100 - q_score)
        risks = {r: max(100.0 - s.q_score, 0.1) for r, s in snaps.items()}
        log_sum = sum(math.log(v) for v in risks.values())
        geo_mean_risk = math.exp(log_sum / len(risks))
        self._state.composite_risk = round(phi_bound_score(geo_mean_risk), 2)

        # Dominant reality = highest risk (lowest q_score)
        self._state.dominant_reality = min(snaps, key=lambda r: snaps[r].q_score)

        # Conflicts: HOWL vs BARK in different realities
        verdicts = {r: s.verdict for r, s in snaps.items()}
        howl_realities = [r for r, v in verdicts.items() if v == "HOWL"]
        bark_realities = [r for r, v in verdicts.items() if v == "BARK"]
        conflicts = []
        for h in howl_realities:
            for b in bark_realities:
                conflicts.append(f"{h}:HOWL vs {b}:BARK")
        self._state.conflicts = conflicts
        if conflicts:
            self._conflict_count += 1

        self._state.last_updated = time.time()

    def world_state(self) -> WorldState:
        return self._state

    def snapshot(self) -> dict[str, Any]:
        s = self._state
        return {
            "composite_risk": s.composite_risk,
            "dominant_reality": s.dominant_reality,
            "conflicts": s.conflicts,
            "realities": {
                r: {"verdict": snap.verdict, "q_score": snap.q_score}
                for r, snap in s.snapshots.items()
            },
            "judgment_count": self._judgment_count,
            "conflict_count": self._conflict_count,
            "last_updated": round(s.last_updated, 3),
        }

    def stats(self) -> dict[str, Any]:
        return {
            "judgment_count": self._judgment_count,
            "conflict_count": self._conflict_count,
            "realities_tracked": len(self._state.snapshots),
            "composite_risk": self._state.composite_risk,
            "dominant_reality": self._state.dominant_reality,
        }
