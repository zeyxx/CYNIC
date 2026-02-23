"""
INTEGRATION TESTS: Real SurrealDB Connection

Tests actual SurrealDB at ws://localhost:8000.
SKIP in CI (no SurrealDB running). Run locally via:
  py -3.13 -m pytest -m integration tests/test_integration_real_surrealdb.py -v

Requires: SurrealDB running at localhost:8000
  docker-compose up surrealdb
"""
import pytest
import asyncio
import json

pytestmark = pytest.mark.integration


@pytest.fixture
def has_surrealdb():
    """Check if SurrealDB is available at localhost:8000."""
    try:
        import surrealdb
        # Simple check: try to import module (doesn't validate connection yet)
        return True
    except ImportError:
        return False


class TestSurrealDBConnection:
    """Validate real SurrealDB connectivity and persistence."""

    @pytest.mark.asyncio
    async def test_surrealdb_available_for_tests(self, has_surrealdb):
        """Skip entire class if SurrealDB not available."""
        if not has_surrealdb:
            pytest.skip("surrealdb module not installed or SurrealDB not running")

    @pytest.mark.asyncio
    async def test_surrealdb_connection_real(self, has_surrealdb):
        """Test real connection to SurrealDB server."""
        if not has_surrealdb:
            pytest.skip("surrealdb not available")

        try:
            from surrealdb import Surreal
        except ImportError:
            pytest.skip("surrealdb module not installed")

        async with Surreal("ws://localhost:8000") as db:
            await db.signin({"user": "root", "pass": "root"})
            await db.use("cynic", "default")

            # Test basic query
            result = await db.query("RETURN 'connected'")
            assert result, "Should get response from SurrealDB"

            print("✓ Real SurrealDB connection successful")

    @pytest.mark.asyncio
    async def test_surrealdb_create_and_retrieve(self, has_surrealdb):
        """Test creating and retrieving a record in SurrealDB."""
        if not has_surrealdb:
            pytest.skip("surrealdb not available")

        try:
            from surrealdb import Surreal
        except ImportError:
            pytest.skip("surrealdb module not installed")

        async with Surreal("ws://localhost:8000") as db:
            await db.signin({"user": "root", "pass": "root"})
            await db.use("cynic", "test_persistence")

            # Create test judgment record
            test_id = "judgment:test_001"
            test_data = {
                "state_key": "test_key",
                "verdict": "WAG",
                "q_score": 65.5,
                "confidence": 0.550,
                "dog_votes": {"ANALYZER": 65, "GUARDIAN": 66},
                "timestamp": 1708000000,
            }

            # Create
            created = await db.create(test_id, test_data)
            assert created, "Should create record"

            # Retrieve
            retrieved = await db.select(test_id)
            assert len(retrieved) > 0, "Should retrieve created record"

            record = retrieved[0]
            assert record["verdict"] == "WAG", "Should preserve verdict"
            assert record["q_score"] == 65.5, "Should preserve q_score"
            assert record["confidence"] == 0.550, "Should preserve confidence"

            # Delete
            await db.delete(test_id)
            final = await db.select(test_id)
            assert len(final) == 0, "Should be deleted"

            print(f"✓ Real SurrealDB persistence: create/retrieve/delete cycle works")

    @pytest.mark.asyncio
    async def test_surrealdb_judgment_table_schema(self, has_surrealdb):
        """Test creating and querying judgment table with real schema."""
        if not has_surrealdb:
            pytest.skip("surrealdb not available")

        try:
            from surrealdb import Surreal
        except ImportError:
            pytest.skip("surrealdb module not installed")

        async with Surreal("ws://localhost:8000") as db:
            await db.signin({"user": "root", "pass": "root"})
            await db.use("cynic", "test_schema")

            # Create judgment table
            await db.query("""
                DEFINE TABLE judgment SCHEMAFULL;
                DEFINE FIELD state_key ON TABLE judgment TYPE string;
                DEFINE FIELD verdict ON TABLE judgment TYPE string;
                DEFINE FIELD q_score ON TABLE judgment TYPE number;
                DEFINE FIELD confidence ON TABLE judgment TYPE number;
                DEFINE FIELD dog_votes ON TABLE judgment TYPE object;
                DEFINE FIELD timestamp ON TABLE judgment TYPE number;
            """)

            # Insert test judgment
            result = await db.create("judgment:test_schema_001", {
                "state_key": "schema_test",
                "verdict": "HOWL",
                "q_score": 88.5,
                "confidence": 0.618,
                "dog_votes": {"ANALYST": 89, "GUARDIAN": 88},
                "timestamp": 1708000001,
            })

            assert result, "Should create record in schema"

            # Query via SurrealQL
            query_result = await db.query(
                "SELECT * FROM judgment WHERE verdict = 'HOWL'"
            )
            assert len(query_result) > 0, "Should find HOWL judgments"

            print(f"✓ Real SurrealDB schema and queries work")

    @pytest.mark.asyncio
    async def test_surrealdb_vector_search_setup(self, has_surrealdb):
        """Test setting up HNSW vector search in SurrealDB."""
        if not has_surrealdb:
            pytest.skip("surrealdb not available")

        try:
            from surrealdb import Surreal
        except ImportError:
            pytest.skip("surrealdb module not installed")

        async with Surreal("ws://localhost:8000") as db:
            await db.signin({"user": "root", "pass": "root"})
            await db.use("cynic", "test_vectors")

            # Define embedding table with HNSW index
            await db.query("""
                DEFINE TABLE embedding SCHEMAFULL;
                DEFINE FIELD cell_id ON TABLE embedding TYPE string;
                DEFINE FIELD vector ON TABLE embedding TYPE array<float>;
                DEFINE FIELD metadata ON TABLE embedding TYPE object;
                DEFINE INDEX idx_embedding ON TABLE embedding COLUMNS vector SEARCH HNSW DIMENSION 384;
            """)

            # Insert test embedding
            test_vector = [0.1] * 384  # 384-dim vector for semantic search
            result = await db.create("embedding:test_001", {
                "cell_id": "code_001",
                "vector": test_vector,
                "metadata": {"content": "test code snippet"},
            })

            assert result, "Should create embedding"

            print("✓ Real SurrealDB HNSW vector index created")


class TestSurrealDBPerformance:
    """Benchmark real SurrealDB latency and throughput."""

    @pytest.mark.asyncio
    async def test_surrealdb_write_latency(self, has_surrealdb):
        """Measure SurrealDB write latency."""
        if not has_surrealdb:
            pytest.skip("surrealdb not available")

        try:
            from surrealdb import Surreal
        except ImportError:
            pytest.skip("surrealdb module not installed")

        import time

        async with Surreal("ws://localhost:8000") as db:
            await db.signin({"user": "root", "pass": "root"})
            await db.use("cynic", "perf_test")

            start = time.perf_counter()
            await db.create("judgment:perf_001", {
                "state_key": "perf_test",
                "verdict": "WAG",
                "q_score": 50.0,
                "confidence": 0.382,
            })
            write_ms = (time.perf_counter() - start) * 1000

            print(f"✓ SurrealDB write latency: {write_ms:.1f}ms")
            assert write_ms < 100, f"Write too slow: {write_ms:.1f}ms"

    @pytest.mark.asyncio
    async def test_surrealdb_read_latency(self, has_surrealdb):
        """Measure SurrealDB read latency."""
        if not has_surrealdb:
            pytest.skip("surrealdb not available")

        try:
            from surrealdb import Surreal
        except ImportError:
            pytest.skip("surrealdb module not installed")

        import time

        async with Surreal("ws://localhost:8000") as db:
            await db.signin({"user": "root", "pass": "root"})
            await db.use("cynic", "perf_test_read")

            # Create test record first
            await db.create("judgment:perf_read_001", {
                "state_key": "perf_read",
                "verdict": "WAG",
                "q_score": 50.0,
            })

            # Measure read
            start = time.perf_counter()
            result = await db.select("judgment:perf_read_001")
            read_ms = (time.perf_counter() - start) * 1000

            assert len(result) > 0, "Should retrieve record"
            print(f"✓ SurrealDB read latency: {read_ms:.1f}ms")
            assert read_ms < 100, f"Read too slow: {read_ms:.1f}ms"

    @pytest.mark.asyncio
    async def test_surrealdb_batch_writes(self, has_surrealdb):
        """Measure SurrealDB batch write throughput."""
        if not has_surrealdb:
            pytest.skip("surrealdb not available")

        try:
            from surrealdb import Surreal
        except ImportError:
            pytest.skip("surrealdb module not installed")

        import time

        async with Surreal("ws://localhost:8000") as db:
            await db.signin({"user": "root", "pass": "root"})
            await db.use("cynic", "perf_batch")

            # Write 100 records
            start = time.perf_counter()
            for i in range(100):
                await db.create(f"judgment:batch_{i:03d}", {
                    "state_key": f"batch_{i}",
                    "verdict": "WAG",
                    "q_score": 50.0 + i,
                })
            batch_ms = (time.perf_counter() - start) * 1000

            per_record_ms = batch_ms / 100
            print(f"✓ SurrealDB batch writes (100 records): {batch_ms:.1f}ms ({per_record_ms:.1f}ms/record)")
            assert per_record_ms < 50, f"Batch write too slow: {per_record_ms:.1f}ms/record"


if __name__ == "__main__":
    import sys
    # Run with: py -3.13 -m pytest tests/test_integration_real_surrealdb.py -v -m integration
    sys.exit(__import__("pytest").main([__file__, "-v", "-m", "integration"]))
