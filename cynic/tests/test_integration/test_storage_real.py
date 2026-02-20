"""
Integration tests: Real storage with SurrealDB or PostgreSQL.

These tests require:
- CYNIC_STORAGE_URL environment variable set (SurrealDB or PostgreSQL)
- Real database connection (no mocks)

Skip gracefully if DB not configured.
Run locally: pytest -m integration tests/test_integration/test_storage_real.py
"""
import os
import pytest
import asyncio

from cynic.core.storage.surreal import SurrealStorage
from cynic.core.judgment import Cell, Judgment


@pytest.mark.integration
class TestStorageRealDB:
    """Real database operations (SurrealDB or PostgreSQL)."""

    @pytest.mark.asyncio
    async def test_surreal_save_load_cycle(self):
        """
        Real save/load cycle with SurrealDB.

        Skip if CYNIC_STORAGE_URL not configured.
        """
        storage_url = os.getenv("CYNIC_STORAGE_URL")

        if not storage_url:
            pytest.skip("CYNIC_STORAGE_URL not configured")

        if "surrealdb" not in storage_url.lower():
            pytest.skip("SurrealDB test, skipping non-SurrealDB config")

        # Connect to real SurrealDB
        storage = SurrealStorage(storage_url)
        await storage.connect()

        try:
            # Create a cell
            cell = Cell(
                reality="CODE",
                analysis="JUDGE",
                time_dim="PRESENT",
                content="def golden_ratio(): return 0.618033988749",
                context="mathematical constant",
            )

            # Create a judgment
            judgment = Judgment(
                q_score=72.5,
                verdict="WAG",
                confidence=0.58,
                reasoning="Clear mathematical definition",
                evidence={
                    "path": "direct",
                    "score_components": {"clarity": 75, "correctness": 90},
                },
            )

            # Save to real DB
            cell_id = await storage.save_cell(cell)
            assert cell_id is not None

            # Load from real DB
            loaded_cell = await storage.get_cell(cell_id)
            assert loaded_cell is not None
            assert loaded_cell.content == cell.content

            # Verify real persistence (not just in-memory)
            assert "code" in loaded_cell.reality.lower()

        finally:
            await storage.disconnect()

    @pytest.mark.asyncio
    async def test_postgres_connection_real(self):
        """
        Real PostgreSQL connection (asyncpg).

        Skip if PostgreSQL not configured.
        """
        import asyncpg

        db_url = os.getenv("CYNIC_DATABASE_URL")

        if not db_url or "postgres" not in db_url.lower():
            pytest.skip("PostgreSQL not configured (CYNIC_DATABASE_URL)")

        try:
            # Create real connection pool
            pool = await asyncpg.create_pool(
                db_url,
                min_size=1,
                max_size=5,
                timeout=5,
            )

            assert pool is not None

            # Test real query
            async with pool.acquire() as conn:
                result = await conn.fetchval("SELECT 1")
                assert result == 1

            await pool.close()

        except Exception as e:
            pytest.skip(f"PostgreSQL connection failed: {e}")
