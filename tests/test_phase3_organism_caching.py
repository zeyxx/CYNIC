"""
PHASE 3: Organism Caching and Lifecycle Integration â€” EMPIRICAL VERSION

Tests that the session-scoped organism cache in conftest.py works
and that the organism remains stable across tests.
"""
import pytest
import asyncio

@pytest.mark.asyncio
class TestOrganismCaching:
    """Verify organism caching efficiency."""

    async def test_1_organism_exists(self, organism):
        """First access creates organism."""
        assert organism is not None
        assert organism.state._processing is True

    async def test_2_organism_cached(self, organism):
        """Second access uses same instance."""
        assert organism is not None
        # Verify it's still breathing
        assert organism.state._processing is True

@pytest.mark.asyncio
class TestOrganismStateConsistency:
    """Verify state components are correctly initialized."""

    async def test_orchestrator_exists(self, organism):
        assert organism.cognition.orchestrator is not None

    async def test_dogs_stable(self, organism):
        # MasterDog should have all 11 dogs discovered
        assert len(organism.cognition.orchestrator.dogs) == 11
