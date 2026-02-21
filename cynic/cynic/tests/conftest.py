"""Test configuration and fixtures for CYNIC tests."""
from __future__ import annotations

import pytest
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
