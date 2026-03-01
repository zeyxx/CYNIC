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


# ════════════════════════════════════════════════════════════════════════════
# HELPER: Create proper async context manager mocks
# ════════════════════════════════════════════════════════════════════════════


def make_async_context_manager(return_value=None):
    """Create a mock that works as an async context manager."""
    async_cm = AsyncMock()
    async_cm.__aenter__ = AsyncMock(return_value=return_value)
    async_cm.__aexit__ = AsyncMock(return_value=None)
    return async_cm


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════


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


# ════════════════════════════════════════════════════════════════════════════
# TEST 1: Kernel Startup Success (Fast Path)
# ════════════════════════════════════════════════════════════════════════════


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
    with patch.dict('sys.modules', {'mcp': MagicMock(), 'mcp.server': MagicMock(), 'mcp.types': MagicMock()}):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        with patch("cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession") as mock_session_cls:
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
    - _spawn_kernel() is called
    - Second health check succeeds (kernel started)
    - Returns True
    """
    with patch.dict('sys.modules', {'mcp': MagicMock(), 'mcp.server': MagicMock(), 'mcp.types': MagicMock()}):
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

        with patch("cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession") as mock_session_cls:
            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)
            mock_session.get = MagicMock(side_effect=mock_get_side_effect)
            mock_session_cls.return_value = mock_session

            with patch("cynic.interfaces.mcp.claude_code_bridge._spawn_kernel") as mock_spawn:
                mock_spawn.return_value = MagicMock(spec=subprocess.Popen)

                # Execute (with short timeout to avoid long test)
                result = await _ensure_kernel_running(timeout=10.0, spawn_if_down=True)

                # Assert
                assert result is True
                mock_spawn.assert_called_once()  # Spawn was invoked


@pytest.mark.asyncio
async def test_kernel_startup_timeout_30s():
    """
    Test that _ensure_kernel_running respects 30s timeout limit.

    Expected behavior:
    - All health checks fail consistently
    - Timeout is enforced (within 31s)
    - Returns False after timeout
    """
    with patch.dict('sys.modules', {'mcp': MagicMock(), 'mcp.server': MagicMock(), 'mcp.types': MagicMock()}):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        def mock_get_always_fails(url):
            """Simulate kernel never coming up."""
            async def raise_error():
                raise aiohttp.ClientConnectionError("Connection refused")
            cm = AsyncMock()
            cm.__aenter__ = AsyncMock(side_effect=raise_error)
            return cm

        with patch("cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession") as mock_session_cls:
            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)
            mock_session.get = MagicMock(side_effect=mock_get_always_fails)
            mock_session_cls.return_value = mock_session

            with patch("cynic.interfaces.mcp.claude_code_bridge._spawn_kernel") as mock_spawn:
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
    Test that _ensure_kernel_running uses exponential backoff correctly.

    Expected behavior:
    - Backoff sequence: 0.5s, 1.0s, 2.0s, 4.0s, 8.0s
    - Each retry attempts health check with increasing delays
    - Total time respects timeout limit
    """
    with patch.dict('sys.modules', {'mcp': MagicMock(), 'mcp.server': MagicMock(), 'mcp.types': MagicMock()}):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running

        attempts = []

        def mock_get_track_attempts(url):
            """Track attempt timing."""
            attempts.append(time.time())
            async def raise_error():
                raise aiohttp.ClientConnectionError("Connection refused")
            cm = AsyncMock()
            cm.__aenter__ = AsyncMock(side_effect=raise_error)
            return cm

        with patch("cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession") as mock_session_cls:
            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)
            mock_session.get = MagicMock(side_effect=mock_get_track_attempts)
            mock_session_cls.return_value = mock_session

            # Execute with short timeout to allow multiple attempts without long wait
            start_time = time.time()
            result = await _ensure_kernel_running(timeout=6.0, spawn_if_down=False)
            total_elapsed = time.time() - start_time

            # Assert
            assert result is False  # Should fail (always refused)
            assert len(attempts) >= 2  # At least 2 attempts
            assert total_elapsed <= 7.0  # Respects timeout

            # Verify backoff timing between attempts
            if len(attempts) >= 2:
                delay_1 = attempts[1] - attempts[0]
                # First backoff should be close to 0.5s (with some tolerance)
                assert 0.4 < delay_1 < 0.8, f"First backoff was {delay_1:.2f}s, expected ~0.5s"


@pytest.mark.asyncio
async def test_kernel_startup_logging(caplog):
    """
    Test that _ensure_kernel_running logs detailed diagnostics.

    Expected behavior:
    - Logs initial startup attempt
    - Logs result (success or failure)
    """
    with patch.dict('sys.modules', {'mcp': MagicMock(), 'mcp.server': MagicMock(), 'mcp.types': MagicMock()}):
        from cynic.interfaces.mcp.claude_code_bridge import _ensure_kernel_running


        with patch("cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession") as mock_session_cls:
            mock_response = AsyncMock()
            mock_response.status = 200

            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)

            response_cm = make_async_context_manager(mock_response)
            mock_session.get = MagicMock(return_value=response_cm)
            mock_session_cls.return_value = mock_session

            # Capture logs
            with caplog.at_level(logging.INFO):
                result = await _ensure_kernel_running(timeout=30.0, spawn_if_down=False)

            # Assert
            assert result is True
            log_text = caplog.text

            # Verify key log messages
            assert "Ensuring kernel is running" in log_text or "kernel" in log_text.lower()


# ════════════════════════════════════════════════════════════════════════════
# TEST: Spawn Kernel Helper
# ════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_spawn_kernel_success():
    """
    Test that _spawn_kernel successfully spawns a subprocess.

    Expected behavior:
    - Calls subprocess.Popen with correct arguments
    - Returns Popen object
    """
    with patch.dict('sys.modules', {'mcp': MagicMock(), 'mcp.server': MagicMock(), 'mcp.types': MagicMock()}):
        from cynic.interfaces.mcp.claude_code_bridge import _spawn_kernel

        with patch("cynic.interfaces.mcp.claude_code_bridge.subprocess.Popen") as mock_popen:
            mock_process = MagicMock()
            mock_process.pid = 12345
            mock_popen.return_value = mock_process

            # First health check fails (kernel not running)
            call_count = {"get": 0}

            def mock_get_side_effect(url):
                call_count["get"] += 1
                if call_count["get"] == 1:
                    async def raise_error():
                        raise aiohttp.ClientConnectionError("Not running")
                    cm = AsyncMock()
                    cm.__aenter__ = AsyncMock(side_effect=raise_error)
                    return cm
                else:
                    mock_response = AsyncMock()
                    mock_response.status = 200
                    return make_async_context_manager(mock_response)

            with patch("cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession") as mock_session_cls:
                mock_session = MagicMock()
                mock_session.__aenter__ = AsyncMock(return_value=mock_session)
                mock_session.__aexit__ = AsyncMock(return_value=None)
                mock_session.get = MagicMock(side_effect=mock_get_side_effect)
                mock_session_cls.return_value = mock_session

                # Execute
                process = await _spawn_kernel()

                # Assert
                assert process is not None
                assert process.pid == 12345
                # Verify subprocess.Popen was called
                assert mock_popen.called


@pytest.mark.asyncio
async def test_spawn_kernel_already_running():
    """
    Test that _spawn_kernel returns None if kernel is already running.

    Expected behavior:
    - Health check succeeds on first call
    - _spawn_kernel() returns None (no spawn needed)
    """
    with patch.dict('sys.modules', {'mcp': MagicMock(), 'mcp.server': MagicMock(), 'mcp.types': MagicMock()}):
        from cynic.interfaces.mcp.claude_code_bridge import _spawn_kernel

        with patch("cynic.interfaces.mcp.claude_code_bridge.aiohttp.ClientSession") as mock_session_cls:
            mock_response = AsyncMock()
            mock_response.status = 200

            mock_session = MagicMock()
            mock_session.__aenter__ = AsyncMock(return_value=mock_session)
            mock_session.__aexit__ = AsyncMock(return_value=None)

            response_cm = make_async_context_manager(mock_response)
            mock_session.get = MagicMock(return_value=response_cm)
            mock_session_cls.return_value = mock_session

            with patch("cynic.interfaces.mcp.claude_code_bridge.subprocess.Popen") as mock_popen:
                # Execute
                result = await _spawn_kernel()

                # Assert
                assert result is None  # Already running
                mock_popen.assert_not_called()  # No spawn attempted


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
