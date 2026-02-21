"""Test configuration and fixtures for CYNIC tests."""
from __future__ import annotations

import pytest
import pytest_asyncio
import asyncio
import uuid
from fastapi.testclient import TestClient


@pytest.fixture
def test_client() -> TestClient:
    """Create a TestClient with the full CYNIC app.

    The lifespan context is explicitly triggered via TestClient
    which will run the full async startup sequence including:
    - AppContainer initialization
    - Organism awakening
    - Auto-registration of routers
    - Scheduler startup
    - Event bus initialization
    """
    # Import app fresh each time (in case state was modified)
    from cynic.api.server import app
    # TestClient will trigger lifespan startup on __enter__
    return TestClient(app)


# ════════════════════════════════════════════════════════════════════════════
# PHASE 3: Real Integration Testing (NO MOCKS)
# ════════════════════════════════════════════════════════════════════════════

@pytest_asyncio.fixture()
async def integration_environment():
    """
    Set up real organism + event buses for integration tests.

    Pattern proven from test_conscious_state.py (18/18 passing):
    - Real EventBus instances (not mocks)
    - Real ConsciousState (singleton)
    - Real Organism with all subsystems
    - Auto-register routers
    - Background scheduler running

    Autouse=True: Runs before every async test automatically.
    No manual fixture injection needed (unless you override it).
    """
    from cynic.api.server import app
    from cynic.api.state import awaken, AppContainer, set_app_container, restore_state
    from cynic.api.routers.auto_register import auto_register_routers

    # Start fresh organism (no database, no LLM calls unless explicitly)
    organism = awaken(db_pool=None)

    # Create AppContainer for dependency injection
    instance_id = uuid.uuid4().hex[:8]
    container = AppContainer(
        organism=organism,
        instance_id=instance_id,
        guidance_path=f"~/.cynic/guidance-test-{instance_id}.json",
    )
    set_app_container(container)

    # Restore ConsciousState subscriptions from all event buses
    # This wires up the handlers that listen to events
    await restore_state(container)

    # Auto-register all routers (important: routers must be registered before app startup)
    auto_register_routers(app)

    yield organism

    # Cleanup: stop background scheduler
    if hasattr(organism, 'learning_loop') and organism.learning_loop:
        organism.learning_loop.stop()
