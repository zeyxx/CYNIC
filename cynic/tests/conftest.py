"""
CYNIC Test Configuration

LAW #1: NO MOCKS. Tests use real PostgreSQL (via pytest-docker or env var).

Setup:
  export CYNIC_DATABASE_URL=postgresql://cynic:cynic@localhost:5432/cynic_py
  pytest tests/

Pure-Python tests (test_phi.py, test_consciousness.py, test_judgment_models.py)
run WITHOUT any database. Only test_storage.py and integration tests require DB.

All tests are async (asyncio_mode = "auto" in pyproject.toml).
"""
from __future__ import annotations

import asyncio
import os
import pytest

from cynic.core.consciousness import reset_consciousness
from cynic.core.event_bus import reset_all_buses


# ────────────────────────────────────────────────────────────────────────────
# DATABASE FIXTURES (opt-in, not autouse)
# ────────────────────────────────────────────────────────────────────────────

@pytest.fixture(scope="session")
def test_db_url() -> str:
    """Database URL for tests. Must be real PostgreSQL."""
    return (
        os.environ.get("CYNIC_DATABASE_URL")
        or os.environ.get("DATABASE_URL")
        or "postgresql://cynic:cynic@localhost:5432/cynic_py"
    )


@pytest.fixture(scope="session")
def db_available(test_db_url: str) -> bool:
    """Check if database is reachable (synchronously). Skip DB tests if not."""
    try:
        import asyncio
        import asyncpg

        async def _check() -> bool:
            try:
                conn = await asyncpg.connect(dsn=test_db_url, timeout=3.0)
                await conn.close()
                return True
            except ValidationError:
                return False

        return asyncio.run(_check())
    except ImportError:
        return False


@pytest.fixture(scope="session")
def ensure_db(db_available: bool, test_db_url: str) -> None:
    """
    Create schema once per session. Skip if DB not reachable.
    Must be explicitly requested by DB tests (not autouse).
    """
    if not db_available:
        pytest.skip("PostgreSQL not available — set CYNIC_DATABASE_URL to run DB tests")

    async def _setup() -> None:
        from cynic.core.storage.postgres import get_pool, create_schema
        await get_pool(dsn=test_db_url)
        await create_schema()

    asyncio.run(_setup())


# ────────────────────────────────────────────────────────────────────────────
# RESET SINGLETONS (autouse — runs for every test)
# ────────────────────────────────────────────────────────────────────────────

@pytest.fixture(autouse=True)
def reset_singletons() -> None:
    """Reset all singletons before each test (isolation). Synchronous."""
    reset_consciousness()
    reset_all_buses()
    yield
    reset_consciousness()
    reset_all_buses()


# ────────────────────────────────────────────────────────────────────────────
# CELL FACTORIES
# ────────────────────────────────────────────────────────────────────────────

@pytest.fixture
def code_cell():
    """A CODE × JUDGE cell for testing."""
    from cynic.core.judgment import Cell
    return Cell(
        reality="CODE",
        analysis="JUDGE",
        content={"file": "test.py", "changes": 42, "complexity": 0.6},
        context="Python file with 42 changes, moderate complexity",
        novelty=0.3,
        complexity=0.6,
        risk=0.2,
        budget_usd=0.1,
    )


@pytest.fixture
def high_risk_cell():
    """A high-risk cell for GUARDIAN testing."""
    from cynic.core.judgment import Cell
    return Cell(
        reality="CODE",
        analysis="JUDGE",
        content={"sql": "DROP TABLE users", "exec": True},
        context="Suspicious SQL with DROP TABLE",
        novelty=0.9,
        complexity=0.4,
        risk=0.95,
        budget_usd=0.1,
    )


@pytest.fixture
def market_cell():
    """A MARKET × PERCEIVE cell for price feed testing."""
    from cynic.core.judgment import Cell
    return Cell(
        reality="MARKET",
        analysis="PERCEIVE",
        content={"price_usd": 0.000042, "volume_24h": 150000, "market_cap": 420000},
        context="$asdfasdfa price tick from DexScreener",
        novelty=0.2,
        complexity=0.3,
        risk=0.5,
        budget_usd=0.05,
    )
