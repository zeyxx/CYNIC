"""
CYNIC E-Score 7D — Reputation Tracker (γ4)

E-Score = Entity Reputation Score across 7 contribution dimensions.

7 Dimensions (φ-weighted, descending priority):
  BURN   (φ³=4.236) — Irreversible token burn (commitment signal)
  BUILD  (φ²=2.618) — Code/artifact quality contributions
  JUDGE  (φ¹=1.618) — Judgment accuracy (prediction vs reality)
  RUN    (φ⁰=1.000) — Execution reliability
  SOCIAL (φ⁻¹=0.618) — Community engagement quality
  GRAPH  (φ⁻²=0.382) — Network connectivity (trust graph)
  HOLD   (φ⁻³=0.236) — Long-term commitment

Formula (same geometric mean as phi_aggregate):
    e_raw = exp(Σ w_i × log(max(d_i, 0.1)) / Σ w_i)
    e_score = phi_bound_score(e_raw)  # clamp to [0, MAX_Q_SCORE=100]

Per-Reality sub-scores:
  Each entity also tracks how E-Score evolves per reality dimension (CODE, SOLANA).
"""

from __future__ import annotations

import asyncio
import logging
import math
import time
from dataclasses import dataclass, field
from typing import Any

import httpx

from cynic.kernel.core.phi import (
    PHI,
    PHI_2,
    PHI_3,
    PHI_INV,
    PHI_INV_2,
    PHI_INV_3,
    phi_bound_score,
)

logger = logging.getLogger("cynic.kernel.core.escore")

# Weights derived from PHI powers
E_SCORE_WEIGHTS = {
    "BURN": PHI_3,  # 4.236
    "BUILD": PHI_2,  # 2.618
    "JUDGE": PHI,  # 1.618
    "RUN": 1.0,  # 1.000
    "SOCIAL": PHI_INV,  # 0.618
    "GRAPH": PHI_INV_2,  # 0.382
    "HOLD": PHI_INV_3,  # 0.236
}

E_SCORE_TOTAL_WEIGHT = sum(E_SCORE_WEIGHTS.values())


@dataclass
class EScoreProfile:
    """Reputation snapshot for a single entity (Human, Dog, or Service)."""

    entity_id: str
    overall_score: float = 50.0
    # Current values for the 7 dimensions [0, 100]
    dimensions: dict[str, float] = field(default_factory=lambda: {k: 50.0 for k in E_SCORE_WEIGHTS})
    # Reality-specific scores (e.g. "CODE": 72.5)
    reality_scores: dict[str, float] = field(default_factory=dict)
    # Metadata
    last_updated: float = field(default_factory=time.time)
    version: int = 1

    def to_dict(self) -> dict:
        return {
            "entity_id": self.entity_id,
            "overall_score": round(self.overall_score, 2),
            "dimensions": {k: round(v, 2) for k, v in self.dimensions.items()},
            "reality_scores": {k: round(v, 2) for k, v in self.reality_scores.items()},
            "last_updated": self.last_updated,
        }


class EScoreTracker:
    """
    Manages reputation tracking across the CYNIC ecosystem.

    Persistence is handled via StateManager (StateLayer.PERSISTENT).
    """

    def __init__(self, state_manager: Any | None = None):
        self.state = state_manager
        self._profiles: dict[str, EScoreProfile] = {}
        self._external_sync_url: str | None = None  # For k-NET reputation sharing

    def get_profile(self, entity_id: str) -> EScoreProfile:
        """Get or create reputation profile for an entity (Synchronous)."""
        if entity_id not in self._profiles:
            # Try to load from state if available
            if self.state:
                # Use query_sync for legacy synchronous callers
                query_method = getattr(self.state, "query_sync", self.state.query)
                saved = query_method(f"escore:profile:{entity_id}")
                if saved:
                    if isinstance(saved, dict):
                        self._profiles[entity_id] = EScoreProfile(**saved)
                    else:
                        self._profiles[entity_id] = saved
                    return self._profiles[entity_id]

            self._profiles[entity_id] = EScoreProfile(entity_id=entity_id)
        return self._profiles[entity_id]

    async def get_profile_async(self, entity_id: str) -> EScoreProfile:
        """Get or create reputation profile for an entity (Asynchronous)."""
        if entity_id not in self._profiles:
            if self.state:
                saved = await self.state.query(f"escore:profile:{entity_id}")
                if saved:
                    if isinstance(saved, dict):
                        self._profiles[entity_id] = EScoreProfile(**saved)
                    else:
                        self._profiles[entity_id] = saved
                    return self._profiles[entity_id]

            self._profiles[entity_id] = EScoreProfile(entity_id=entity_id)
        return self._profiles[entity_id]

    def update_dimension(self, entity_id: str, dimension: str, value: float, weight: float = 1.0, **kwargs: Any) -> float:
        """
        Update a specific reputation dimension for an entity.
        Value is blended using EMA (Exponential Moving Average).
        """
        if dimension not in E_SCORE_WEIGHTS:
            logger.warning("Invalid E-Score dimension: %s", dimension)
            return 0.0

        profile = self.get_profile(entity_id)

        # Track reality-specific scores if provided
        reality = kwargs.get("reality")
        if reality:
            profile.reality_scores[reality] = value

        current = profile.dimensions.get(dimension, 50.0)


        # EMA blending: alpha = 0.382 (PHI_INV_2) — favors stability
        alpha = PHI_INV_2 * weight
        new_val = (alpha * value) + (1.0 - alpha) * current
        profile.dimensions[dimension] = phi_bound_score(new_val)

        # Recalculate overall
        profile.overall_score = self._calculate_aggregate(profile.dimensions)
        profile.last_updated = time.time()

        # Persist if state manager is wired
        if self.state:
            asyncio.create_task(
                self.state.update(f"escore:profile:{entity_id}", profile, source="escore_tracker")
            )

        return profile.overall_score

    def _calculate_aggregate(self, dimensions: dict[str, float]) -> float:
        """Geometric mean of all 7 dimensions weighted by PHI."""
        log_sum = 0.0
        for dim, val in dimensions.items():
            w = E_SCORE_WEIGHTS.get(dim, 1.0)
            log_sum += w * math.log(max(val, 0.1))

        raw_score = math.exp(log_sum / E_SCORE_TOTAL_WEIGHT)
        return phi_bound_score(raw_score)

    def get_total_escore(self) -> float:
        """Return the average reputation of all active entities (System Health)."""
        if not self._profiles:
            return 50.0
        scores = [p.overall_score for p in self._profiles.values()]
        return sum(scores) / len(scores)

    # ── External Integration ──────────────────────────────────────────────

    async def sync_remote_reputation(self, peer_id: str) -> bool:
        """Fetch reputation score for a peer instance via κ-NET."""
        if not self._external_sync_url:
            return False

        try:
            async with httpx.AsyncClient(timeout=2.0) as client:
                resp = await client.get(f"{self._external_sync_url}/reputation/{peer_id}")
                if resp.status_code == 200:
                    resp.json()
                    # We don't overwrite local profile, we just use it for trust decisions
                    return True
        except Exception as e:
            logger.debug("Remote reputation sync failed for %s: %s", peer_id, e)
        return False

    async def broadcast_reputation(self) -> None:
        """Announce system-wide reputation metrics to the network."""
        # TODO: Implement κ-NET broadcast
        pass

    def stats(self) -> dict:
        """Reputation engine diagnostics."""
        return {
            "entities_tracked": len(self._profiles),
            "system_avg_score": round(self.get_total_escore(), 2),
            "top_entities": sorted(
                [{"id": p.entity_id, "score": p.overall_score} for p in self._profiles.values()],
                key=lambda x: x["score"],
                reverse=True,
            )[:5],
        }
