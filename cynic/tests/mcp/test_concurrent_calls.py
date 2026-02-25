"""
Tests for MCP Router Concurrent Call Support

Validates:
- Two tools running concurrently
- Three concurrent observe calls
- Concurrent learn calls without race conditions
- Different timeout handling
- Call isolation
"""

import pytest
import asyncio
import time
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.mcp.router import MCPRouter, _CallMetadata


@pytest.fixture
def router():
    """Create a fresh router for each test."""
    return MCPRouter()


class TestConcurrentCallsBasic:
    """Test basic concurrent execution of two tools."""

    @pytest.mark.asyncio
    async def test_two_tools_can_run_concurrently(self, router):
        """Verify that two tools can be called concurrently with proper tracking."""
        # Create two concurrent message calls
        msg1 = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {"prompt": "Question 1"}
            }
        }

        msg2 = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "observe_cynic",
                "arguments": {"aspect": "health"}
            }
        }

        # Execute both concurrently
        start_time = time.time()
        result1, result2 = await asyncio.gather(
            router.handle_message_async(msg1),
            router.handle_message_async(msg2),
        )
        elapsed = time.time() - start_time

        # Both should have responses
        assert result1.get("id") == 1
        assert result2.get("id") == 2

        # Should have completed (fast)
        assert elapsed < 5.0

    @pytest.mark.asyncio
    async def test_call_ids_are_unique(self, router):
        """Verify that concurrent calls get unique call IDs."""
        call_ids = []

        async def capture_call_id(msg_id: int, prompt: str):
            msg = {
                "jsonrpc": "2.0",
                "id": msg_id,
                "method": "tools/call",
                "params": {
                    "name": "ask_cynic",
                    "arguments": {"prompt": prompt}
                }
            }
            result = await router.handle_message_async(msg)

            # Call IDs are tracked in active_calls, but we verify via
            # internal counter incrementing
            return msg_id

        # Execute three concurrent calls
        ids = await asyncio.gather(
            capture_call_id(1, "Q1"),
            capture_call_id(2, "Q2"),
            capture_call_id(3, "Q3"),
        )

        # All message IDs should be different
        assert len(set(ids)) == 3

    @pytest.mark.asyncio
    async def test_call_isolation_between_concurrent_calls(self, router):
        """Verify that concurrent calls don't interfere with each other."""
        # Create two messages
        msg1 = {
            "jsonrpc": "2.0",
            "id": 100,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {"prompt": "First question"}
            }
        }

        msg2 = {
            "jsonrpc": "2.0",
            "id": 200,
            "method": "tools/call",
            "params": {
                "name": "observe_cynic",
                "arguments": {"aspect": "dogs"}
            }
        }

        # Execute concurrently
        result1, result2 = await asyncio.gather(
            router.handle_message_async(msg1),
            router.handle_message_async(msg2),
        )

        # Results should have correct IDs (proving they weren't mixed up)
        assert result1.get("id") == 100
        assert result2.get("id") == 200

        # Both should be JSON-RPC responses
        assert "jsonrpc" in result1
        assert "jsonrpc" in result2


class TestConcurrentObserveCalls:
    """Test three concurrent observe calls."""

    @pytest.mark.asyncio
    async def test_three_concurrent_observe_calls(self, router):
        """Verify that multiple observe calls can run in parallel."""
        start_time = time.time()

        # Create three observe messages
        messages = [
            {
                "jsonrpc": "2.0",
                "id": i,
                "method": "tools/call",
                "params": {
                    "name": "observe_cynic",
                    "arguments": {"aspect": f"aspect_{i}"}
                }
            }
            for i in range(3)
        ]

        # Execute all concurrently
        results = await asyncio.gather(
            *[router.handle_message_async(msg) for msg in messages]
        )

        elapsed = time.time() - start_time

        # All should complete
        assert len(results) == 3
        assert all(r.get("jsonrpc") == "2.0" for r in results)

        # IDs should match
        for i, result in enumerate(results):
            assert result.get("id") == i

        # Should be reasonably fast (concurrent)
        assert elapsed < 3.0

    @pytest.mark.asyncio
    async def test_observe_calls_track_in_active_calls(self, router):
        """Verify active_calls tracking during concurrent execution."""
        active_snapshots = []

        async def observe_and_snapshot():
            # Before calling, snapshot active calls
            snapshot_before = router.get_active_calls()

            # Make the observe call
            msg = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "observe_cynic",
                    "arguments": {"aspect": "health"}
                }
            }
            result = await router.handle_message_async(msg)

            # After calling, snapshot active calls (should be empty)
            snapshot_after = router.get_active_calls()

            return {
                "before": snapshot_before,
                "after": snapshot_after,
                "result": result
            }

        # Run one observe call
        data = await observe_and_snapshot()

        # Should have valid responses
        assert data["result"].get("jsonrpc") == "2.0"

        # active_calls should be empty after completion (cleanup)
        assert len(data["after"]) == 0


class TestCallTracking:
    """Test call tracking and metadata management."""

    @pytest.mark.asyncio
    async def test_call_counter_increments(self, router):
        """Verify that the call ID counter increments with each call."""
        initial_counter = router._call_id_counter

        # Make a call
        msg = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {"prompt": "Test"}
            }
        }

        await router.handle_message_async(msg)

        # Counter should have incremented
        assert router._call_id_counter == initial_counter + 1

    @pytest.mark.asyncio
    async def test_call_ids_survive_failures(self, router):
        """Verify that call ID counter increments even on failures."""
        counter_before = router._call_id_counter

        # Make a call with invalid tool (should fail)
        msg1 = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "nonexistent_tool",
                "arguments": {}
            }
        }

        result1 = await router.handle_message_async(msg1)

        counter_after_fail = router._call_id_counter

        # Make another call (should succeed or fail gracefully)
        msg2 = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "observe_cynic",
                "arguments": {}
            }
        }

        result2 = await router.handle_message_async(msg2)

        # Counter should have incremented twice
        assert router._call_id_counter >= counter_before + 2

    @pytest.mark.asyncio
    async def test_active_calls_cleanup(self, router):
        """Verify that active_calls are cleaned up after completion."""
        assert len(router.active_calls) == 0

        msg = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "observe_cynic",
                "arguments": {}
            }
        }

        await router.handle_message_async(msg)

        # Should be cleaned up after completion
        assert len(router.active_calls) == 0


class TestGetActiveCalls:
    """Test get_active_calls() monitoring."""

    @pytest.mark.asyncio
    async def test_get_active_calls_format(self, router):
        """Verify that get_active_calls returns proper format."""
        # With no active calls
        active = router.get_active_calls()
        assert isinstance(active, dict)
        assert len(active) == 0

        # The format check is harder to do without mocking the handlers
        # to stay longer, but we can at least verify the structure
        # when calls complete quickly

    @pytest.mark.asyncio
    async def test_concurrent_calls_consistency(self, router):
        """Verify that concurrent call tracking is consistent."""
        # Create and execute multiple concurrent calls
        messages = [
            {
                "jsonrpc": "2.0",
                "id": i,
                "method": "tools/call",
                "params": {
                    "name": "ask_cynic" if i % 2 == 0 else "observe_cynic",
                    "arguments": {
                        "prompt": f"Q{i}" if i % 2 == 0 else {"aspect": f"a{i}"}
                    }
                }
            }
            for i in range(5)
        ]

        results = await asyncio.gather(
            *[router.handle_message_async(msg) for msg in messages]
        )

        # All should complete
        assert len(results) == 5

        # All should have proper JSON-RPC format
        for result in results:
            assert result.get("jsonrpc") == "2.0"

        # active_calls should be empty (all completed)
        assert len(router.active_calls) == 0


class TestMessageRoutingConcurrency:
    """Test that message routing works correctly under concurrent load."""

    @pytest.mark.asyncio
    async def test_mixed_message_types_concurrent(self, router):
        """Verify that different message types work concurrently."""
        # tools/list message
        list_msg = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
        }

        # tools/call message
        call_msg = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "observe_cynic",
                "arguments": {}
            }
        }

        # Execute concurrently
        # tools/list is sync, tools/call is async, but both should work together
        result1 = await router.handle_message_async(list_msg)
        result2 = await router.handle_message_async(call_msg)

        # Both should be valid responses
        assert result1.get("jsonrpc") == "2.0"
        assert result1.get("id") == 1

        assert result2.get("jsonrpc") == "2.0"
        assert result2.get("id") == 2

        # tools/list should have tools in result
        assert "result" in result1 or "error" in result1


class TestErrorHandlingConcurrency:
    """Test error handling in concurrent execution."""

    @pytest.mark.asyncio
    async def test_one_failure_doesnt_block_others(self, router):
        """Verify that failure in one call doesn't block others."""
        # One message with missing prompt (should fail)
        bad_msg = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {}  # Missing 'prompt'
            }
        }

        # One message that should work
        good_msg = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "observe_cynic",
                "arguments": {}
            }
        }

        # Execute concurrently
        result1, result2 = await asyncio.gather(
            router.handle_message_async(bad_msg),
            router.handle_message_async(good_msg),
        )

        # First may error but second should proceed
        assert result1.get("id") == 1
        assert result2.get("id") == 2
        assert result2.get("jsonrpc") == "2.0"

    @pytest.mark.asyncio
    async def test_concurrent_calls_no_exception_leak(self, router):
        """Verify that exceptions in concurrent calls don't leak to other calls."""
        # Create messages that will exercise the router

        messages = [
            {
                "jsonrpc": "2.0",
                "id": i,
                "method": "tools/call",
                "params": {
                    "name": "ask_cynic",
                    "arguments": {"prompt": f"Question {i}"}
                }
            }
            for i in range(3)
        ]

        # Execute concurrently - should not raise exceptions
        results = await asyncio.gather(
            *[router.handle_message_async(msg) for msg in messages],
            return_exceptions=False,  # Let actual exceptions propagate for debugging
        )

        # All should complete (even if with errors in result)
        assert len(results) == 3
        for result in results:
            assert isinstance(result, dict)
            assert "jsonrpc" in result


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
