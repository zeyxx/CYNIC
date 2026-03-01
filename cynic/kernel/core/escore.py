"""
CYNIC E-Score 7D — Reputation Tracker (γ4)

E-Score = Entity Reputation Score across 7 contribution dimensions.
Purely memory-based for performance, with async background persistence.
"""

from __future__ import annotations

import asyncio
import logging
import math
import time
from dataclasses import dataclass, field
from typing import Any, Optional

from cynic.kernel.core.phi import (
    PHI,
    PHI_2,
    PHI_3,
    PHI_INV,
    PHI_INV_2,
    PHI_INV_3,
    phi_bound_score,
)
from cynic.kernel.core.event_bus import (
    EventBus,
    CoreEvent,
    Event,
)
from cynic.kernel.core.events_schema import ReputationSyncPayload

logger = logging.getLogger("cynic.kernel.core.escore")

E_SCORE_WEIGHTS = {
    "BURN": PHI_3, "BUILD": PHI_2, "JUDGE": PHI, "RUN": 1.0,
    "SOCIAL": PHI_INV, "GRAPH": PHI_INV_2, "HOLD": PHI_INV_3,
}
E_SCORE_TOTAL_WEIGHT = sum(E_SCORE_WEIGHTS.values())

@dataclass
class EScoreProfile:
    entity_id: str
    overall_score: float = 50.0
    dimensions: dict[str, float] = field(default_factory=lambda: {k: 50.0 for k in E_SCORE_WEIGHTS})
    reality_scores: dict[str, float] = field(default_factory=dict)
    last_updated: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return {
            "entity_id": self.entity_id,
            "overall_score": round(self.overall_score, 2),
            "dimensions": {k: round(v, 2) for k, v in self.dimensions.items()},
            "reality_scores": {k: round(v, 2) for k, v in self.reality_scores.items()},
            "last_updated": self.last_updated,
        }

class EScoreTracker:
    def __init__(self, bus: EventBus, instance_id: str, state_manager: Any | None = None):
        self.bus = bus
        self.state = state_manager
        self.instance_id = instance_id
        self._profiles: dict[str, EScoreProfile] = {}

    def get_profile(self, entity_id: str) -> EScoreProfile:
        """Get profile from memory cache. Non-blocking."""
        if entity_id not in self._profiles:
            self._profiles[entity_id] = EScoreProfile(entity_id=entity_id)
        return self._profiles[entity_id]

    def update_dimension(self, entity_id: str, dimension: str, value: float, weight: float = 1.0, **kwargs: Any) -> float:
        if dimension not in E_SCORE_WEIGHTS:
            return 0.0

        profile = self.get_profile(entity_id)
        reality = kwargs.get("reality")
        if reality:
            profile.reality_scores[reality] = value

        current = profile.dimensions.get(dimension, 50.0)
        alpha = PHI_INV_2 * weight
        new_val = (alpha * value) + (1.0 - alpha) * current
        profile.dimensions[dimension] = phi_bound_score(new_val)
        profile.overall_score = self._calculate_aggregate(profile.dimensions)
        profile.last_updated = time.time()

        if self.state:
            asyncio.create_task(self.state.update(f"escore:profile:{entity_id}", profile.to_dict()))
        
        return profile.overall_score

    def _calculate_aggregate(self, dimensions: dict[str, float]) -> float:
        log_sum = sum(E_SCORE_WEIGHTS[d] * math.log(max(v, 0.1)) for d, v in dimensions.items())
        return phi_bound_score(math.exp(log_sum / E_SCORE_TOTAL_WEIGHT))

    async def broadcast_reputation(self) -> None:
        for profile in self._profiles.values():
            await self.bus.emit(Event.typed(
                CoreEvent.REPUTATION_SYNC,
                ReputationSyncPayload(
                    entity_id=profile.entity_id,
                    overall_score=profile.overall_score,
                    dimensions=profile.dimensions,
                    reality_scores=profile.reality_scores,
                    last_updated=profile.last_updated
                ),
                source="escore_tracker"
            ))

    def stats(self) -> dict:
        return {"entities": len(self._profiles)}
