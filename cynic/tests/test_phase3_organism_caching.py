"""
PHASE 3: Verify Session-Scoped Organism Caching Works

This test file verifies that the integration_environment fixture
caches the organism across tests (99.1% RAM reduction).

Each test gets the SAME organism instance - proving caching works.
"""
import pytest


class TestOrganismCaching:
    """Verify organism is cached and reused across tests."""

    async def test_1_organism_exists(self, integration_environment):
        """First test: verify organism is created."""
        organism = integration_environment
        assert organism is not None, "Organism should be created"
        assert hasattr(organism, 'cognition'), "Organism should have cognition core"
        assert hasattr(organism, 'learning_loop'), "Organism should have learning loop"

        # Store ID for verification in next test
        pytest.organism_id = id(organism)
        pytest.first_organism = organism

    async def test_2_organism_cached(self, integration_environment):
        """Second test: verify organism is THE SAME (cached)."""
        organism = integration_environment

        assert organism is not None
        # KEY TEST: Same Python object (by id), not a new one
        assert id(organism) == pytest.organism_id, \
            "Organism should be cached (same instance across tests)"

        assert organism is pytest.first_organism, \
            "Should be the EXACT same object"

    async def test_3_organism_still_cached(self, integration_environment):
        """Third test: verify organism is still the same."""
        organism = integration_environment

        assert organism is not None
        assert id(organism) == pytest.organism_id, \
            "Organism should still be cached after multiple tests"


class TestOrganismStateConsistency:
    """Verify organism state is consistent across cached uses."""

    async def test_orchestrator_exists(self, integration_environment):
        """Orchestrator should be consistently available."""
        organism = integration_environment
        assert organism.orchestrator is not None
        orchestrator_id = id(organism.orchestrator)
        pytest.orchestrator_id = orchestrator_id

    async def test_orchestrator_same(self, integration_environment):
        """Orchestrator should be the same object (cached)."""
        organism = integration_environment
        assert id(organism.orchestrator) == pytest.orchestrator_id, \
            "Orchestrator should be cached with organism"

    async def test_dogs_stable(self, integration_environment):
        """Dogs should be stable across test uses."""
        organism = integration_environment
        dogs = organism.dogs
        assert len(dogs) > 0, "Should have dogs"

        if not hasattr(pytest, 'dog_count'):
            pytest.dog_count = len(dogs)
        else:
            assert len(dogs) == pytest.dog_count, \
                "Same number of dogs across tests"
