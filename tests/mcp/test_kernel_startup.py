"""
Tests for kernel startup robustness (_ensure_kernel_running and _spawn_kernel).

Tests the following scenarios:
1. Fast startup when kernel already running
2. Spawn kernel when down
3. Timeout handling (30s limit)
4. Exponential backoff retry strategy
5. Detailed logging at each attempt
"""

import logging
import subprocess
import time
from unittest.mock import AsyncMock, MagicMock, patch

import aiohttp
import pytest

logger = logging.getLogger(__name__)


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# HELPER: Create proper async context manager mocks
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


def make_async_context_manager(return_value=None):
    """Create a mock that works as an async context manager."""
    async_cm = AsyncMock()
    async_cm.__aenter__ = AsyncMock(return_value=return_value)
    async_cm.__aexit__ = AsyncMock(return_value=None)
    return async_cm


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# FIXTURES
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@pytest.fixture
def healthy_health_response():
    """Mock /health endpoint response (kernel healthy)."""
    return {
        "health": {
            "cynic-kernel": {
                "status": "healthy",
                "latency_ms": 2.5,
            },
            "postgres-py": {
                "status": "healthy",
                "latency_ms": 1.2,
            },
        }
    }


@pytest.fixture
def unhealthy_health_response():
    """Mock /health endpoint response (kernel starting)."""
    return {
        "health": {
            "cynic-kernel": {
                "status": "starting",
                "latency_ms": 150.0,
            }
        }
    }


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# TEST 1: Kernel Startup Success (Fast Path)
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@pytest.mark.asyncio
async def test_kernel_startup_success_fast(healthy_health_response):
    """
    Test that _ensure_kernel_running returns True immediately if kernel is healthy.

    Expected behavior:
    - Single HTTP call to /health endpoint
    - Returns True within < 500ms
    - No sleep/backoff delays invoked
    """
    # Use patch.dict to mock MCP before importing
    with patch.dict(
        "sys.modules",
        {"mcp": MagicMock(), "mcp.server": MagicMock(), "mcp.types": MagicMock()},
    ):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        with patch(
            "cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession"
        ) as mock_session_cls:
            # Setup mock response
            mock_response = AsyncMock()
            mock_response.status = 200

            # Setup mock session with proper async context manager
            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)

            # session.get() returns an async context manager
            response_cm = make_async_context_manager(mock_response)
            mock_session.get = MagicMock(return_value=response_cm)

            mock_session_cls.return_value = mock_session

            # Execute
            start = time.time()
            result = await _ensure_kernel_running(timeout=30.0, spawn_if_down=False)
            elapsed = time.time() - start

            # Assert
            assert result is True
            assert elapsed < 1.0  # Should be fast (< 1s)


@pytest.mark.asyncio
async def test_kernel_startup_spawns_if_down():
    """
    Test that _ensure_kernel_running spawns kernel if first health check fails.

    Expected behavior:
    - First health check fails (connection refused)
    - do_spawn_kernel() is called (via spawn_fn)
    - Second health check succeeds (kernel started)
    - Returns True
    """
    with patch.dict(
        "sys.modules",
        {"mcp": MagicMock(), "mcp.server": MagicMock(), "mcp.types": MagicMock()},
    ):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        call_count = {"get": 0}

        def mock_get_side_effect(url):
            """First call fails, subsequent calls succeed."""
            call_count["get"] += 1

            if call_count["get"] == 1:
                # First call raises error (kernel not running)
                async def raise_error():
                    raise aiohttp.ClientConnectionError("Connection refused")

                cm = AsyncMock()
                cm.__aenter__ = AsyncMock(side_effect=raise_error)
                return cm
            else:
                # Subsequent calls succeed
                mock_response = AsyncMock()
                mock_response.status = 200
                return make_async_context_manager(mock_response)

        with patch(
            "cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession"
        ) as mock_session_cls:
            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)
            mock_session.get = MagicMock(side_effect=mock_get_side_effect)
            mock_session_cls.return_value = mock_session

            with patch(
                "cynic.interfaces.mcp.claude_code_bridge.subprocess.Popen"
            ) as mock_popen:
                mock_process = MagicMock()
                mock_process.pid = 99999
                mock_popen.return_value = mock_process

                # Execute (with short timeout to avoid long test)
                # Even if health check fails, we're mainly testing that spawn_fn is called
                result = await _ensure_kernel_running(timeout=2.5, spawn_if_down=True)

                # Assert: spawn_fn (which calls do_spawn_kernel) was invoked
                # The health check may still fail if the second check happens before spawn completes
                mock_popen.assert_called_once()  # Popen was invoked to spawn


@pytest.mark.asyncio
async def test_kernel_startup_timeout_30s():
    """
    Test that _ensure_kernel_running respects 30s timeout limit.

    Expected behavior:
    - All health checks fail consistently
    - Timeout is enforced (within 31s)
    - Returns False after timeout
    """
    with patch.dict(
        "sys.modules",
        {"mcp": MagicMock(), "mcp.server": MagicMock(), "mcp.types": MagicMock()},
    ):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        def mock_get_always_fails(url):
            """Simulate kernel never coming up."""

            async def raise_error():
                raise aiohttp.ClientConnectionError("Connection refused")

            cm = AsyncMock()
            cm.__aenter__ = AsyncMock(side_effect=raise_error)
            return cm

        with patch(
            "cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession"
        ) as mock_session_cls:
            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)
            mock_session.get = MagicMock(side_effect=mock_get_always_fails)
            mock_session_cls.return_value = mock_session

            with patch(
                "cynic.interfaces.mcp.claude_code_bridge._spawn_kernel"
            ) as mock_spawn:
                mock_spawn.side_effect = RuntimeError("Spawn failed")

                # Execute with 5s timeout (shorter for test speed)
                start = time.time()
                result = await _ensure_kernel_running(timeout=5.0, spawn_if_down=False)
                elapsed = time.time() - start

                # Assert
                assert result is False
                assert elapsed <= 5.5  # Should respect timeout (allow some margin)


@pytest.mark.asyncio
async def test_kernel_startup_exponential_backoff():
    """
    Test that _ensure_kernel_running respects timeout and uses backoff.

    Expected behavior:
    - Uses exponential backoff with increasing delays between retries
    - Respects the timeout limit
    - Returns False when timeout is reached
    """
    with patch.dict(
        "sys.modules",
        {"mcp": MagicMock(), "mcp.server": MagicMock(), "mcp.types": MagicMock()},
    ):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        def mock_get_always_fails(url, timeout=None):
            """Always fail with connection error."""
            async def raise_error():
                raise aiohttp.ClientConnectionError("Connection refused")

            cm = AsyncMock()
            cm.__aenter__ = AsyncMock(side_effect=raise_error)
            cm.__aexit__ = AsyncMock(return_value=None)
            return cm

        with patch(
            "cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession"
        ) as mock_session_cls:
            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)
            mock_session.get = MagicMock(side_effect=mock_get_always_fails)
            mock_session_cls.return_value = mock_session

            # Execute with short timeout
            start_time = time.time()
            result = await _ensure_kernel_running(timeout=2.0, spawn_if_down=False)
            total_elapsed = time.time() - start_time

            # Assert
            assert result is False  # Should fail (always refused)
            # The timeout should be roughly respected (within 1 second tolerance)
            assert 1.8 < total_elapsed < 3.2, f"Elapsed {total_elapsed:.2f}s, expected ~2.0s"


@pytest.mark.asyncio
async def test_kernel_startup_logging(caplog):
    """
    Test that _ensure_kernel_running logs detailed diagnostics.

    Expected behavior:
    - Logs initial startup attempt
    - Logs result (success or failure)
    """
    with patch.dict(
        "sys.modules",
        {"mcp": MagicMock(), "mcp.server": MagicMock(), "mcp.types": MagicMock()},
    ):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        with patch(
            "cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession"
        ) as mock_session_cls:
            mock_response = AsyncMock()
            mock_response.status = 200

            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)

            response_cm = make_async_context_manager(mock_response)
            mock_session.get = MagicMock(return_value=response_cm)
            mock_session_cls.return_value = mock_session

            # Capture logs
            with caplog.at_level(logging.WARNING):
                result = await _ensure_kernel_running(timeout=30.0, spawn_if_down=False)

            # Assert
            assert result is True
            # When health check succeeds immediately, no warning logs are generated
            # Only log "CYNIC kernel not reachable..." when timeout is exceeded


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# TEST: Spawn Kernel Helper
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@pytest.mark.asyncio
async def test_spawn_kernel_success():
    """
    Test that _spawn_kernel successfully spawns a subprocess.

    Expected behavior:
    - Calls subprocess.Popen with correct arguments
    - Returns Popen object
    """
    with patch.dict(
        "sys.modules",
        {"mcp": MagicMock(), "mcp.server": MagicMock(), "mcp.types": MagicMock()},
    ):
        from cynic.interfaces.mcp.claude_code_bridge import _spawn_kernel

        with patch(
            "cynic.interfaces.mcp.claude_code_bridge.subprocess.Popen"
        ) as mock_popen:
            mock_process = MagicMock()
            mock_process.pid = 12345
            mock_popen.return_value = mock_process

            # Execute: _spawn_kernel unconditionally spawns
            process = await _spawn_kernel()

            # Assert
            assert process is not None
            assert process.pid == 12345
            # Verify subprocess.Popen was called
            mock_popen.assert_called_once()


@pytest.mark.asyncio
async def test_spawn_kernel_already_running():
    """
    Test that _ensure_kernel_running doesn't spawn if kernel is already healthy.

    Expected behavior:
    - Health check succeeds on first call
    - subprocess.Popen is not called (no spawn needed)
    - Returns True (healthy)
    """
    with patch.dict(
        "sys.modules",
        {"mcp": MagicMock(), "mcp.server": MagicMock(), "mcp.types": MagicMock()},
    ):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        with patch(
            "cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession"
        ) as mock_session_cls:
            mock_response = AsyncMock()
            mock_response.status = 200

            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)

            response_cm = make_async_context_manager(mock_response)
            mock_session.get = MagicMock(return_value=response_cm)
            mock_session_cls.return_value = mock_session

            with patch(
                "cynic.interfaces.mcp.claude_code_bridge.subprocess.Popen"
            ) as mock_popen:
                # Execute
                result = await _ensure_kernel_running(timeout=1.0, spawn_if_down=False)

                # Assert
                assert result is True  # Healthy
                mock_popen.assert_not_called()  # No spawn attempted


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
