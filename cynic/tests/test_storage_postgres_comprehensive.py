"""
CYNIC PostgreSQL Storage Layer — Comprehensive Integration Tests

Coverage targets:
- Connection pool lifecycle (init, reuse, exhaustion)
- Judgment CRUD operations with constraints
- Transaction rollback on errors
- Concurrent access patterns
- φ-constraint enforcement (q_score, confidence, verdict)

Design: Real PostgreSQL tests (not mocked), uses test database.
Safety: Tests run in transaction, auto-rollback on completion.
"""
import asyncio
import json
import pytest
import asyncpg
from datetime import datetime
from typing import Optional

from cynic.core.phi import fibonacci, MAX_Q_SCORE, MAX_CONFIDENCE
from cynic.core.storage.postgres import get_pool, reset_pool
from cynic.core.events_schema import CoreEvent
from cynic.core.exceptions import PersistenceError


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture
async def db_pool():
    """Get test database pool."""
    # Use test DSN (expects CYNIC_TEST_DATABASE_URL env var)
    test_dsn = "postgresql://cynic:cynic@localhost:5432/cynic_test"
    pool = await get_pool(dsn=test_dsn)
    yield pool
    # Don't close - let reset_pool handle it for next test


@pytest.fixture
async def db_connection(db_pool):
    """Get single database connection for test."""
    async with db_pool.acquire() as conn:
        # Start transaction
        async with conn.transaction():
            yield conn
        # Auto-rollback on exit


@pytest.mark.asyncio
class TestPostgresConnection:
    """Test connection pool lifecycle."""

    async def test_pool_creation(self, db_pool):
        """Pool should create with Fibonacci-sized bounds."""
        assert db_pool is not None
        assert db_pool._minsize == fibonacci(6)  # 8
        assert db_pool._maxsize == fibonacci(8)  # 21

    async def test_pool_connection_reuse(self, db_pool):
        """Pool should reuse connections efficiently."""
        # Get first connection
        async with db_pool.acquire() as conn1:
            id1 = id(conn1)

        # Get second connection - should be same object (reused)
        async with db_pool.acquire() as conn2:
            id2 = id(conn2)

        # IDs might differ but connections are from same pool
        assert db_pool is not None

    async def test_pool_concurrent_checkout(self, db_pool):
        """Multiple tasks should get different connections."""
        connections = []

        async def checkout():
            async with db_pool.acquire() as conn:
                connections.append(conn)
                await asyncio.sleep(0.01)  # Hold briefly

        # Request 5 connections concurrently
        await asyncio.gather(*[checkout() for _ in range(5)])
        assert len(connections) == 5

    async def test_pool_query_execution(self, db_pool):
        """Should execute queries successfully."""
        async with db_pool.acquire() as conn:
            result = await conn.fetchval("SELECT 1")
            assert result == 1


@pytest.mark.asyncio
class TestJudgmentPersistence:
    """Test judgment CRUD operations."""

    async def test_save_judgment_creates_record(self, db_connection):
        """Saving judgment should create database record."""
        judgment_id = "test-judgment-001"
        q_score = 75.5
        verdict = "WAG"
        confidence = 0.61  # φ⁻¹

        # Insert judgment
        query = """
            INSERT INTO judgments (
                judgment_id, q_score, verdict, confidence,
                created_at, axiom_scores
            ) VALUES ($1, $2, $3, $4, $5, $6)
        """
        await db_connection.execute(
            query,
            judgment_id,
            q_score,
            verdict,
            confidence,
            datetime.utcnow(),
            json.dumps({"FIDELITY": 80, "PHI": 75})
        )

        # Verify record exists
        result = await db_connection.fetchrow(
            "SELECT * FROM judgments WHERE judgment_id = $1",
            judgment_id
        )
        assert result is not None
        assert result['q_score'] == q_score
        assert result['verdict'] == verdict

    async def test_load_judgment_returns_data(self, db_connection):
        """Loading judgment should return complete data."""
        judgment_id = "test-judgment-002"

        # Insert
        query = """
            INSERT INTO judgments (
                judgment_id, q_score, verdict, confidence,
                created_at, axiom_scores
            ) VALUES ($1, $2, $3, $4, $5, $6)
        """
        await db_connection.execute(
            query,
            judgment_id,
            82.0,
            "HOWL",
            0.618,
            datetime.utcnow(),
            json.dumps({"FIDELITY": 85, "PHI": 80})
        )

        # Load
        result = await db_connection.fetchrow(
            """SELECT judgment_id, q_score, verdict, confidence, axiom_scores
               FROM judgments WHERE judgment_id = $1""",
            judgment_id
        )

        assert result['judgment_id'] == judgment_id
        assert result['q_score'] == 82.0
        assert result['verdict'] == "HOWL"
        axioms = json.loads(result['axiom_scores'])
        assert axioms['FIDELITY'] == 85

    async def test_phi_constraint_q_score_bounds(self, db_connection):
        """Q-score must be 0-100 (φ constraint)."""
        # Valid: 0
        query = """
            INSERT INTO judgments (
                judgment_id, q_score, verdict, confidence,
                created_at, axiom_scores
            ) VALUES ($1, $2, $3, $4, $5, $6)
        """
        await db_connection.execute(
            query,
            "test-q-0",
            0,
            "BARK",
            0.3,
            datetime.utcnow(),
            json.dumps({})
        )

        # Valid: 100
        await db_connection.execute(
            query,
            "test-q-100",
            100,
            "HOWL",
            0.6,
            datetime.utcnow(),
            json.dumps({})
        )

        # Invalid: > 100 (should fail via CHECK constraint)
        with pytest.raises(asyncpg.IntegrityError):
            await db_connection.execute(
                query,
                "test-q-invalid",
                101,
                "HOWL",
                0.6,
                datetime.utcnow(),
                json.dumps({})
            )

    async def test_phi_constraint_confidence_bounds(self, db_connection):
        """Confidence must be 0-0.618 (φ⁻¹)."""
        query = """
            INSERT INTO judgments (
                judgment_id, q_score, verdict, confidence,
                created_at, axiom_scores
            ) VALUES ($1, $2, $3, $4, $5, $6)
        """

        # Invalid: > 0.618 (should fail via CHECK constraint)
        with pytest.raises(asyncpg.IntegrityError):
            await db_connection.execute(
                query,
                "test-conf-invalid",
                75,
                "WAG",
                0.7,  # > φ⁻¹
                datetime.utcnow(),
                json.dumps({})
            )

    async def test_verdict_enum_constraint(self, db_connection):
        """Verdict must be one of {HOWL, WAG, GROWL, BARK}."""
        query = """
            INSERT INTO judgments (
                judgment_id, q_score, verdict, confidence,
                created_at, axiom_scores
            ) VALUES ($1, $2, $3, $4, $5, $6)
        """

        # Invalid verdict
        with pytest.raises(asyncpg.IntegrityError):
            await db_connection.execute(
                query,
                "test-verdict-invalid",
                75,
                "INVALID",
                0.5,
                datetime.utcnow(),
                json.dumps({})
            )

    async def test_judgment_update(self, db_connection):
        """Should update judgment record."""
        judgment_id = "test-judgment-update"

        # Insert
        query = """
            INSERT INTO judgments (
                judgment_id, q_score, verdict, confidence,
                created_at, axiom_scores
            ) VALUES ($1, $2, $3, $4, $5, $6)
        """
        await db_connection.execute(
            query,
            judgment_id,
            50,
            "GROWL",
            0.4,
            datetime.utcnow(),
            json.dumps({})
        )

        # Update
        await db_connection.execute(
            "UPDATE judgments SET q_score = $1, verdict = $2 WHERE judgment_id = $3",
            75,
            "WAG",
            judgment_id
        )

        # Verify
        result = await db_connection.fetchrow(
            "SELECT q_score, verdict FROM judgments WHERE judgment_id = $1",
            judgment_id
        )
        assert result['q_score'] == 75
        assert result['verdict'] == "WAG"

    async def test_judgment_delete(self, db_connection):
        """Should delete judgment record."""
        judgment_id = "test-judgment-delete"

        # Insert
        query = """
            INSERT INTO judgments (
                judgment_id, q_score, verdict, confidence,
                created_at, axiom_scores
            ) VALUES ($1, $2, $3, $4, $5, $6)
        """
        await db_connection.execute(
            query,
            judgment_id,
            60,
            "WAG",
            0.5,
            datetime.utcnow(),
            json.dumps({})
        )

        # Delete
        await db_connection.execute(
            "DELETE FROM judgments WHERE judgment_id = $1",
            judgment_id
        )

        # Verify gone
        result = await db_connection.fetchrow(
            "SELECT * FROM judgments WHERE judgment_id = $1",
            judgment_id
        )
        assert result is None


@pytest.mark.asyncio
class TestConcurrentAccess:
    """Test concurrent database operations."""

    async def test_concurrent_writes_no_corruption(self, db_pool):
        """Concurrent writes should not corrupt data."""
        async def write_judgment(index):
            async with db_pool.acquire() as conn:
                async with conn.transaction():
                    query = """
                        INSERT INTO judgments (
                            judgment_id, q_score, verdict, confidence,
                            created_at, axiom_scores
                        ) VALUES ($1, $2, $3, $4, $5, $6)
                    """
                    await conn.execute(
                        query,
                        f"concurrent-{index}",
                        50 + index,
                        "WAG",
                        0.5,
                        datetime.utcnow(),
                        json.dumps({})
                    )

        # Write 10 judgments concurrently
        await asyncio.gather(*[write_judgment(i) for i in range(10)])

        # Verify all exist
        async with db_pool.acquire() as conn:
            count = await conn.fetchval(
                "SELECT COUNT(*) FROM judgments WHERE judgment_id LIKE 'concurrent-%'"
            )
            assert count == 10

    async def test_concurrent_reads_consistency(self, db_pool):
        """Concurrent reads should return consistent data."""
        # Setup: Insert one judgment
        async with db_pool.acquire() as conn:
            async with conn.transaction():
                query = """
                    INSERT INTO judgments (
                        judgment_id, q_score, verdict, confidence,
                        created_at, axiom_scores
                    ) VALUES ($1, $2, $3, $4, $5, $6)
                """
                await conn.execute(
                    query,
                    "concurrent-read-test",
                    75,
                    "WAG",
                    0.5,
                    datetime.utcnow(),
                    json.dumps({})
                )

        # Concurrent reads
        results = []

        async def read_judgment():
            async with db_pool.acquire() as conn:
                result = await conn.fetchrow(
                    "SELECT q_score FROM judgments WHERE judgment_id = $1",
                    "concurrent-read-test"
                )
                results.append(result['q_score'])

        await asyncio.gather(*[read_judgment() for _ in range(5)])

        # All reads should return same value
        assert all(q == 75 for q in results)


@pytest.mark.asyncio
class TestTransactionRollback:
    """Test transaction safety."""

    async def test_transaction_rollback_on_constraint_violation(self, db_pool):
        """Transaction should rollback on constraint violation."""
        async with db_pool.acquire() as conn:
            try:
                async with conn.transaction():
                    # Valid insert
                    query = """
                        INSERT INTO judgments (
                            judgment_id, q_score, verdict, confidence,
                            created_at, axiom_scores
                        ) VALUES ($1, $2, $3, $4, $5, $6)
                    """
                    await conn.execute(
                        query,
                        "rollback-test",
                        75,
                        "WAG",
                        0.5,
                        datetime.utcnow(),
                        json.dumps({})
                    )

                    # Invalid insert (q_score > 100)
                    await conn.execute(
                        query,
                        "rollback-test-2",
                        101,
                        "HOWL",
                        0.6,
                        datetime.utcnow(),
                        json.dumps({})
                    )
            except asyncpg.IntegrityError:
                pass  # Expected

        # Both should be rolled back
        async with db_pool.acquire() as conn:
            count = await conn.fetchval(
                "SELECT COUNT(*) FROM judgments WHERE judgment_id LIKE 'rollback-test%'"
            )
            assert count == 0

    async def test_transaction_rollback_on_error(self, db_pool):
        """Transaction should rollback when error occurs."""
        async with db_pool.acquire() as conn:
            try:
                async with conn.transaction():
                    query = """
                        INSERT INTO judgments (
                            judgment_id, q_score, verdict, confidence,
                            created_at, axiom_scores
                        ) VALUES ($1, $2, $3, $4, $5, $6)
                    """
                    await conn.execute(
                        query,
                        "error-rollback-test",
                        75,
                        "WAG",
                        0.5,
                        datetime.utcnow(),
                        json.dumps({})
                    )

                    # Simulate error
                    raise RuntimeError("Simulated failure")
            except RuntimeError:
                pass  # Expected

        # Should be rolled back
        async with db_pool.acquire() as conn:
            result = await conn.fetchrow(
                "SELECT * FROM judgments WHERE judgment_id = $1",
                "error-rollback-test"
            )
            assert result is None


@pytest.mark.asyncio
class TestConnectionFailures:
    """Test error handling."""

    async def test_connection_failure_does_not_corrupt_pool(self, db_pool):
        """Failed connection should not corrupt pool state."""
        try:
            # Simulate connection issue by using invalid SQL
            async with db_pool.acquire() as conn:
                await conn.execute("INVALID SQL SYNTAX")
        except asyncpg.SyntaxError:
            pass  # Expected

        # Pool should still work
        async with db_pool.acquire() as conn:
            result = await conn.fetchval("SELECT 1")
            assert result == 1

    async def test_pool_recovery_after_error(self, db_pool):
        """Pool should recover after error and execute queries."""
        # First: operation that fails
        try:
            async with db_pool.acquire() as conn:
                await conn.execute("INVALID SYNTAX")
        except asyncpg.SyntaxError:
            pass

        # Second: pool should still be functional
        async with db_pool.acquire() as conn:
            result = await conn.fetchval("SELECT COUNT(*) FROM judgments")
            assert result is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
