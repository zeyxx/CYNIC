"""
CYNIC Q-Learning — TD(0) with Thompson Sampling Exploration

State  = cell.state_key() → "CODE:JUDGE:PRESENT:1" (reality:analysis:time:lod)
Action = verdict           → "BARK" | "GROWL" | "WAG" | "HOWL"
Reward = q_score / 61.8   → [0, 1] (normalized φ-bounded reward)

Algorithm: TD(0) — simplest correct Q-Learning variant.
  Q(s,a) ← Q(s,a) + α × (r − Q(s,a))

Where:
  α = LEARNING_RATE = φ⁻² / 10 ≈ 0.038 (conservative, avoids catastrophic forgetting)
  r = reward ∈ [0, 1] (normalized Q-Score from judgment)
  Q(s,a) ∈ [0, 1] (stored, maps to quality of action in state)

Exploration: Thompson Sampling via Beta distribution
  For each action: sample Beta(α_wins + 1, α_losses + 1)
  Pick action with highest sample → natural Bayesian exploration

Persistence:
  In-memory dict (fast) + async flush to PostgreSQL q_table (durable).
  On startup: load q_table from DB → warm start.

φ-Integration:
  - Learning rate = φ⁻² / 10 (conservative homeostasis)
  - Confidence cap = φ⁻¹ (61.8% max certainty in any prediction)
  - Thompson α/β = Fibonacci-seeded (F(5)=5 prior, balanced exploration)

EWC (Elastic Weight Consolidation):
  - Fisher weight = min(visits / F(8), 1.0) — visit count as importance proxy
  - effective_α = α × (1 - λ × fisher), λ = EWC_PENALTY = φ⁻¹ = 0.618
  - Effect: New states learn at full α; consolidated states (≥21 visits) learn at 0.382×α
  - Prevents catastrophic forgetting when task distribution shifts (CODE→MARKET→CODE)
"""
from __future__ import annotations

import asyncio
import logging
import math
import random
import time
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

from cynic.core.phi import (
    EWC_PENALTY, LEARNING_RATE, MAX_CONFIDENCE, MAX_Q_SCORE, PHI_INV,
    PHI_INV_2, fibonacci, phi_bound,
)

logger = logging.getLogger("cynic.learning.qlearning")

# All possible verdicts (actions in Q-space)
VERDICTS: List[str] = ["BARK", "GROWL", "WAG", "HOWL"]

# Thompson prior: Fibonacci(5) = 5 pseudo-observations per arm before real data
THOMPSON_PRIOR: int = fibonacci(5)  # 5 — neutral prior, not zero


# ════════════════════════════════════════════════════════════════════════════
# DATA STRUCTURES
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class QEntry:
    """
    One (state, action) pair in the Q-Table.

    q_value: TD(0) estimate ∈ [0, 1]
    visits:  How many times this (s,a) was observed
    wins:    Thompson α — positive reward observations (reward > 0.5)
    losses:  Thompson β — negative reward observations (reward ≤ 0.5)
    last_updated: Unix timestamp
    """
    state_key: str
    action: str
    q_value: float = 0.5             # Neutral start (CYNIC doubts itself)
    visits: int = 0
    wins: int = THOMPSON_PRIOR       # F(5)=5 pseudo-wins (balanced prior)
    losses: int = THOMPSON_PRIOR     # F(5)=5 pseudo-losses (balanced prior)
    last_updated: float = field(default_factory=time.time)

    def thompson_sample(self) -> float:
        """Sample from Beta(wins+1, losses+1) for Thompson exploration."""
        # Beta(α, β) via ratio of Gamma samples (fast Python impl)
        a = self.wins + 1
        b = self.losses + 1
        x = random.gammavariate(a, 1.0)
        y = random.gammavariate(b, 1.0)
        return x / (x + y) if (x + y) > 0 else 0.5

    def to_dict(self) -> Dict:
        return {
            "state_key": self.state_key,
            "action": self.action,
            "q_value": round(self.q_value, 4),
            "visits": self.visits,
            "wins": self.wins,
            "losses": self.losses,
            "last_updated": self.last_updated,
        }


@dataclass
class LearningSignal:
    """
    A single learning event from the judgment pipeline.

    Matches the LEARNING_EVENT payload emitted by JudgeOrchestrator.
    """
    state_key: str       # cell.state_key() → "CODE:JUDGE:PRESENT:1"
    action: str          # verdict → "GROWL"
    reward: float        # q_score / MAX_Q_SCORE → [0, 1]
    judgment_id: str = ""
    loop_name: str = "JUDGE_ORCHESTRATOR"
    timestamp: float = field(default_factory=time.time)

    def __post_init__(self) -> None:
        # φ-bound reward to [0, 1]
        self.reward = max(0.0, min(1.0, self.reward))
        if self.action not in VERDICTS:
            raise ValueError(f"action must be one of {VERDICTS}, got '{self.action}'")


# ════════════════════════════════════════════════════════════════════════════
# Q-TABLE
# ════════════════════════════════════════════════════════════════════════════

class QTable:
    """
    CYNIC's Q-Learning memory.

    Stores Q(state, action) estimates using TD(0) + Thompson Sampling.
    Operates fully in-memory for speed; async flushes to PostgreSQL.

    Usage:
        qtable = QTable()

        # After a judgment:
        signal = LearningSignal(state_key="CODE:JUDGE:PRESENT:1", action="GROWL", reward=0.52)
        qtable.update(signal)

        # Before next judgment (policy consultation):
        best_action = qtable.exploit(state_key)
        explore_action = qtable.explore(state_key)

        # Check what CYNIC has learned:
        stats = qtable.stats()
    """

    def __init__(
        self,
        learning_rate: float = LEARNING_RATE,
        discount: float = PHI_INV_2,  # γ = φ⁻² = 0.382 (short-horizon)
    ) -> None:
        self._alpha = learning_rate       # ≈ 0.038
        self._gamma = discount            # 0.382 — discount future rewards conservatively
        # Nested dict: {state_key: {action: QEntry}}
        self._table: Dict[str, Dict[str, QEntry]] = defaultdict(dict)
        self._total_updates: int = 0
        self._total_states: int = 0
        self._pending_flush: List[QEntry] = []  # batch for async DB write
        self._created_at: float = time.time()

    # ── Core Update ────────────────────────────────────────────────────────

    def update(self, signal: LearningSignal) -> QEntry:
        """
        TD(0) update: Q(s,a) ← Q(s,a) + α × (r − Q(s,a))

        Also updates Thompson arms (wins/losses).
        Returns the updated QEntry.
        """
        entry = self._get_or_create(signal.state_key, signal.action)

        # TD(0) update with EWC (Elastic Weight Consolidation).
        # Fisher weight ≈ visits / F(8): heavily-visited entries resist overwriting.
        # effective_α = α × (1 - λ × fisher)
        # At visits=0:     effective_α = α         (full learning — unknown state)
        # At visits=F(8):  effective_α = α × 0.382 (consolidated — resist forgetting)
        old_q = entry.q_value
        fisher_weight = min(entry.visits / fibonacci(8), 1.0)
        effective_alpha = self._alpha * (1.0 - EWC_PENALTY * fisher_weight)
        new_q = old_q + effective_alpha * (signal.reward - old_q)
        entry.q_value = max(0.0, min(1.0, new_q))

        # Thompson arms: win if reward > 0.5 (above neutral), loss otherwise
        if signal.reward > 0.5:
            entry.wins += 1
        else:
            entry.losses += 1

        entry.visits += 1
        entry.last_updated = signal.timestamp

        self._total_updates += 1
        self._pending_flush.append(entry)

        logger.debug(
            "Q[%s][%s]: %.3f → %.3f (reward=%.3f, visits=%d)",
            signal.state_key, signal.action, old_q, entry.q_value,
            signal.reward, entry.visits,
        )

        return entry

    # ── Policy ─────────────────────────────────────────────────────────────

    def exploit(self, state_key: str) -> str:
        """
        Greedy policy: return action with highest Q-value.
        If unseen state → GROWL (cautious default, φ-aligned).
        """
        actions = self._table.get(state_key, {})
        if not actions:
            return "GROWL"  # Default: cautious, not blind optimism

        return max(actions.items(), key=lambda kv: kv[1].q_value)[0]

    def explore(self, state_key: str) -> str:
        """
        Thompson Sampling policy: sample Beta per action, pick max.
        Natural exploration — no ε-greedy hacks needed.
        """
        samples: Dict[str, float] = {}
        for action in VERDICTS:
            entry = self._get_or_create(state_key, action)
            samples[action] = entry.thompson_sample()

        return max(samples.items(), key=lambda kv: kv[1])[0]

    def predict_q(self, state_key: str, action: str) -> float:
        """Return Q(s,a) ∈ [0,1]. Returns 0.5 (neutral) if unseen."""
        entry = self._table.get(state_key, {}).get(action)
        return entry.q_value if entry else 0.5

    def confidence(self, state_key: str) -> float:
        """
        Confidence in predictions for this state = visits-based.

        confidence = min(visits / F(8), φ⁻¹)
        where F(8)=21 = "well-seen" threshold.
        Caps at φ⁻¹ = 61.8% (LAW OF DOUBT).
        """
        total_visits = sum(
            e.visits for e in self._table.get(state_key, {}).values()
        )
        raw = total_visits / fibonacci(8)  # F(8) = 21 — "enough data" threshold
        return phi_bound(raw, 0.0, MAX_CONFIDENCE)

    # ── Batch Flush ─────────────────────────────────────────────────────────

    async def flush_to_db(self, pool) -> int:
        """
        Flush pending QEntry updates to PostgreSQL q_table.

        q_table schema:
          state_key TEXT, action TEXT, q_value REAL, visit_count INT,
          last_updated TIMESTAMPTZ

        Returns: number of rows upserted.
        """
        if not self._pending_flush:
            return 0

        batch = self._pending_flush.copy()
        self._pending_flush.clear()

        async with pool.acquire() as conn:
            await conn.executemany(
                """
                INSERT INTO q_table (state_key, action, q_value, visit_count, last_updated)
                VALUES ($1, $2, $3, $4, NOW())
                ON CONFLICT (state_key, action)
                DO UPDATE SET
                    q_value = EXCLUDED.q_value,
                    visit_count = EXCLUDED.visit_count,
                    last_updated = NOW()
                """,
                [(e.state_key, e.action, e.q_value, e.visits) for e in batch],
            )

        logger.debug("Flushed %d Q-entries to DB", len(batch))
        return len(batch)

    async def load_from_db(self, pool) -> int:
        """
        Warm-start: load all Q-entries from PostgreSQL on startup.
        Returns: number of entries loaded.
        """
        async with pool.acquire() as conn:
            rows = await conn.fetch(
                "SELECT state_key, action, q_value, visit_count FROM q_table"
            )

        for row in rows:
            entry = self._get_or_create(row["state_key"], row["action"])
            entry.q_value = float(row["q_value"])
            entry.visits = int(row["visit_count"])
            # Reconstruct Thompson arms from visits (approximation)
            # Assume visits split equally between wins/losses + prior
            half = max(entry.visits // 2, 0)
            entry.wins = THOMPSON_PRIOR + half
            entry.losses = THOMPSON_PRIOR + (entry.visits - half)

        logger.info("Loaded %d Q-entries from DB (warm start)", len(rows))
        return len(rows)

    def load_from_entries(self, entries: List[Dict]) -> int:
        """
        Warm-start from a list of dicts (source-agnostic).

        Used by SurrealDB path: server.py fetches rows from SurrealDB,
        passes them here. Same logic as load_from_db() without asyncpg.

        entries: [{"state_key": str, "action": str, "q_value": float, "visit_count": int}, ...]
        Returns: number of entries loaded.
        """
        for row in entries:
            entry = self._get_or_create(row["state_key"], row["action"])
            entry.q_value = float(row.get("q_value", 0.5))
            entry.visits = int(row.get("visit_count", 0))
            half = max(entry.visits // 2, 0)
            entry.wins = THOMPSON_PRIOR + half
            entry.losses = THOMPSON_PRIOR + (entry.visits - half)
        logger.info("Loaded %d Q-entries from entries (warm start)", len(entries))
        return len(entries)

    # ── Introspection ──────────────────────────────────────────────────────

    def stats(self) -> Dict:
        """Return learning system health metrics."""
        total_entries = sum(len(v) for v in self._table.values())
        total_visits = sum(
            e.visits
            for actions in self._table.values()
            for e in actions.values()
        )
        # Average Q per state
        state_avgs = {}
        for sk, actions in self._table.items():
            avg = sum(e.q_value for e in actions.values()) / len(actions)
            state_avgs[sk] = round(avg, 3)

        # EWC: average effective learning rate across all entries
        all_entries = [e for v in self._table.values() for e in v.values()]
        if all_entries:
            avg_fisher = sum(min(e.visits / fibonacci(8), 1.0) for e in all_entries) / len(all_entries)
            avg_effective_alpha = round(self._alpha * (1.0 - EWC_PENALTY * avg_fisher), 5)
            ewc_consolidated = sum(1 for e in all_entries if e.visits >= fibonacci(8))
        else:
            avg_effective_alpha = self._alpha
            ewc_consolidated = 0

        return {
            "states": len(self._table),
            "entries": total_entries,
            "total_updates": self._total_updates,
            "total_visits": total_visits,
            "learning_rate": self._alpha,
            "discount": self._gamma,
            "state_averages": state_avgs,
            "pending_flush": len(self._pending_flush),
            "uptime_s": round(time.time() - self._created_at, 1),
            "ewc_effective_alpha": avg_effective_alpha,   # adaptive learning rate
            "ewc_consolidated": ewc_consolidated,          # entries with visits ≥ F(8)
        }

    def top_states(self, n: int = 5) -> List[Dict]:
        """Return top-N most visited states with their best actions."""
        state_data = []
        for sk, actions in self._table.items():
            total = sum(e.visits for e in actions.values())
            best_action, best_entry = max(actions.items(), key=lambda kv: kv[1].q_value)
            state_data.append({
                "state_key": sk,
                "visits": total,
                "best_action": best_action,
                "best_q": round(best_entry.q_value, 3),
                "confidence": round(self.confidence(sk), 3),
            })
        return sorted(state_data, key=lambda d: d["visits"], reverse=True)[:n]

    def reset(self) -> None:
        """Reset Q-Table (for testing). Does NOT touch DB."""
        self._table.clear()
        self._pending_flush.clear()
        self._total_updates = 0
        self._created_at = time.time()

    # ── Internal ───────────────────────────────────────────────────────────

    def _get_or_create(self, state_key: str, action: str) -> QEntry:
        """Get or create QEntry for (state, action). Tracks new states."""
        if state_key not in self._table:
            self._total_states += 1
            self._table[state_key] = {}

        if action not in self._table[state_key]:
            self._table[state_key][action] = QEntry(
                state_key=state_key,
                action=action,
            )

        return self._table[state_key][action]


# ════════════════════════════════════════════════════════════════════════════
# LEARNING LOOP (Event-driven integration)
# ════════════════════════════════════════════════════════════════════════════

class LearningLoop:
    """
    Connects QTable to the CYNIC event bus.

    Subscribes to LEARNING_EVENT → calls qtable.update().
    Flushes to DB every F(8)=21 updates.

    Usage:
        loop = LearningLoop(qtable, pool=postgres_pool)
        loop.start(event_bus)
        # ... CYNIC runs, events flow, Q-Table learns automatically ...
        loop.stop()
    """

    FLUSH_INTERVAL = fibonacci(8)  # F(8) = 21 updates before flush

    def __init__(self, qtable: QTable, pool=None) -> None:
        self.qtable = qtable
        self._pool = pool
        self._active = False
        self._updates_since_flush: int = 0

    def start(self, event_bus) -> None:
        """Register LEARNING_EVENT listener on the event bus."""
        from cynic.core.event_bus import CoreEvent

        event_bus.on(CoreEvent.LEARNING_EVENT, self._on_learning_event)
        self._active = True
        logger.info("LearningLoop started — listening for LEARNING_EVENT")

    def stop(self) -> None:
        self._active = False
        logger.info("LearningLoop stopped")

    async def _on_learning_event(self, event) -> None:
        """Handle a LEARNING_EVENT from the orchestrator."""
        if not self._active:
            return

        payload = event.payload
        try:
            signal = LearningSignal(
                state_key=payload["state_key"],
                action=payload["action"],
                reward=float(payload["reward"]),
                judgment_id=payload.get("judgment_id", ""),
                loop_name=payload.get("loop_name", "UNKNOWN"),
            )
        except (KeyError, ValueError) as e:
            logger.warning("Invalid LEARNING_EVENT payload: %s — %s", payload, e)
            return

        entry = self.qtable.update(signal)
        self._updates_since_flush += 1

        # Emit EWC_CHECKPOINT when entry first consolidates (visits == F(8) = 21).
        # Only on the EXACT crossing — never re-emitted for the same entry.
        if entry.visits == fibonacci(8):
            from cynic.core.event_bus import CoreEvent, Event, get_core_bus
            await get_core_bus().emit(Event(
                type=CoreEvent.EWC_CHECKPOINT,
                payload={
                    "state_key": signal.state_key,
                    "action":    signal.action,
                    "q_value":   entry.q_value,
                    "visits":    entry.visits,
                    "loop_name": signal.loop_name,
                },
                source="learning_loop",
            ))
            logger.info(
                "EWC_CHECKPOINT: state=%s action=%s consolidated at %d visits (q=%.3f)",
                signal.state_key, signal.action, entry.visits, entry.q_value,
            )

        # Flush to DB every FLUSH_INTERVAL updates; emit Q_TABLE_UPDATED on success.
        if self._pool and self._updates_since_flush >= self.FLUSH_INTERVAL:
            flushed = await self.qtable.flush_to_db(self._pool)
            self._updates_since_flush = 0
            stats = self.qtable.stats()
            from cynic.core.event_bus import CoreEvent, Event, get_core_bus
            await get_core_bus().emit(Event(
                type=CoreEvent.Q_TABLE_UPDATED,
                payload={
                    "flushed":          flushed,
                    "total_entries":    stats["entries"],
                    "ewc_consolidated": stats["ewc_consolidated"],
                    "total_updates":    stats["total_updates"],
                },
                source="learning_loop",
            ))
