"""
Integration tests for ClaudeCodeAdapter in claude_code_bridge.

Verifies that the bridge correctly uses the adapter's high-level methods
instead of making direct HTTP calls.
"""

from unittest.mock import AsyncMock, patch

import pytest

# Skip entire test module if MCP server is not available in test environment
pytest.importorskip("mcp.server", minversion=None)

from cynic.interfaces.mcp.claude_code_bridge import (
    _tool_ask_cynic,
    _tool_cynic_get_job_status,
    _tool_cynic_get_test_results,
    _tool_cynic_query_telemetry,
    _tool_cynic_run_empirical_test,
    _tool_cynic_test_axiom_irreducibility,
    _tool_learn_cynic,
)


@pytest.mark.asyncio
async def test_empirical_test_uses_adapter():
    """Test that _tool_cynic_run_empirical_test uses adapter.start_empirical_test."""
    with patch("cynic.interfaces.mcp.claude_code_bridge.get_adapter") as mock_get:
        mock_adapter = AsyncMock()
        mock_adapter.start_empirical_test = AsyncMock(return_value={
            "job_id": "test-123",
            "status": "queued",
        })
        mock_get.return_value = mock_adapter

        result = await _tool_cynic_run_empirical_test({"count": 1000})

        # Verify adapter method was called
        mock_adapter.start_empirical_test.assert_called_once_with(count=1000, seed=None)

        # Verify response contains job_id
        assert len(result) == 1
        assert "test-123" in result[0].text


@pytest.mark.asyncio
async def test_job_status_uses_adapter():
    """Test that _tool_cynic_get_job_status uses adapter.poll_test_progress."""
    with patch("cynic.interfaces.mcp.claude_code_bridge.get_adapter") as mock_get:
        mock_adapter = AsyncMock()
        mock_adapter.poll_test_progress = AsyncMock(return_value={
            "status": "running",
            "progress_percent": 45.0,
            "iterations_done": 450,
            "iterations_total": 1000,
            "eta_s": 300,
        })
        mock_get.return_value = mock_adapter

        result = await _tool_cynic_get_job_status({"job_id": "test-123"})

        # Verify adapter method was called
        mock_adapter.poll_test_progress.assert_called_once_with("test-123")

        # Verify response contains progress
        assert len(result) == 1
        assert "45" in result[0].text


@pytest.mark.asyncio
async def test_get_results_uses_adapter():
    """Test that _tool_cynic_get_test_results uses adapter.get_test_results."""
    with patch("cynic.interfaces.mcp.claude_code_bridge.get_adapter") as mock_get:
        mock_adapter = AsyncMock()
        mock_adapter.get_test_results = AsyncMock(return_value={
            "q_scores": [45.2, 48.1, 51.3],
            "avg_q": 52.4,
            "min_q": 45.2,
            "max_q": 61.8,
            "learning_efficiency": 1.18,
            "emergences": 3,
            "duration_s": 1200,
        })
        mock_get.return_value = mock_adapter

        result = await _tool_cynic_get_test_results({"job_id": "test-123"})

        # Verify adapter method was called
        mock_adapter.get_test_results.assert_called_once_with("test-123")

        # Verify response contains metrics
        assert len(result) == 1
        assert "52.4" in result[0].text


@pytest.mark.asyncio
async def test_axiom_irreducibility_uses_adapter():
    """Test that _tool_cynic_test_axiom_irreducibility uses adapter.test_axiom_irreducibility."""
    with patch("cynic.interfaces.mcp.claude_code_bridge.get_adapter") as mock_get:
        mock_adapter = AsyncMock()
        mock_adapter.test_axiom_irreducibility = AsyncMock(return_value={
            "axiom_impacts": [
                {
                    "name": "PHI",
                    "baseline_q": 52.4,
                    "disabled_q": 38.1,
                    "impact_percent": 27.3,
                    "irreducible": True,
                }
            ]
        })
        mock_get.return_value = mock_adapter

        result = await _tool_cynic_test_axiom_irreducibility({"axiom": "PHI"})

        # Verify adapter method was called
        mock_adapter.test_axiom_irreducibility.assert_called_once_with(axiom="PHI")

        # Verify response contains results
        assert len(result) == 1
        assert "PHI" in result[0].text


@pytest.mark.asyncio
async def test_query_telemetry_uses_adapter():
    """Test that _tool_cynic_query_telemetry uses adapter.query_telemetry."""
    with patch("cynic.interfaces.mcp.claude_code_bridge.get_adapter") as mock_get:
        mock_adapter = AsyncMock()
        mock_adapter.query_telemetry = AsyncMock(return_value={
            "uptime_s": 3600.0,
            "q_table_entries": 1024,
            "total_judgments": 12500,
            "learning_rate": 0.001,
        })
        mock_get.return_value = mock_adapter

        result = await _tool_cynic_query_telemetry({"metric": "uptime_s"})

        # Verify adapter method was called
        mock_adapter.query_telemetry.assert_called_once_with(metric="uptime_s")

        # Verify response contains metrics
        assert len(result) == 1
        assert "3600" in result[0].text


@pytest.mark.asyncio
async def test_ask_cynic_uses_adapter():
    """Test that _tool_ask_cynic uses adapter.ask_cynic."""
    with patch("cynic.interfaces.mcp.claude_code_bridge.get_adapter") as mock_get:
        mock_adapter = AsyncMock()
        mock_adapter.ask_cynic = AsyncMock(return_value={
            "q_score": 72,
            "verdict": "WAG",
            "confidence": 0.58,
            "judgment_id": "j-123",
        })
        mock_get.return_value = mock_adapter

        result = await _tool_ask_cynic({
            "question": "Is this code good?",
            "context": "function foo() {}",
            "reality": "CODE",
        })

        # Verify adapter method was called
        mock_adapter.ask_cynic.assert_called_once_with(
            question="Is this code good?",
            context="function foo() {}",
            reality="CODE",
        )

        # Verify response contains judgment
        assert len(result) == 1
        assert "72" in result[0].text


@pytest.mark.asyncio
async def test_learn_cynic_uses_adapter():
    """Test that _tool_learn_cynic uses adapter.teach_cynic."""
    with patch("cynic.interfaces.mcp.claude_code_bridge.get_adapter") as mock_get:
        mock_adapter = AsyncMock()
        mock_adapter.teach_cynic = AsyncMock(return_value={
            "status": "updated",
            "judgment_id": "j-123",
            "qtable_updated": True,
            "new_q_score": 75,
            "learning_rate": 0.001,
        })
        mock_get.return_value = mock_adapter

        result = await _tool_learn_cynic({
            "judgment_id": "j-123",
            "rating": 0.8,
            "comment": "Good judgment",
        })

        # Verify adapter method was called
        mock_adapter.teach_cynic.assert_called_once_with(
            judgment_id="j-123",
            rating=0.8,
            comment="Good judgment",
        )

        # Verify response confirms update
        assert len(result) == 1
        assert "updated" in result[0].text


@pytest.mark.asyncio
async def test_adapter_error_handling():
    """Test that tools handle adapter errors gracefully."""
    with patch("cynic.interfaces.mcp.claude_code_bridge.get_adapter") as mock_get:
        mock_adapter = AsyncMock()
        mock_adapter.start_empirical_test = AsyncMock(return_value={
            "error": "CYNIC not ready",
        })
        mock_get.return_value = mock_adapter

        result = await _tool_cynic_run_empirical_test({"count": 1000})

        # Verify error is communicated
        assert len(result) == 1
        assert "Error" in result[0].text


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
