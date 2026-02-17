"""
Integration tests — real PostgreSQL required.

Run:   pytest -m db_integration
Skip:  pytest (default — no DB integration tests run)

These tests hit real infrastructure:
    - PostgreSQL (cynic_py Docker or CYNIC_DATABASE_URL env)
    - asyncpg pool lifecycle
    - Real Q-table flush + warm-load cycle
    - Real judgment + learning event persistence

Validates the full persistence pipeline that unit tests mock with make_mock_pool().

Setup:
    docker run -e POSTGRES_USER=cynic -e POSTGRES_PASSWORD=cynic \\
               -e POSTGRES_DB=cynic_py -p 5432:5432 postgres:16
    # or:
    export CYNIC_DATABASE_URL=postgresql://cynic:cynic@localhost:5432/cynic_py
"""
from __future__ import annotations

import uuid
import asyncio
import pytest
import asyncpg

from cynic.core.phi import MAX_Q_SCORE, PHI_INV

pytestmark = pytest.mark.db_integration


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(scope="module")
def db_dsn(test_db_url: str) -> str:
    """DSN from conftest — defaults to cynic_py Docker."""
    return test_db_url


@pytest.fixture(scope="module")
def db_reachable(db_dsn: str) -> bool:
    """Check once per module if PostgreSQL is reachable."""
    async def _check() -> bool:
        try:
            conn = await asyncpg.connect(dsn=db_dsn, timeout=3.0)
            await conn.close()
            return True
        except Exception:
            return False
    return asyncio.run(_check())


@pytest.fixture
async def pool(db_dsn: str, db_reachable: bool):
    """
    Fresh asyncpg pool per test — resets the global singleton before/after.
    Creates schema if needed.
    """
    if not db_reachable:
        pytest.skip("PostgreSQL not reachable")

    import cynic.core.storage.postgres as pg_module

    # Reset singleton so this test owns its own pool in the current event loop
    pg_module._pool = None
    p = await pg_module.get_pool(dsn=db_dsn)
    await pg_module.create_schema()

    yield p

    await pg_module.close_pool()


# ════════════════════════════════════════════════════════════════════════════
# Connection + Schema
# ════════════════════════════════════════════════════════════════════════════

class TestConnection:
    async def test_pool_connects(self, pool):
        """asyncpg pool is alive and can acquire connections."""
        async with pool.acquire() as conn:
            result = await conn.fetchval("SELECT 1")
        assert result == 1

    async def test_schema_tables_exist(self, pool):
        """All 7 schema tables created by create_schema()."""
        async with pool.acquire() as conn:
            rows = await conn.fetch(
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public'"
            )
        tables = {r["tablename"] for r in rows}
        expected = {
            "judgments", "cells", "learning_events",
            "q_table", "e_scores", "llm_benchmarks", "consciousness_snapshots",
        }
        assert expected <= tables, f"Missing tables: {expected - tables}"


# ════════════════════════════════════════════════════════════════════════════
# Q-Table Repository — flush + load round-trip
# ════════════════════════════════════════════════════════════════════════════

class TestQTableRepository:
    async def test_get_missing_returns_zero(self, pool):
        from cynic.core.storage.postgres import qtable
        repo = qtable()
        val = await repo.get("nonexistent_state_" + uuid.uuid4().hex, "WAG")
        assert val == 0.0

    async def test_update_and_get_roundtrip(self, pool):
        from cynic.core.storage.postgres import qtable
        repo = qtable()
        state = "integration_test_state_" + uuid.uuid4().hex
        await repo.update(state, "HOWL", 0.75)
        val = await repo.get(state, "HOWL")
        assert abs(val - 0.75) < 0.001

        # Cleanup
        async with pool.acquire() as conn:
            await conn.execute("DELETE FROM q_table WHERE state_key = $1", state)

    async def test_update_increments_visit_count(self, pool):
        from cynic.core.storage.postgres import qtable
        repo = qtable()
        state = "visit_count_test_" + uuid.uuid4().hex
        await repo.update(state, "WAG", 0.5)
        await repo.update(state, "WAG", 0.6)  # second call → visit_count = 2
        async with pool.acquire() as conn:
            row = await conn.fetchrow(
                "SELECT visit_count FROM q_table WHERE state_key=$1 AND action='WAG'",
                state,
            )
        assert row["visit_count"] == 2

        async with pool.acquire() as conn:
            await conn.execute("DELETE FROM q_table WHERE state_key = $1", state)

    async def test_get_all_actions(self, pool):
        from cynic.core.storage.postgres import qtable
        repo = qtable()
        state = "all_actions_test_" + uuid.uuid4().hex
        await repo.update(state, "HOWL", 0.8)
        await repo.update(state, "WAG", 0.5)
        await repo.update(state, "GROWL", 0.3)

        actions = await repo.get_all_actions(state)
        assert set(actions.keys()) == {"HOWL", "WAG", "GROWL"}
        assert abs(actions["HOWL"] - 0.8) < 0.001

        async with pool.acquire() as conn:
            await conn.execute("DELETE FROM q_table WHERE state_key = $1", state)


# ════════════════════════════════════════════════════════════════════════════
# QTable warm-load via qlearning.py — the real thing test_warmload.py mocks
# ════════════════════════════════════════════════════════════════════════════

class TestQTableWarmLoad:
    async def test_load_from_db_with_real_pool(self, pool):
        """QTable.load_from_db(pool) loads entries seeded in DB."""
        from cynic.learning.qlearning import QTable

        state = "warmload_real_test_" + uuid.uuid4().hex

        # Seed DB directly
        async with pool.acquire() as conn:
            await conn.execute(
                """INSERT INTO q_table (state_key, action, q_value, visit_count)
                   VALUES ($1, 'HOWL', 0.72, 10)
                   ON CONFLICT (state_key, action) DO UPDATE SET q_value = 0.72""",
                state,
            )

        # Warm-load into a fresh QTable
        qtable = QTable()
        count = await qtable.load_from_db(pool)

        assert count >= 1
        # Entry should be loaded
        assert qtable._q_table[state]["HOWL"].value == pytest.approx(0.72, abs=0.01)

        # Cleanup
        async with pool.acquire() as conn:
            await conn.execute("DELETE FROM q_table WHERE state_key = $1", state)

    async def test_load_is_additive_not_destructive(self, pool):
        """Warm-load adds to existing Q-table — doesn't wipe in-memory entries."""
        from cynic.learning.qlearning import QTable

        qtable = QTable()
        # Pre-populate in memory
        qtable.update("pre_existing_state", "WAG", reward=0.6)

        state = "additive_test_" + uuid.uuid4().hex
        async with pool.acquire() as conn:
            await conn.execute(
                """INSERT INTO q_table (state_key, action, q_value, visit_count)
                   VALUES ($1, 'BARK', 0.2, 3)
                   ON CONFLICT (state_key, action) DO UPDATE SET q_value = 0.2""",
                state,
            )

        await qtable.load_from_db(pool)

        # Both entries coexist
        assert "pre_existing_state" in qtable._q_table
        assert state in qtable._q_table

        async with pool.acquire() as conn:
            await conn.execute("DELETE FROM q_table WHERE state_key = $1", state)

    async def test_flush_to_db_writes_real_data(self, pool):
        """QTable.flush_to_db(pool) persists in-memory updates to DB."""
        from cynic.learning.qlearning import QTable

        state = "flush_test_" + uuid.uuid4().hex
        qtable = QTable()
        qtable.update(state, "HOWL", reward=0.9)

        flushed = await qtable.flush_to_db(pool)
        assert flushed >= 1

        # Verify persisted
        async with pool.acquire() as conn:
            row = await conn.fetchrow(
                "SELECT q_value FROM q_table WHERE state_key=$1 AND action='HOWL'",
                state,
            )
        assert row is not None
        assert row["q_value"] == pytest.approx(0.9, abs=0.05)

        async with pool.acquire() as conn:
            await conn.execute("DELETE FROM q_table WHERE state_key = $1", state)

    async def test_flush_then_load_round_trip(self, pool):
        """Flush → close QTable → fresh QTable → load → same values."""
        from cynic.learning.qlearning import QTable

        state = "roundtrip_test_" + uuid.uuid4().hex

        # QTable 1: write and flush
        qt1 = QTable()
        qt1.update(state, "WAG", reward=0.65)
        await qt1.flush_to_db(pool)

        # QTable 2: start cold, warm-load
        qt2 = QTable()
        await qt2.load_from_db(pool)

        assert state in qt2._q_table
        loaded_val = qt2._q_table[state]["WAG"].value
        expected_val = qt1._q_table[state]["WAG"].value
        assert abs(loaded_val - expected_val) < 0.05

        async with pool.acquire() as conn:
            await conn.execute("DELETE FROM q_table WHERE state_key = $1", state)


# ════════════════════════════════════════════════════════════════════════════
# Judgment Repository
# ════════════════════════════════════════════════════════════════════════════

class TestJudgmentRepository:
    def _make_judgment(self) -> dict:
        return {
            "judgment_id": "test_" + uuid.uuid4().hex,
            "cell_id": "cell_" + uuid.uuid4().hex,
            "reality": "CODE",
            "analysis": "JUDGE",
            "time_dim": "PRESENT",
            "lod": 1,
            "consciousness": 0,
            "q_score": 42.0,
            "verdict": "WAG",
            "confidence": 0.45,
            "axiom_scores": {},
            "active_axioms": [],
            "dog_votes": {},
            "consensus_votes": 5,
            "consensus_quorum": 7,
            "consensus_reached": False,
            "cost_usd": 0.0,
            "llm_calls": 7,
            "llm_tokens": 210,
            "residual_variance": 0.1,
            "unnameable_detected": False,
            "duration_ms": 350.0,
        }

    async def test_save_and_get_roundtrip(self, pool):
        from cynic.core.storage.postgres import judgments
        repo = judgments()
        j = self._make_judgment()

        await repo.save(j)
        loaded = await repo.get(j["judgment_id"])

        assert loaded is not None
        assert loaded["judgment_id"] == j["judgment_id"]
        assert loaded["verdict"] == "WAG"
        assert abs(loaded["q_score"] - 42.0) < 0.01
        assert abs(loaded["confidence"] - 0.45) < 0.001

        async with pool.acquire() as conn:
            await conn.execute(
                "DELETE FROM judgments WHERE judgment_id = $1", j["judgment_id"]
            )

    async def test_get_nonexistent_returns_none(self, pool):
        from cynic.core.storage.postgres import judgments
        repo = judgments()
        result = await repo.get("nonexistent_" + uuid.uuid4().hex)
        assert result is None

    async def test_recent_returns_saved_judgment(self, pool):
        from cynic.core.storage.postgres import judgments
        repo = judgments()
        j = self._make_judgment()
        await repo.save(j)

        recent = await repo.recent(reality="CODE", limit=55)
        ids = [r["judgment_id"] for r in recent]
        assert j["judgment_id"] in ids

        async with pool.acquire() as conn:
            await conn.execute(
                "DELETE FROM judgments WHERE judgment_id = $1", j["judgment_id"]
            )

    async def test_stats_includes_verdict(self, pool):
        from cynic.core.storage.postgres import judgments
        repo = judgments()
        j = self._make_judgment()
        await repo.save(j)

        stats = await repo.stats()
        assert stats["total"] >= 1
        assert "WAG" in stats["by_verdict"]

        async with pool.acquire() as conn:
            await conn.execute(
                "DELETE FROM judgments WHERE judgment_id = $1", j["judgment_id"]
            )


# ════════════════════════════════════════════════════════════════════════════
# Learning Repository
# ════════════════════════════════════════════════════════════════════════════

class TestLearningRepository:
    async def test_save_and_retrieve_event(self, pool):
        from cynic.core.storage.postgres import learning
        repo = learning()
        event_id = "evt_" + uuid.uuid4().hex
        state = "state_" + uuid.uuid4().hex

        event = {
            "event_id": event_id,
            "loop_name": "q_learning",
            "judgment_id": None,
            "state_key": state,
            "action": "HOWL",
            "reward": 0.8,
            "q_delta": 0.05,
        }
        await repo.save(event)

        recent = await repo.recent_for_loop("q_learning", limit=55)
        ids = [r["event_id"] for r in recent]
        assert event_id in ids

        async with pool.acquire() as conn:
            await conn.execute(
                "DELETE FROM learning_events WHERE event_id = $1", event_id
            )

    async def test_loop_stats_counts_events(self, pool):
        from cynic.core.storage.postgres import learning
        repo = learning()
        loop_name = "test_loop_" + uuid.uuid4().hex[:8]
        event_ids = []

        for i in range(3):
            eid = "evt_" + uuid.uuid4().hex
            event_ids.append(eid)
            await repo.save({
                "event_id": eid,
                "loop_name": loop_name,
                "judgment_id": None,
                "state_key": f"state_{i}",
                "action": "WAG",
                "reward": 0.5,
                "q_delta": 0.0,
            })

        stats = await repo.loop_stats()
        assert stats.get(loop_name, 0) == 3

        async with pool.acquire() as conn:
            for eid in event_ids:
                await conn.execute(
                    "DELETE FROM learning_events WHERE event_id = $1", eid
                )
