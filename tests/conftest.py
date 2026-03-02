"""Test configuration and fixtures for CYNIC tests."""
from __future__ import annotations

import uuid
import pytest
import pytest_asyncio
import logging
from fastapi.testclient import TestClient

# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# PHASE 4A: Old Architecture Test Collection (V5 Migration)
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
collect_ignore = [
    "test_authentication.py",
    "test_near_integration_live.py",
    "test_near_rpc_submission.py",
    "test_near_transaction_signing.py",
    "test_proposal_templates.py",
    "test_reputation.py",
    "test_treasury.py",
    "test_voting_mechanics.py",
    "cynic/judges/test_dogs.py",
    "test_config_management.py",
    "test_consciousness.py",
    "test_docker_manager.py",
    "test_learning/test_sona_wiring.py",
    "test_llm/test_llm_registry.py",
    "test_llm/test_ollama_adapter.py",
    "test_state_manager.py",
    "test_track_f_pipeline.py",
    "test_topology_integration.py",
    "test_telemetry_ws.py",
    "cognition/test_sage_dog.py",
]

_CACHED_ORGANISM = None
_CACHED_CONTAINER = None

async def _get_or_create_organism_async():
    """Get cached organism or create new one (async version for test fixture)."""
    global _CACHED_ORGANISM, _CACHED_CONTAINER

    from cynic.interfaces.api.state import AppContainer, restore_state, set_app_container
    from cynic.kernel.organism.organism import awaken

    if _CACHED_ORGANISM is not None:
        return _CACHED_ORGANISM, _CACHED_CONTAINER

    logger = logging.getLogger("cynic.tests.conftest")
    logger.info("ðŸ§¬ SESSION: Creating shared organism (Empirical)")

    # Create organism once
    organism = await awaken(db_pool=None)
    await organism.start()
    
    instance_id = uuid.uuid4().hex[:8]
    container = AppContainer(
        organism=organism,
        instance_id=instance_id,
        guidance_path=f"~/.cynic/guidance-test-{instance_id}.json",
    )
    set_app_container(container)

    # Restore state
    await restore_state(container)

    _CACHED_ORGANISM = organism
    _CACHED_CONTAINER = container
    return organism, container

@pytest_asyncio.fixture(scope="function")
async def organism():
    """Real organism for tests."""
    o, _ = await _get_or_create_organism_async()
    return o

@pytest_asyncio.fixture(scope="function")
async def integration_environment(organism):
    """Fixture for tests needing a real organism environment."""
    yield organism

@pytest.fixture
def test_client() -> TestClient:
    """Real FastAPI client."""
    from cynic.interfaces.api.server import app
    with TestClient(app) as client:
        yield client

@pytest.fixture(autouse=True)
def skip_mcp_server_in_tests():
    import os
    os.environ["CYNIC_SKIP_MCP_SERVER"] = "1"
    yield
