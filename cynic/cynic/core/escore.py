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
  Each entity also tracks how E-Score evolves per reality dimension
  (CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS). Sub-scores
  are computed as simple exponential moving averages of dimension scores
  within each reality context.

Usage:
    tracker = EScoreTracker()
    tracker.update("user:alice", dimension="BUILD", value=75.0)
    tracker.update("user:alice", dimension="BURN",  value=90.0, reality="SOLANA")
    score = tracker.get_score("user:alice")   # φ-weighted aggregate
    detail = tracker.get_detail("user:alice") # breakdown per dimension
"""
from __future__ import annotations

import math
import time
import logging
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

from cynic.core.phi import (
    E_SCORE_WEIGHTS, E_SCORE_TOTAL_WEIGHT,
    MAX_Q_SCORE, PHI_INV, PHI_INV_2, phi_bound_score,
)

logger = logging.getLogger("cynic.core.escore")

# Valid realities (7 dimensions of reality)
REALITIES = frozenset({"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"})

# Valid E-Score dimensions
E_DIMENSIONS = frozenset(E_SCORE_WEIGHTS.keys())

# Exponential moving average α for sub-scores (φ⁻¹ = 0.618)
# New value weight: α, old value weight: 1-α
EMA_ALPHA: float = PHI_INV

# Default starting score for a new entity/dimension (middle of scale)
DEFAULT_DIM_SCORE: float = 50.0


# ── DimScore ──────────────────────────────────────────────────────────────

@dataclass
class DimScore:
    """Running score for one E-Score dimension of one entity."""
    dimension: str
    value: float = DEFAULT_DIM_SCORE   # Current EMA value
    updates: int = 0
    last_updated: float = field(default_factory=time.time)

    def apply(self, new_value: float) -> None:
        """Update via exponential moving average."""
        new_value = max(0.0, min(new_value, MAX_Q_SCORE))
        if self.updates == 0:
            self.value = new_value
        else:
            self.value = EMA_ALPHA * new_value + (1 - EMA_ALPHA) * self.value
        self.updates += 1
        self.last_updated = time.time()


# ── EntityScore ───────────────────────────────────────────────────────────

@dataclass
class EntityScore:
    """All E-Score data for one entity."""
    entity_id: str
    dims: dict[str, DimScore] = field(default_factory=dict)
    # Per-reality: reality → dimension → DimScore
    reality_dims: dict[str, dict[str, DimScore]] = field(default_factory=dict)
    created_at: float = field(default_factory=time.time)

    def get_dim(self, dimension: str) -> DimScore:
        if dimension not in self.dims:
            self.dims[dimension] = DimScore(dimension=dimension)
        return self.dims[dimension]

    def get_reality_dim(self, reality: str, dimension: str) -> DimScore:
        if reality not in self.reality_dims:
            self.reality_dims[reality] = {}
        if dimension not in self.reality_dims[reality]:
            self.reality_dims[reality][dimension] = DimScore(dimension=dimension)
        return self.reality_dims[reality][dimension]

    def aggregate_score(self) -> float:
        """
        φ-weighted geometric mean across all 7 dimensions.
        Dimensions with no updates use DEFAULT_DIM_SCORE.
        """
        log_sum = 0.0
        for dim, weight in E_SCORE_WEIGHTS.items():
            score = self.dims[dim].value if dim in self.dims else DEFAULT_DIM_SCORE
            log_sum += weight * math.log(max(score, 0.1))
        geo_mean = math.exp(log_sum / E_SCORE_TOTAL_WEIGHT)
        return phi_bound_score(geo_mean)

    def reality_score(self, reality: str) -> float:
        """
        φ-weighted geometric mean for a specific reality sub-score.
        Uses reality-specific DimScores where available, falls back to global.
        """
        if reality not in self.reality_dims:
            return self.aggregate_score()  # No reality-specific data

        log_sum = 0.0
        for dim, weight in E_SCORE_WEIGHTS.items():
            r_dims = self.reality_dims.get(reality, {})
            if dim in r_dims:
                score = r_dims[dim].value
            elif dim in self.dims:
                score = self.dims[dim].value
            else:
                score = DEFAULT_DIM_SCORE
            log_sum += weight * math.log(max(score, 0.1))
        geo_mean = math.exp(log_sum / E_SCORE_TOTAL_WEIGHT)
        return phi_bound_score(geo_mean)


# ── EScoreTracker ─────────────────────────────────────────────────────────

class EScoreTracker:
    """
    Tracks E-Score reputation for all entities observed by CYNIC.

    Thread-safe for single-event-loop use (asyncio). In-memory storage.
    All scores φ-bounded to [0, MAX_Q_SCORE].

    Entities:
        Any hashable string: "user:alice", "token:abc123", "agent:SAGE"

    Dimensions:
        BURN, BUILD, JUDGE, RUN, SOCIAL, GRAPH, HOLD

    Realities (optional per-update context):
        CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS
    """

    def __init__(self) -> None:
        self._entities: dict[str, EntityScore] = {}
        self._total_updates: int = 0
        self._created_at: float = time.time()

    # ── Mutation ──────────────────────────────────────────────────────────

    def update(
        self,
        entity_id: str,
        dimension: str,
        value: float,
        reality: str | None = None,
    ) -> float:
        """
        Record a new score observation for an entity.

        Args:
            entity_id:  Entity identifier (e.g., "user:alice", "agent:SAGE")
            dimension:  E-Score dimension (BURN/BUILD/JUDGE/RUN/SOCIAL/GRAPH/HOLD)
            value:      Score [0, MAX_Q_SCORE]
            reality:    Optional reality context (CODE/SOLANA/etc.)

        Returns:
            New aggregate E-Score for the entity after update.

        Raises:
            ValueError: If dimension or reality is invalid.
        """
        if dimension not in E_DIMENSIONS:
            raise ValueError(
                f"Invalid dimension {dimension!r}. "
                f"Valid: {sorted(E_DIMENSIONS)}"
            )
        if reality is not None and reality not in REALITIES:
            raise ValueError(
                f"Invalid reality {reality!r}. "
                f"Valid: {sorted(REALITIES)}"
            )

        entity = self._get_or_create(entity_id)

        # Update global dimension
        entity.get_dim(dimension).apply(value)

        # Update per-reality dimension if reality provided
        if reality:
            entity.get_reality_dim(reality, dimension).apply(value)

        self._total_updates += 1
        new_score = entity.aggregate_score()

        logger.debug(
            "EScore update: %s %s=%.1f%s → %.2f",
            entity_id, dimension, value,
            f" ({reality})" if reality else "",
            new_score,
        )
        return new_score

    # ── Query ─────────────────────────────────────────────────────────────

    def get_score(self, entity_id: str) -> float:
        """
        Get current aggregate E-Score for entity.
        Returns DEFAULT_DIM_SCORE if entity unknown.
        """
        if entity_id not in self._entities:
            return DEFAULT_DIM_SCORE
        return self._entities[entity_id].aggregate_score()

    def get_reality_score(self, entity_id: str, reality: str) -> float:
        """
        Get E-Score for entity within a specific reality context.
        Falls back to aggregate score if no reality-specific data.
        """
        if entity_id not in self._entities:
            return DEFAULT_DIM_SCORE
        return self._entities[entity_id].reality_score(reality)

    def get_detail(self, entity_id: str) -> dict[str, Any]:
        """
        Get full E-Score breakdown for entity.

        Returns:
            {
                "entity_id": str,
                "aggregate": float,
                "dimensions": {dim: {"value": float, "updates": int}},
                "reality_scores": {reality: float},
            }
        """
        if entity_id not in self._entities:
            return {
                "entity_id": entity_id,
                "aggregate": DEFAULT_DIM_SCORE,
                "dimensions": {
                    dim: {"value": DEFAULT_DIM_SCORE, "updates": 0}
                    for dim in E_SCORE_WEIGHTS
                },
                "reality_scores": {},
            }

        entity = self._entities[entity_id]
        dims_out = {}
        for dim in E_SCORE_WEIGHTS:
            ds = entity.dims.get(dim)
            dims_out[dim] = {
                "value": round(ds.value if ds else DEFAULT_DIM_SCORE, 2),
                "updates": ds.updates if ds else 0,
                "weight": E_SCORE_WEIGHTS[dim],
            }

        reality_scores = {
            r: round(entity.reality_score(r), 2)
            for r in entity.reality_dims
        }

        return {
            "entity_id": entity_id,
            "aggregate": round(entity.aggregate_score(), 2),
            "dimensions": dims_out,
            "reality_scores": reality_scores,
        }

    def top_entities(self, n: int = 5) -> list[tuple[str, float]]:
        """
        Return top-N entities by aggregate E-Score (descending).
        """
        scored = [
            (eid, entity.aggregate_score())
            for eid, entity in self._entities.items()
        ]
        scored.sort(key=lambda x: x[1], reverse=True)
        return scored[:n]

    def entity_count(self) -> int:
        return len(self._entities)

    def stats(self) -> dict[str, Any]:
        return {
            "entities": len(self._entities),
            "total_updates": self._total_updates,
            "uptime_s": round(time.time() - self._created_at, 1),
        }

    # ── Private ───────────────────────────────────────────────────────────

    # ── Persistence ───────────────────────────────────────────────────────

    async def persist(self, pool) -> int:
        """
        Write all entity scores to the e_scores table.

        Uses ON CONFLICT upsert — safe to call repeatedly.
        Returns number of entities written.
        """
        if not self._entities or pool is None:
            return 0

        written = 0
        async with pool.acquire() as conn:
            for entity_id, entity in self._entities.items():
                dims = entity.dims
                get_v = lambda d: dims[d].value if d in dims else DEFAULT_DIM_SCORE  # noqa: E731
                try:
                    await conn.execute("""
                        INSERT INTO e_scores
                            (agent_id, total, burn_score, build_score, judge_score,
                             run_score, social_score, graph_score, hold_score)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                        ON CONFLICT (agent_id) DO UPDATE SET
                            total        = EXCLUDED.total,
                            burn_score   = EXCLUDED.burn_score,
                            build_score  = EXCLUDED.build_score,
                            judge_score  = EXCLUDED.judge_score,
                            run_score    = EXCLUDED.run_score,
                            social_score = EXCLUDED.social_score,
                            graph_score  = EXCLUDED.graph_score,
                            hold_score   = EXCLUDED.hold_score,
                            updated_at   = NOW()
                    """,
                        entity_id,
                        entity.aggregate_score(),
                        get_v("BURN"), get_v("BUILD"), get_v("JUDGE"),
                        get_v("RUN"), get_v("SOCIAL"), get_v("GRAPH"), get_v("HOLD"),
                    )
                    written += 1
                except Exception as exc:
                    logger.debug("EScore persist failed for %s: %s", entity_id, exc)

        logger.info("EScoreTracker: persisted %d entities to DB", written)
        return written

    async def restore(self, pool) -> int:
        """
        Load entity scores from e_scores table on startup.

        Directly sets dimension values (no EMA — recovering exact saved state).
        Returns number of entities restored.
        """
        if pool is None:
            return 0
        try:
            async with pool.acquire() as conn:
                rows = await conn.fetch("SELECT * FROM e_scores")

            dim_columns = {
                "BURN":   "burn_score",
                "BUILD":  "build_score",
                "JUDGE":  "judge_score",
                "RUN":    "run_score",
                "SOCIAL": "social_score",
                "GRAPH":  "graph_score",
                "HOLD":   "hold_score",
            }
            for row in rows:
                entity = self._get_or_create(row["agent_id"])
                for dim, col in dim_columns.items():
                    ds = entity.get_dim(dim)
                    ds.value = float(row[col])
                    ds.updates = 1  # mark as initialized (not a fresh default)

            count = len(rows)
            logger.info("EScoreTracker: restored %d entities from DB", count)
            return count
        except Exception as exc:
            logger.warning("EScoreTracker restore failed: %s", exc)
            return 0

    def _get_or_create(self, entity_id: str) -> EntityScore:
        if entity_id not in self._entities:
            self._entities[entity_id] = EntityScore(entity_id=entity_id)
        return self._entities[entity_id]
