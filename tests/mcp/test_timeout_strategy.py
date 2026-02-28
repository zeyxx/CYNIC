"""
Test suite for context-aware timeout strategy.

Tests that tools are correctly mapped to timeout categories and that
timeouts are enforced during actual operations.

Coverage:
- TimeoutCategory enum values
- TimeoutConfig tool mappings
- Timeout application in adapter
- Fallback to NORMAL for unknown tools
"""
import pytest
import asyncio
from cynic.interfaces.mcp.timeouts import TimeoutConfig, TimeoutCategory


class TestTimeoutCategory:
    """Test TimeoutCategory enum."""

    def test_fast_timeout_value(self):
        """Test FAST timeout is 2 seconds."""
        assert TimeoutCategory.FAST.value == 2.0

    def test_normal_timeout_value(self):
        """Test NORMAL timeout is 30 seconds."""
        assert TimeoutCategory.NORMAL.value == 30.0

    def test_batch_timeout_value(self):
        """Test BATCH timeout is 300 seconds."""
        assert TimeoutCategory.BATCH.value == 300.0

    def test_stream_timeout_value(self):
        """Test STREAM timeout is None (indefinite)."""
        assert TimeoutCategory.STREAM.value is None

    def test_timeout_category_float_conversion(self):
        """Test that TimeoutCategory can be converted to float."""
        assert float(TimeoutCategory.FAST) == 2.0
        assert float(TimeoutCategory.NORMAL) == 30.0
        assert float(TimeoutCategory.BATCH) == 300.0
        assert float(TimeoutCategory.STREAM) == float("inf")


class TestTimeoutConfig:
    """Test TimeoutConfig class."""

    def test_fast_tools_have_2s_timeout(self):
        """Test that health/status tools use FAST timeout."""
        fast_tools = ["cynic_health", "cynic_status", "cynic_get_job_status"]
        for tool in fast_tools:
            timeout = TimeoutConfig.get_timeout(tool)
            assert timeout == 2.0, f"{tool} should have 2.0s timeout"

    def test_normal_tools_have_30s_timeout(self):
        """Test that cognitive tools use NORMAL timeout."""
        normal_tools = [
            "ask_cynic",
            "observe_cynic",
            "learn_cynic",
            "discuss_cynic",
            "cynic_query_telemetry",
        ]
        for tool in normal_tools:
            timeout = TimeoutConfig.get_timeout(tool)
            assert timeout == 30.0, f"{tool} should have 30.0s timeout"

    def test_batch_tools_have_300s_timeout(self):
        """Test that empirical test tools use BATCH timeout."""
        batch_tools = [
            "cynic_run_empirical_test",
            "cynic_test_axiom_irreducibility",
            "cynic_benchmark_learning_efficiency",
            "cynic_run_load_test",
        ]
        for tool in batch_tools:
            timeout = TimeoutConfig.get_timeout(tool)
            assert timeout == 300.0, f"{tool} should have 300.0s timeout"

    def test_stream_tools_have_no_timeout(self):
        """Test that streaming tools have no timeout."""
        stream_tools = ["cynic_watch_telemetry", "cynic_watch_source", "cynic_stream_judgments"]
        for tool in stream_tools:
            timeout = TimeoutConfig.get_timeout(tool)
            assert timeout is None, f"{tool} should have None (indefinite) timeout"

    def test_get_category(self):
        """Test get_category returns correct TimeoutCategory."""
        assert TimeoutConfig.get_category("cynic_health") == TimeoutCategory.FAST
        assert TimeoutConfig.get_category("ask_cynic") == TimeoutCategory.NORMAL
        assert TimeoutConfig.get_category("cynic_run_empirical_test") == TimeoutCategory.BATCH
        assert TimeoutConfig.get_category("cynic_watch_telemetry") == TimeoutCategory.STREAM

    def test_unknown_tool_defaults_to_normal(self):
        """Test that unknown tools default to NORMAL timeout."""
        unknown_timeout = TimeoutConfig.get_timeout("unknown_tool_xyz")
        assert unknown_timeout == 30.0  # Default to NORMAL

        unknown_category = TimeoutConfig.get_category("unknown_tool_xyz")
        assert unknown_category == TimeoutCategory.NORMAL

    def test_summary_structure(self):
        """Test that summary() returns correct structure."""
        summary = TimeoutConfig.summary()

        # Should have 4 categories
        assert set(summary.keys()) == {"FAST", "NORMAL", "BATCH", "STREAM"}

        # Each category should have timeout_s and tools keys
        for category_name, category_data in summary.items():
            assert "timeout_s" in category_data
            assert "tools" in category_data
            assert isinstance(category_data["tools"], list)

        # Fast should have correct timeout
        assert summary["FAST"]["timeout_s"] == 2.0
        assert "cynic_health" in summary["FAST"]["tools"]

        # Normal should have correct timeout
        assert summary["NORMAL"]["timeout_s"] == 30.0
        assert "ask_cynic" in summary["NORMAL"]["tools"]

        # Batch should have correct timeout
        assert summary["BATCH"]["timeout_s"] == 300.0
        assert "cynic_run_empirical_test" in summary["BATCH"]["tools"]

        # Stream should have no timeout
        assert summary["STREAM"]["timeout_s"] is None
        assert "cynic_watch_telemetry" in summary["STREAM"]["tools"]

    def test_summary_tools_are_sorted(self):
        """Test that tools in summary are alphabetically sorted."""
        summary = TimeoutConfig.summary()

        for category_data in summary.values():
            tools = category_data["tools"]
            assert tools == sorted(tools), "Tools should be alphabetically sorted"

    def test_all_configured_tools_appear_in_summary(self):
        """Test that all tools in TOOL_TIMEOUTS appear in summary."""
        summary = TimeoutConfig.summary()

        all_summary_tools = set()
        for category_data in summary.values():
            all_summary_tools.update(category_data["tools"])

        assert all_summary_tools == set(TimeoutConfig.TOOL_TIMEOUTS.keys())


class TestTimeoutApplication:
    """Test that timeouts are correctly applied in adapter operations."""

    @pytest.mark.asyncio
    async def test_fast_tool_timeout_enforced(self):
        """Test that FAST tools timeout after 2 seconds."""
        from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter

        adapter = ClaudeCodeAdapter()

        # Create a coroutine that takes longer than 2 seconds
        async def slow_operation():
            await asyncio.sleep(3)  # Sleep for 3 seconds
            return {"result": "done"}

        # Should timeout because it exceeds 2s FAST timeout
        with pytest.raises(asyncio.TimeoutError):
            await adapter._call_with_timeout("cynic_health", slow_operation())

    @pytest.mark.asyncio
    async def test_normal_tool_timeout_enforced(self):
        """Test that NORMAL tools timeout after 30 seconds."""
        from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter

        adapter = ClaudeCodeAdapter()

        # Create a coroutine that takes longer than 30 seconds
        async def slow_operation():
            await asyncio.sleep(35)  # Sleep for 35 seconds
            return {"result": "done"}

        # Should timeout because it exceeds 30s NORMAL timeout
        with pytest.raises(asyncio.TimeoutError):
            await adapter._call_with_timeout("ask_cynic", slow_operation())

    @pytest.mark.asyncio
    async def test_stream_tool_no_timeout(self):
        """Test that STREAM tools don't have timeout."""
        from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter

        adapter = ClaudeCodeAdapter()

        # Create a coroutine that finishes quickly
        async def quick_operation():
            await asyncio.sleep(0.1)  # Sleep for 100ms
            return {"result": "done"}

        # Should complete successfully without timeout
        result = await adapter._call_with_timeout("cynic_watch_telemetry", quick_operation())
        assert result == {"result": "done"}

    @pytest.mark.asyncio
    async def test_successful_operation_within_timeout(self):
        """Test that operations completing within timeout succeed."""
        from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter

        adapter = ClaudeCodeAdapter()

        async def quick_operation():
            await asyncio.sleep(0.1)  # Sleep for 100ms (well within 30s)
            return {"status": "success"}

        result = await adapter._call_with_timeout("ask_cynic", quick_operation())
        assert result == {"status": "success"}

    def test_get_timeout_for_tool_helper(self):
        """Test _get_timeout_for_tool helper method."""
        from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter

        adapter = ClaudeCodeAdapter()

        # Test FAST
        timeout = adapter._get_timeout_for_tool("cynic_health")
        assert timeout == 2.0

        # Test NORMAL
        timeout = adapter._get_timeout_for_tool("ask_cynic")
        assert timeout == 30.0

        # Test BATCH
        timeout = adapter._get_timeout_for_tool("cynic_run_empirical_test")
        assert timeout == 300.0

        # Test STREAM
        timeout = adapter._get_timeout_for_tool("cynic_watch_telemetry")
        assert timeout is None


class TestTimeoutEdgeCases:
    """Test edge cases and special scenarios."""

    def test_timeout_config_registry_completeness(self):
        """Test that all essential tools are registered."""
        essential_tools = [
            # Health
            "cynic_health",
            "cynic_status",
            # Cognitive
            "ask_cynic",
            "observe_cynic",
            "learn_cynic",
            # Batch
            "cynic_run_empirical_test",
            "cynic_test_axiom_irreducibility",
            # Stream
            "cynic_watch_telemetry",
        ]

        for tool in essential_tools:
            assert tool in TimeoutConfig.TOOL_TIMEOUTS, f"Tool {tool} should be registered"

    def test_timeout_values_are_reasonable(self):
        """Test that timeout values are reasonable and ordered correctly."""
        assert TimeoutCategory.FAST.value < TimeoutCategory.NORMAL.value
        assert TimeoutCategory.NORMAL.value < TimeoutCategory.BATCH.value

        # All timeouts should be positive or None
        for tool_name, category in TimeoutConfig.TOOL_TIMEOUTS.items():
            timeout = category.value
            assert timeout is None or timeout > 0, f"{tool_name} has invalid timeout {timeout}"

    @pytest.mark.asyncio
    async def test_timeout_at_boundary(self):
        """Test timeout behavior well under timeout (avoid timing precision issues)."""
        from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter

        adapter = ClaudeCodeAdapter()

        # Operation that sleeps well under the timeout (leave margin for precision)
        # FAST timeout is 2.0s, so 0.5s is safe with plenty of margin
        async def safe_operation():
            await asyncio.sleep(0.5)
            return {"result": "success"}

        result = await adapter._call_with_timeout("cynic_health", safe_operation())
        assert result == {"result": "success"}


class TestTimeoutDocumentation:
    """Test that timeout configuration is well-documented."""

    def test_timeout_category_has_docstring(self):
        """Test that TimeoutCategory has documentation."""
        assert TimeoutCategory.__doc__ is not None
        assert "timeout" in TimeoutCategory.__doc__.lower()

    def test_timeout_config_has_docstring(self):
        """Test that TimeoutConfig has documentation."""
        assert TimeoutConfig.__doc__ is not None
        assert "timeout" in TimeoutConfig.__doc__.lower()

    def test_get_timeout_has_docstring(self):
        """Test that get_timeout method is documented."""
        assert TimeoutConfig.get_timeout.__doc__ is not None

    def test_get_category_has_docstring(self):
        """Test that get_category method is documented."""
        assert TimeoutConfig.get_category.__doc__ is not None

    def test_summary_has_docstring(self):
        """Test that summary method is documented."""
        assert TimeoutConfig.summary.__doc__ is not None
