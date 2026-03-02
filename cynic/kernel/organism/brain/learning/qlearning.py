"""
CYNIC Q-Learning â€” TD(0) with Thompson Sampling Exploration

State  = cell.state_key() â’ "CODE:JUDGE:PRESENT:1" (reality:analysis:time:lod)
Action = verdict           â’ "BARK" | "GROWL" | "WAG" | "HOWL"
Reward = q_score / 61.8   â’ [0, 1] (normalized Ï-bounded reward)

Algorithm: TD(0) â€” simplest correct Q-Learning variant.
  Q(s,a) â Q(s,a) + Î Ã— (r âˆ’ Q(s,a))

Where:
  Î = LEARNING_RATE = Ïâ»Â² / 10 â‰ˆ 0.038 (conservative, avoids catastrophic forgetting)
  r = reward âˆˆ [0, 1] (normalized Q-Score from judgment)
  Q(s,a) âˆˆ [0, 1] (stored, maps to quality of action in state)

Exploration: Thompson Sampling via Beta distribution
  For each action: sample Beta(Î_wins + 1, Î_losses + 1)
  Pick action with highest sample â’ natural Bayesian exploration

Persistence:
  In-memory dict (fast) + async flush to PostgreSQL q_table (durable).
  On startup: load q_table from DB â’ warm start.

Ï-Integration:
  - Learning rate = Ïâ»Â² / 10 (conservative homeostasis)
  - Confidence cap = Ïâ»Â¹ (61.8% max certainty in any prediction)
  - Thompson Î/Î² = Fibonacci-seeded (F(5)=5 prior, balanced exploration)

EWC (Elastic Weight Consolidation):
  - Fisher weight = min(visits / F(8), 1.0) â€” visit count as importance proxy
  - effective_Î = Î Ã— (1 - Î» Ã— fisher), Î» = EWC_PENALTY = Ïâ»Â¹ = 0.618
  - Effect: New states learn at full Î; consolidated states (â‰¥21 visits) learn at 0.382Ã—Î
  - Prevents catastrophic forgetting when task distribution shifts (CODEâ’MARKETâ’CODE)
"""

from __future__ import annotations

import logging
import random
import time
from collections import defaultdict
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any, Optional

if TYPE_CHECKING:
    from cynic.kernel.core.event_bus import Event

from cynic.kernel.core.phi import (
    EWC_PENALTY,
    LEARNING_RATE,
    MAX_CONFIDENCE,
    PHI_INV_2,
    fibonacci,
    phi_bound,
)

logger = logging.getLogger("cynic.kernel.organism.brain.learning.qlearning")

# All possible verdicts (actions in Q-space)
VERDICTS: list[str] = ["BARK", "GROWL", "WAG", "HOWL"]

# Thompson prior: Fibonacci(5) = 5 pseudo-observations per arm before real data
THOMPSON_PRIOR: int = fibonacci(5)  # 5 â€” neutral prior, not zero


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# DATA STRUCTURES
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@dataclass
class QEntry:
    """
    One (state, action) pair in the Q-Table.

    q_value: TD(0) estimate âˆˆ [0, 1]
    visits:  How many times this (s,a) was observed
    wins:    Thompson Î â€” positive reward observations (reward > 0.5)
    losses:  Thompson Î² â€” negative reward observations (reward â‰¤ 0.5)
    last_updated: Unix timestamp
    """

    state_key: str
    action: str
    q_value: float = 0.5  # Neutral start (CYNIC doubts itself)
    visits: int = 0
    wins: int = THOMPSON_PRIOR  # F(5)=5 pseudo-wins (balanced prior)
    losses: int = THOMPSON_PRIOR  # F(5)=5 pseudo-losses (balanced prior)
    last_updated: float = field(default_factory=time.time)

    def thompson_sample(self) -> float:
        """Sample from Beta(wins+1, losses+1) for Thompson exploration."""
        # Beta(Î, Î²) via ratio of Gamma samples (fast Python impl)
        a = self.wins + 1
        b = self.losses + 1
        x = random.gammavariate(a, 1.0)
        y = random.gammavariate(b, 1.0)
        return x / (x + y) if (x + y) > 0 else 0.5

    def to_dict(self) -> dict[str, Any]:
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

    state_key: str  # cell.state_key() â’ "CODE:JUDGE:PRESENT:1"
    action: str  # verdict â’ "GROWL"
    reward: float  # q_score / MAX_Q_SCORE â’ [0, 1]
    judgment_id: str = ""
    loop_name: str = "JUDGE_ORCHESTRATOR"
    timestamp: float = field(default_factory=time.time)

    def __post_init__(self) -> None:
        # Ï-bound reward to [0, 1]
        self.reward = max(0.0, min(1.0, self.reward))
        if self.action not in VERDICTS:
            raise ValueError(f"action must be one of {VERDICTS}, got '{self.action}'")


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# Q-TABLE
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


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
        discount: float = PHI_INV_2,  # Î³ = Ïâ»Â² = 0.382 (short-horizon)
        storage: Optional[Any] = None,
    ) -> None:
        self._alpha = learning_rate  # â‰ˆ 0.038
        self._gamma = discount  # 0.382 â€” discount future rewards conservatively
        self.storage = storage
        # Nested dict: {state_key: {action: QEntry}}
        self._table: dict[str, dict[str, QEntry]] = defaultdict(dict)
        self._total_updates: int = 0
        self._total_states: int = 0
        self._pending_flush: list[QEntry] = []  # batch for async DB write
        self._created_at: float = time.time()

    # â”€â”€ Core Update â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    def update(self, signal: LearningSignal) -> QEntry:
        """
        TD(0) update: Q(s,a) â Q(s,a) + Î Ã— (r âˆ’ Q(s,a))

        Also updates Thompson arms (wins/losses).
        Returns the updated QEntry.
        """
        entry = self._get_or_create(signal.state_key, signal.action)

        # TD(0) update with EWC (Elastic Weight Consolidation).
        # Fisher weight â‰ˆ visits / F(8): heavily-visited entries resist overwriting.
        # effective_Î = Î Ã— (1 - Î» Ã— fisher)
        # At visits=0:     effective_Î = Î         (full learning â€” unknown state)
        # At visits=F(8):  effective_Î = Î Ã— 0.382 (consolidated â€” resist forgetting)
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
            "Q[%s][%s]: %.3f â’ %.3f (reward=%.3f, visits=%d)",
            signal.state_key,
            signal.action,
            old_q,
            entry.q_value,
            signal.reward,
            entry.visits,
        )

        return entry

    # â”€â”€ Policy â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    def exploit(self, state_key: str) -> str:
        """
        Greedy policy: return action with highest Q-value.
        If unseen state â’ GROWL (cautious default, Ï-aligned).
        """
        actions = self._table.get(state_key, {})
        if not actions:
            return "GROWL"  # Default: cautious, not blind optimism

        return max(actions.items(), key=lambda kv: kv[1].q_value)[0]

    def explore(self, state_key: str) -> str:
        """
        Thompson Sampling policy: sample Beta per action, pick max.
        Natural exploration â€” no Îµ-greedy hacks needed.
        """
        samples: dict[str, float] = {}
        for action in VERDICTS:
            entry = self._get_or_create(state_key, action)
            samples[action] = entry.thompson_sample()

        return max(samples.items(), key=lambda kv: kv[1])[0]

    def predict_q(self, state_key: str, action: str) -> float:
        """Return Q(s,a) âˆˆ [0,1]. Returns 0.5 (neutral) if unseen."""
        entry = self._table.get(state_key, {}).get(action)
        return entry.q_value if entry else 0.5

    def confidence(self, state_key: str) -> float:
        """
        Confidence in predictions for this state = visits-based.

        confidence = min(visits / F(8), Ïâ»Â¹)
        where F(8)=21 = "well-seen" threshold.
        Caps at Ïâ»Â¹ = 61.8% (LAW OF DOUBT).
        """
        total_visits = sum(e.visits for e in self._table.get(state_key, {}).values())
        raw = total_visits / fibonacci(8)  # F(8) = 21 â€” "enough data" threshold
        return phi_bound(raw, 0.0, MAX_CONFIDENCE)

    # â”€â”€ Batch Flush â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    async def flush_to_db(self) -> int:
        """
        Flush pending QEntry updates to SurrealDB.

        PRIORITY 3 FIX: Async/sync boundary safety.
        - Takes atomic snapshot of _pending_flush (copy+clear pattern)
        - Creates immutable dict copies to prevent concurrent mutation
        - No interference between sync update() and async flush()
        - If update() is called during flush, new entries go to new batch
        """
        if not self.storage:
            return 0

        # Atomic snapshot: copy list, then clear original
        # This prevents race where update() adds entries while flush is reading
        batch = self._pending_flush.copy()
        self._pending_flush.clear()

        if not batch:
            return 0

        count = 0
        for e in batch:
            try:
                # Create immutable snapshot of entry to prevent concurrent mutation
                # If update() modifies entry while flush is running, this snapshot stays consistent
                entry_snapshot = {
                    "state_key": e.state_key,
                    "action": e.action,
                    "q_value": e.q_value,
                    "visits": e.visits,
                    "wins": e.wins,
                    "losses": e.losses,
                    "last_updated": e.last_updated,
                }

                # Delegate to the real repository
                await self.storage.update(
                    state_key=entry_snapshot["state_key"],
                    action=entry_snapshot["action"],
                    q_value=entry_snapshot["q_value"],
                    visits=entry_snapshot["visits"]
                )
                count += 1
            except Exception as exc:
                logger.error("QTable flush failed for %s: %s", e.state_key, exc)

        if count > 0:
            logger.debug("Flushed %d Q-entries to SurrealDB", count)
        return count

    async def load_from_db(self) -> int:
        """
        Warm-start: load all Q-entries from SurrealDB on startup.
        """
        if not self.storage:
            return 0
            
        try:
            entries = await self.storage.get_all()
            return self.load_from_entries(entries)
        except Exception as e:
            logger.warning("Failed to load QTable from SurrealDB: %s", e)
            return 0

    def load_from_entries(self, entries: list[dict]) -> int:
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

    # â”€â”€ Introspection â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    def stats(self) -> dict[str, Any]:
        """Return learning system health metrics."""
        total_entries = sum(len(v) for v in self._table.values())
        total_visits = sum(e.visits for actions in self._table.values() for e in actions.values())
        # Average Q per state
        state_avgs = {}
        for sk, actions in self._table.items():
            avg = sum(e.q_value for e in actions.values()) / len(actions)
            state_avgs[sk] = round(avg, 3)

        # EWC: average effective learning rate across all entries
        all_entries = [e for v in self._table.values() for e in v.values()]
        if all_entries:
            avg_fisher = sum(min(e.visits / fibonacci(8), 1.0) for e in all_entries) / len(
                all_entries
            )
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
            "ewc_effective_alpha": avg_effective_alpha,  # adaptive learning rate
            "ewc_consolidated": ewc_consolidated,  # entries with visits â‰¥ F(8)
        }

    def prune(self, max_entries: int = 10000) -> int:
        """
        Active Forgetting (Axiom BURN) to survive infinity.

        Removes the least valuable knowledge from RAM to prevent OOM.
        Least valuable = Low visits AND (Q-Value near 0.5 or high Fisher penalty).

        Returns: number of entries pruned.
        """
        all_entries = []
        for state_key, actions in self._table.items():
            for action, entry in actions.items():
                all_entries.append((state_key, action, entry))

        if len(all_entries) <= max_entries:
            return 0

        # Score entries for survival: High visits = good. Q near 0.5 = boring/useless.
        def survival_score(e: QEntry) -> float:
            q_variance = abs(e.q_value - 0.5)  # How decisive is this knowledge?
            return e.visits * q_variance

        all_entries.sort(key=lambda item: survival_score(item[2]), reverse=True)

        # Keep top max_entries
        survivors = all_entries[:max_entries]
        doomed = all_entries[max_entries:]

        # Rebuild table with survivors
        self._table.clear()
        for state_key, action, entry in survivors:
            if state_key not in self._table:
                self._table[state_key] = {}
            self._table[state_key][action] = entry

        logger.info("QTable pruned %d least valuable entries (BURN axiom active)", len(doomed))
        return len(doomed)

    def top_states(self, n: int = 5) -> list[dict]:
        """Return top-N most visited states with their best actions."""
        state_data = []
        for sk, actions in self._table.items():
            total = sum(e.visits for e in actions.values())
            best_action, best_entry = max(actions.items(), key=lambda kv: kv[1].q_value)
            state_data.append(
                {
                    "state_key": sk,
                    "visits": total,
                    "best_action": best_action,
                    "best_q": round(best_entry.q_value, 3),
                    "confidence": round(self.confidence(sk), 3),
                }
            )
        return sorted(state_data, key=lambda d: d["visits"], reverse=True)[:n]

    def matrix_stats(self) -> dict:
        """
        7Ã—7Ã—7 Lazy Materialization coverage report.

        Shows which cells of the 343-cell matrix have been visited.
        State keys follow format: "REALITY:ANALYSIS:TIME_DIM:LOD"

        Returns dict with:
          total_cells: states seen (materialized)
          matrix_343: max possible (7Ã—7Ã—7 = 343, ignoring LOD)
          coverage_pct: % of 7Ã—7Ã—7 matrix materialized
          by_reality, by_analysis, by_time_dim: per-dimension counts
        """
        by_reality: dict = {}
        by_analysis: dict = {}
        by_time_dim: dict = {}

        for sk in self._table:
            parts = sk.split(":")
            if len(parts) >= 3:
                reality, analysis, time_dim = parts[0], parts[1], parts[2]
                by_reality[reality] = by_reality.get(reality, 0) + 1
                by_analysis[analysis] = by_analysis.get(analysis, 0) + 1
                by_time_dim[time_dim] = by_time_dim.get(time_dim, 0) + 1

        total = len(self._table)
        coverage = round(total / 343 * 100, 1)  # 7Ã—7Ã—7 = 343

        return {
            "total_cells": total,
            "matrix_343": 343,
            "coverage_pct": coverage,
            "by_reality": by_reality,
            "by_analysis": by_analysis,
            "by_time_dim": by_time_dim,
        }

    def reset(self) -> None:
        """Reset Q-Table (for testing). Does NOT touch DB."""
        self._table.clear()
        self._pending_flush.clear()
        self._total_updates = 0
        self._created_at = time.time()

    # â”€â”€ Internal â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# LEARNING LOOP (Event-driven integration)
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class LearningLoop:
    """
    Connects QTable to the CYNIC event bus.

    Subscribes to LEARNING_EVENT â’ calls qtable.update().
    Flushes to DB every F(8)=21 updates.

    Usage:
        loop = LearningLoop(qtable, instance_id=instance_id, pool=postgres_pool)
        loop.start(event_bus)
        # ... CYNIC runs, events flow, Q-Table learns automatically ...
        loop.stop()
    """

    FLUSH_INTERVAL = fibonacci(8)  # F(8) = 21 updates before flush

    def __init__(self, qtable: QTable, instance_id: str, pool=None) -> None:
        self.qtable = qtable
        self._pool = pool
        self._instance_id = instance_id

        self._updates_since_flush: int = 0
        self._learning_rate = qtable._alpha  # Reference to QTable's Î
        self.instance_id = instance_id  # Level 2 multi-instance support
        self._bus = None  # Cached bus reference (set in start())

    def adjust_learning_rate(self, delta: float) -> None:
        """
        Adjust the learning rate (Î) by delta.

        Called by MetaCognitionHandler to adapt exploration/exploitation.
        Delta is Ï-bounded: max Â0.618 per call.
        """
        new_rate = self._learning_rate + delta
        # Clamp to Ï-bounded range: [0.01, 0.2]
        self._learning_rate = max(0.01, min(0.2, new_rate))
        # Also update the underlying QTable
        self.qtable._alpha = self._learning_rate
        logger.info("Learning rate Î adjusted to %.4f (delta=%.3f)", self._learning_rate, delta)

    def start(self, event_bus: Optional[Any] = None) -> None:
        """Register LEARNING_EVENT listener on the event bus."""
        from cynic.kernel.core.event_bus import CoreEvent, get_core_bus

        target_bus = event_bus or get_core_bus(self.instance_id)
        self._bus = target_bus  # Cache for use in _on_learning_event
        target_bus.on(CoreEvent.LEARNING_EVENT, self._on_learning_event)
        self._active = True
        logger.info("LearningLoop started â€” listening for LEARNING_EVENT")

    def stop(self) -> None:
        self._active = False
        logger.info("LearningLoop stopped")

    async def _on_learning_event(self, event: Event) -> None:
        """Handle a LEARNING_EVENT from the orchestrator."""
        if not self._active:
            return

        payload = event.dict_payload
        try:
            signal = LearningSignal(
                state_key=payload["state_key"],
                action=payload["action"],
                reward=float(payload["reward"]),
                judgment_id=payload.get("judgment_id", ""),
                loop_name=payload.get("loop_name", "UNKNOWN"),
            )
        except (KeyError, ValueError) as e:
            logger.warning("Invalid LEARNING_EVENT payload: %s â€” %s", payload, e)
            return

        entry = self.qtable.update(signal)
        self._updates_since_flush += 1

        # Emit EWC_CHECKPOINT when entry first consolidates (visits == F(8) = 21).
        # Only on the EXACT crossing â€” never re-emitted for the same entry.
        if entry.visits == fibonacci(8):
            from cynic.kernel.core.event_bus import CoreEvent, Event
            from cynic.kernel.core.events_schema import EwcCheckpointPayload

            await self._bus.emit(
                Event.typed(
                    CoreEvent.EWC_CHECKPOINT,
                    EwcCheckpointPayload(
                        q_value=entry.q_value,
                        state_key=signal.state_key,
                        action=signal.action,
                        visits=entry.visits,
                        loop_name=signal.loop_name,
                    ),
                    source="learning_loop",
                )
            )
            logger.info(
                "EWC_CHECKPOINT: state=%s action=%s consolidated at %d visits (q=%.3f)",
                signal.state_key,
                signal.action,
                entry.visits,
                entry.q_value,
            )

        # Flush to DB every FLUSH_INTERVAL updates; emit Q_TABLE_UPDATED on success.
        if self.qtable.storage and self._updates_since_flush >= self.FLUSH_INTERVAL:
            flushed = await self.qtable.flush_to_db()
            self._updates_since_flush = 0
            stats = self.qtable.stats()
            from cynic.kernel.core.event_bus import CoreEvent, Event
            from cynic.kernel.core.events_schema import QTableUpdatedPayload

            await self._bus.emit(
                Event.typed(
                    CoreEvent.Q_TABLE_UPDATED,
                    QTableUpdatedPayload(
                        flushed=flushed,
                        total_entries=stats["entries"],
                        ewc_consolidated=stats["ewc_consolidated"],
                        total_updates=stats["total_updates"],
                    ),
                    source="learning_loop",
                )
            )
