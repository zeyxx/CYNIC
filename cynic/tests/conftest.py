"""Test configuration and fixtures for CYNIC tests."""
from __future__ import annotations

import pytest
import pytest_asyncio
import asyncio
import uuid
from unittest.mock import AsyncMock, patch
from fastapi.testclient import TestClient


@pytest.fixture(autouse=True)
def mock_llm_discovery():
    """Mock LLM discovery to prevent Ollama timeout hangs in tests.

    Autouse=True: Applies to all tests automatically.
    Prevents 5-30 second hangs when Ollama isn't running.
    """
    from unittest.mock import MagicMock
    mock_registry = MagicMock()
    mock_registry.discover = AsyncMock(return_value=[])  # Async method, no LLMs
    mock_registry.get_available.return_value = []  # Sync method, empty list

    with patch("cynic.llm.adapter.get_registry", return_value=mock_registry):
        yield


@pytest.fixture(autouse=True)
def skip_mcp_server_in_tests():
    """Skip MCP server startup/shutdown in tests to prevent hang.

    Autouse=True: Applies to all tests automatically.
    MCP server spawns port 8766 and hangs on teardown between tests.
    """
    import os
    old_env = os.environ.get("CYNIC_SKIP_MCP_SERVER")
    os.environ["CYNIC_SKIP_MCP_SERVER"] = "1"
    yield
    if old_env is None:
        del os.environ["CYNIC_SKIP_MCP_SERVER"]
    else:
        os.environ["CYNIC_SKIP_MCP_SERVER"] = old_env


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
    from cynic.organism.organism import awaken
    from cynic.api.state import AppContainer, set_app_container, restore_state
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


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION TEST FIXTURES FOR GOVERNANCE BOT
# ════════════════════════════════════════════════════════════════════════════


@pytest.fixture
def mock_discord_client():
    """Mock Discord bot client for integration tests.

    Provides a mock Discord client with essential attributes:
    - user: Bot identity
    - guilds: List of servers the bot is in
    - get_channel: Method to retrieve channels
    - http.token: Authentication token
    """
    from unittest.mock import MagicMock

    client = AsyncMock()
    client.user = MagicMock()
    client.user.name = "TestBot"
    client.guilds = []
    client.get_channel = AsyncMock(return_value=MagicMock())
    return client


@pytest.fixture
def mock_cynic_service():
    """Mock CYNIC service for integration tests.

    Provides a mock CYNIC service that returns:
    - judge: Returns verdict, confidence, and reasoning
    - learn: Records learning outcomes
    """
    service = AsyncMock()
    service.judge = AsyncMock(return_value={
        "verdict": "WAG",
        "confidence": 0.5,
        "reasoning": "Test judgment"
    })
    service.learn = AsyncMock(return_value={"success": True})
    return service


@pytest_asyncio.fixture
async def test_bot(mock_discord_client):
    """Integration test bot instance.

    Creates a minimal Discord bot for testing command handlers
    and bot-level functionality. Uses mock Discord client internally.
    """
    from discord.ext import commands
    import discord

    intents = discord.Intents.default()
    bot = commands.Bot(command_prefix="!", intents=intents)
    # Inject mock client
    bot._connection = mock_discord_client
    bot.http.token = "test_token"
    return bot


@pytest.fixture
def mock_database():
    """Mock database with transaction support.

    Provides:
    - db_mock: Database connection with session_context and health checks
    - db_health: Separate health check module

    Useful for testing database operations without real DB connections.
    """
    db_mock = AsyncMock()
    db_mock.session_context = AsyncMock()
    db_mock.verify_data_consistency = AsyncMock(return_value={"status": "healthy"})

    db_health = AsyncMock()
    db_health.check_health = AsyncMock(return_value={"status": "healthy"})

    return db_mock, db_health
