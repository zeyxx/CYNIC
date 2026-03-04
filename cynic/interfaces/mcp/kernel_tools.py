"""Universal CYNIC tool implementations — returns raw dicts, no MCP types."""
from __future__ import annotations

from typing import Any

from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter


async def tool_run_empirical_test(adapter: ClaudeCodeAdapter, args: dict) -> dict:
    """Run an empirical test with given count and optional seed."""
    count = args.get("count", 1000)
    seed = args.get("seed")
    result = await adapter.start_empirical_test(count=count, seed=seed)
    return result or {"error": "No response from kernel"}


async def tool_get_job_status(adapter: ClaudeCodeAdapter, args: dict) -> dict:
    """Poll test progress by job_id."""
    job_id = args.get("job_id", "")
    result = await adapter.poll_test_progress(job_id)
    return result or {"error": "No response from kernel"}


async def tool_get_test_results(adapter: ClaudeCodeAdapter, args: dict) -> dict:
    """Get test results by job_id."""
    job_id = args.get("job_id", "")
    result = await adapter.get_test_results(job_id)
    return result or {"error": "No response from kernel"}


async def tool_test_axiom_irreducibility(adapter: ClaudeCodeAdapter, args: dict) -> dict:
    """Test axiom irreducibility."""
    axiom = args.get("axiom")
    result = await adapter.test_axiom_irreducibility(axiom=axiom)
    return result or {"error": "No response from kernel"}


async def tool_query_telemetry(adapter: ClaudeCodeAdapter, args: dict) -> dict:
    """Query CYNIC telemetry metrics."""
    metric = args.get("metric", "uptime_s")
    result = await adapter.query_telemetry(metric=metric)
    return result or {"error": "No response from kernel"}


async def tool_ask_cynic(adapter: ClaudeCodeAdapter, args: dict) -> dict:
    """Ask CYNIC a question."""
    question = args.get("question", "")
    context = args.get("context")
    reality = args.get("reality", "CODE")
    result = await adapter.ask_cynic(question=question, context=context, reality=reality)
    return result or {"error": "No response from kernel"}


async def tool_learn_cynic(adapter: ClaudeCodeAdapter, args: dict) -> dict:
    """Teach CYNIC by providing feedback on a judgment."""
    judgment_id = args.get("judgment_id", "")
    rating = args.get("rating", 0.0)
    comment = args.get("comment")
    result = await adapter.teach_cynic(judgment_id=judgment_id, rating=rating, comment=comment)
    return result or {"error": "No response from kernel"}


TOOL_REGISTRY: dict[str, Any] = {
    "cynic_run_empirical_test": tool_run_empirical_test,
    "cynic_get_job_status": tool_get_job_status,
    "cynic_get_test_results": tool_get_test_results,
    "cynic_test_axiom_irreducibility": tool_test_axiom_irreducibility,
    "cynic_query_telemetry": tool_query_telemetry,
    "ask_cynic": tool_ask_cynic,
    "learn_cynic": tool_learn_cynic,
}
