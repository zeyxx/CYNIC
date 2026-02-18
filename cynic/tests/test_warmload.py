"""
CYNIC Q-Table Warm-Load Tests — Task #8

Validates PostgreSQL warm-load path WITHOUT a real database.
Uses asyncio mock objects to simulate asyncpg pool behavior.

Tests:
  - QTable.load_from_db() populates in-memory table from rows
  - QTable.flush_to_db() writes pending entries to DB
  - Thompson arms reconstructed correctly from visits
  - Empty DB → 0 entries, no crash
  - Multiple states/actions load independently
  - Warm-load does not override fresher in-memory data (load is additive)
"""
from __future__ import annotations

import asyncio
import time
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.core.phi import MAX_CONFIDENCE, LEARNING_RATE, fibonacci
from cynic.learning.qlearning import (
    QTable, LearningSignal, LearningLoop, QEntry, VERDICTS, THOMPSON_PRIOR,
)


# ════════════════════════════════════════════════════════════════════════════
# MOCK HELPERS
# ════════════════════════════════════════════════════════════════════════════

def make_mock_pool(rows: list) -> MagicMock:
    """
    Build a mock asyncpg pool that returns `rows` from conn.fetch().

    rows: list of dicts with keys matching q_table schema:
      state_key, action, q_value, visit_count
    """
    # Build asyncpg-compatible record-like dicts
    record_rows = [
        {
            "state_key": r["state_key"],
            "action": r["action"],
            "q_value": float(r["q_value"]),
            "visit_count": int(r.get("visit_count", 0)),
        }
        for r in rows
    ]

    conn = AsyncMock()
    conn.fetch = AsyncMock(return_value=record_rows)
    conn.executemany = AsyncMock(return_value=None)

    pool = MagicMock()
    # Make pool.acquire() work as async context manager
    pool.acquire = MagicMock()
    pool.acquire.return_value.__aenter__ = AsyncMock(return_value=conn)
    pool.acquire.return_value.__aexit__ = AsyncMock(return_value=False)

    return pool, conn


# ════════════════════════════════════════════════════════════════════════════
# load_from_db()
# ════════════════════════════════════════════════════════════════════════════

class TestQTableLoadFromDB:
    """QTable.load_from_db() warm-start from PostgreSQL."""

    @pytest.mark.asyncio
    async def test_empty_db_returns_zero(self):
        """Empty DB → 0 entries loaded, table still empty."""
        pool, _ = make_mock_pool([])
        qtable = QTable()
        count = await qtable.load_from_db(pool)
        assert count == 0
        assert qtable.stats()["states"] == 0

    @pytest.mark.asyncio
    async def test_loads_single_entry(self):
        """Single row loads correctly into Q-table."""
        pool, _ = make_mock_pool([
            {"state_key": "CODE:JUDGE:PRESENT:1", "action": "WAG", "q_value": 0.72, "visit_count": 10},
        ])
        qtable = QTable()
        count = await qtable.load_from_db(pool)
        assert count == 1
        assert qtable.predict_q("CODE:JUDGE:PRESENT:1", "WAG") == pytest.approx(0.72)

    @pytest.mark.asyncio
    async def test_loads_multiple_states(self):
        """Multiple states/actions load independently."""
        rows = [
            {"state_key": "CODE:JUDGE:PRESENT:1", "action": "WAG",  "q_value": 0.80, "visit_count": 20},
            {"state_key": "CODE:JUDGE:PRESENT:1", "action": "HOWL", "q_value": 0.60, "visit_count": 5},
            {"state_key": "SOLANA:ACT:PRESENT:1", "action": "BARK", "q_value": 0.20, "visit_count": 3},
        ]
        pool, _ = make_mock_pool(rows)
        qtable = QTable()
        count = await qtable.load_from_db(pool)

        assert count == 3
        assert qtable.predict_q("CODE:JUDGE:PRESENT:1", "WAG") == pytest.approx(0.80)
        assert qtable.predict_q("CODE:JUDGE:PRESENT:1", "HOWL") == pytest.approx(0.60)
        assert qtable.predict_q("SOLANA:ACT:PRESENT:1", "BARK") == pytest.approx(0.20)

    @pytest.mark.asyncio
    async def test_exploit_uses_loaded_values(self):
        """After load, exploit() returns the best pre-learned action."""
        rows = [
            {"state_key": "s", "action": "WAG",  "q_value": 0.90, "visit_count": 50},
            {"state_key": "s", "action": "BARK", "q_value": 0.10, "visit_count": 50},
            {"state_key": "s", "action": "GROWL","q_value": 0.40, "visit_count": 10},
            {"state_key": "s", "action": "HOWL", "q_value": 0.60, "visit_count": 10},
        ]
        pool, _ = make_mock_pool(rows)
        qtable = QTable()
        await qtable.load_from_db(pool)

        best = qtable.exploit("s")
        assert best == "WAG", f"Expected WAG (highest Q), got {best}"

    @pytest.mark.asyncio
    async def test_visits_reconstructed(self):
        """Visits loaded correctly → confidence grows."""
        rows = [
            {"state_key": "s", "action": "WAG", "q_value": 0.70, "visit_count": 21},
        ]
        pool, _ = make_mock_pool(rows)
        qtable = QTable()
        await qtable.load_from_db(pool)

        entry = qtable._table.get("s", {}).get("WAG")
        assert entry is not None
        assert entry.visits == 21

        # Confidence should be > 0 (21 visits = F(8) threshold)
        conf = qtable.confidence("s")
        assert conf > 0.0
        assert conf <= MAX_CONFIDENCE

    @pytest.mark.asyncio
    async def test_thompson_arms_reconstructed(self):
        """Thompson arms approximate from visits + THOMPSON_PRIOR."""
        rows = [
            {"state_key": "s", "action": "WAG", "q_value": 0.70, "visit_count": 10},
        ]
        pool, _ = make_mock_pool(rows)
        qtable = QTable()
        await qtable.load_from_db(pool)

        entry = qtable._table["s"]["WAG"]
        # wins + losses ≥ 2 × THOMPSON_PRIOR (neutral prior + visits)
        assert entry.wins >= THOMPSON_PRIOR
        assert entry.losses >= THOMPSON_PRIOR

    @pytest.mark.asyncio
    async def test_load_does_not_reset_existing_memory(self):
        """Load is additive — does not wipe existing in-memory state."""
        qtable = QTable()
        # Pre-populate in-memory
        qtable.update(LearningSignal(state_key="fresh", action="HOWL", reward=0.9))

        pool, _ = make_mock_pool([
            {"state_key": "loaded", "action": "BARK", "q_value": 0.2, "visit_count": 5},
        ])
        await qtable.load_from_db(pool)

        # Both entries coexist
        assert qtable.predict_q("fresh", "HOWL") > 0.5   # Still in memory
        assert qtable.predict_q("loaded", "BARK") == pytest.approx(0.2)

    @pytest.mark.asyncio
    async def test_q_values_stay_in_unit_range(self):
        """All loaded Q-values must be in [0, 1]."""
        rows = [
            {"state_key": "s1", "action": "HOWL", "q_value": 0.0,  "visit_count": 0},
            {"state_key": "s2", "action": "BARK", "q_value": 1.0,  "visit_count": 100},
            {"state_key": "s3", "action": "WAG",  "q_value": 0.5,  "visit_count": 5},
        ]
        pool, _ = make_mock_pool(rows)
        qtable = QTable()
        await qtable.load_from_db(pool)

        for sk, actions in qtable._table.items():
            for action, entry in actions.items():
                assert 0.0 <= entry.q_value <= 1.0, (
                    f"Q[{sk}][{action}] = {entry.q_value} out of [0,1]"
                )


# ════════════════════════════════════════════════════════════════════════════
# flush_to_db()
# ════════════════════════════════════════════════════════════════════════════

class TestQTableFlushToDB:
    """QTable.flush_to_db() persistence to PostgreSQL."""

    @pytest.mark.asyncio
    async def test_flush_empty_returns_zero(self):
        """No pending updates → flush returns 0."""
        pool, conn = make_mock_pool([])
        qtable = QTable()
        count = await qtable.flush_to_db(pool)
        assert count == 0
        conn.executemany.assert_not_called()

    @pytest.mark.asyncio
    async def test_flush_writes_pending_entries(self):
        """After updates, flush writes them to DB."""
        pool, conn = make_mock_pool([])
        qtable = QTable()

        # Generate 3 pending updates
        for action in ("WAG", "BARK", "GROWL"):
            qtable.update(LearningSignal(state_key="s", action=action, reward=0.5))

        count = await qtable.flush_to_db(pool)
        assert count == 3
        conn.executemany.assert_called_once()

    @pytest.mark.asyncio
    async def test_flush_clears_pending(self):
        """After flush, pending list is empty."""
        pool, conn = make_mock_pool([])
        qtable = QTable()
        qtable.update(LearningSignal(state_key="s", action="WAG", reward=0.6))

        await qtable.flush_to_db(pool)
        assert qtable.stats()["pending_flush"] == 0

    @pytest.mark.asyncio
    async def test_flush_passes_correct_values(self):
        """Flush passes (state_key, action, q_value, visits) to DB."""
        pool, conn = make_mock_pool([])
        qtable = QTable()
        qtable.update(LearningSignal(state_key="TEST:STATE", action="HOWL", reward=0.8))

        await qtable.flush_to_db(pool)

        call_args = conn.executemany.call_args
        sql = call_args[0][0]
        data = call_args[0][1]

        assert "INSERT INTO q_table" in sql
        assert len(data) == 1
        row = data[0]
        assert row[0] == "TEST:STATE"   # state_key
        assert row[1] == "HOWL"         # action
        assert isinstance(row[2], float) # q_value
        assert isinstance(row[3], int)   # visits


# ════════════════════════════════════════════════════════════════════════════
# LearningLoop flush integration
# ════════════════════════════════════════════════════════════════════════════

class TestLearningLoopFlush:
    """LearningLoop flushes at FLUSH_INTERVAL updates."""

    @pytest.mark.asyncio
    async def test_flush_triggered_at_interval(self):
        """After FLUSH_INTERVAL updates, flush_to_db is called automatically."""
        from cynic.core.event_bus import get_core_bus, reset_all_buses, Event, CoreEvent

        reset_all_buses()
        pool, conn = make_mock_pool([])
        qtable = QTable()
        loop = LearningLoop(qtable=qtable, pool=pool)
        loop.start(get_core_bus())

        # Emit exactly FLUSH_INTERVAL learning events
        for i in range(LearningLoop.FLUSH_INTERVAL):
            await get_core_bus().emit(Event(
                type=CoreEvent.LEARNING_EVENT,
                payload={
                    "state_key": "s",
                    "action": "WAG",
                    "reward": 0.7,
                    "judgment_id": f"j-{i}",
                },
            ))

        # Give event loop a tick
        await asyncio.sleep(0.01)
        loop.stop()

        # DB should have been written at least once
        assert conn.executemany.called, (
            "flush_to_db should have been called at FLUSH_INTERVAL updates"
        )

    @pytest.mark.asyncio
    async def test_loop_handles_invalid_payload_gracefully(self):
        """Invalid payload does not crash the loop."""
        from cynic.core.event_bus import get_core_bus, reset_all_buses, Event, CoreEvent

        reset_all_buses()
        qtable = QTable()
        loop = LearningLoop(qtable=qtable)
        loop.start(get_core_bus())

        # Emit broken event
        await get_core_bus().emit(Event(
            type=CoreEvent.LEARNING_EVENT,
            payload={"missing_keys": True},  # No state_key, action, reward
        ))
        await asyncio.sleep(0.01)
        loop.stop()

        # QTable should have 0 updates (invalid payload ignored)
        assert qtable.stats()["total_updates"] == 0
