#!/usr/bin/env python3.13
"""Directly test calling the hypergraph handler function."""
import asyncio
from unittest.mock import AsyncMock, MagicMock

async def test_hypergraph():
    # Import the handler
    from cynic.api.routers.mcp import get_hypergraph_edges

    # Create mock state
    state = MagicMock()
    state.event_journal = AsyncMock()
    state.event_journal.time_range = AsyncMock(return_value=[])
    state.decision_tracer = AsyncMock()

    # Call the handler
    result = await get_hypergraph_edges(limit=10, state=state)

    print("Handler result:")
    print(f"  Keys: {result.keys()}")
    print(f"  Count: {result.get('count')}")
    print(f"  Edges: {result.get('edges', [])}")
    print("\nHandler works correctly!")
    return True

if __name__ == "__main__":
    success = asyncio.run(test_hypergraph())
    exit(0 if success else 1)
