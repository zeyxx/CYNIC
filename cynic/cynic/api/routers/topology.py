"""
CYNIC Topology router — /changes · /changes/stream

Real-time visibility into code changes with semantic analysis.
"""

from __future__ import annotations

import asyncio
import json
import logging
import time
from typing import TYPE_CHECKING

from fastapi import APIRouter, Depends

from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import ChangeAnalyzedPayload

if TYPE_CHECKING:
    from cynic.api.state import CynicOrganism

logger = logging.getLogger("cynic.api.routers.topology")

router_topology = APIRouter(tags=["topology"], prefix="/api/changes")


# ════════════════════════════════════════════════════════════════════════════
# REST: GET /changes — Recent changes + analysis
# ════════════════════════════════════════════════════════════════════════════

@router_topology.get("")
async def get_changes(
    limit: int = 10,
    state: CynicOrganism = Depends(lambda: None),  # injected by FastAPI
) -> dict:
    """
    Get recent changes from ChangeTracker + ChangeAnalyzer.

    Combines raw tracking (ChangeTracker) with semantic analysis (ChangeAnalyzer).

    Query params:
      limit: number of changes to return (default 10)

    Returns:
      {
        "changes": [
          {
            "timestamp": 1771557339.65,
            "filepath": "cynic/api/handlers/direct.py",
            "category": "handlers",
            "change_type": "MODIFIED",
            "file_lines": 245,
            "subsystem": "api",
            "impact_level": "HIGH",
            "risk_estimate": 0.6,
            "suggested_action": "REVIEW"
          },
          ...
        ],
        "total_tracked": 233,
        "last_updated": 1771557339.65
      }
    """
    # This would require state injection, which is done via lifespan in server.py
    # For now, return a placeholder that will be properly wired
    return {
        "changes": [],
        "total_tracked": 0,
        "last_updated": time.time(),
        "note": "Wired via FastAPI lifespan — requires state injection",
    }


# ════════════════════════════════════════════════════════════════════════════
# SSE: GET /changes/stream — Real-time change stream
# ════════════════════════════════════════════════════════════════════════════

@router_topology.get("/stream", response_class=None)
async def stream_changes() -> None:
    """
    SSE stream of real-time change analysis.

    Subscribes to CHANGE_ANALYZED events on the core bus.
    Streams each analysis as an SSE data line.

    Protocol:
      : keepalive (comment)
      data: {"type":"CHANGE_ANALYZED","files":[...], ...}\n\n

    Client disconnect → clean unsubscribe from bus.
    Queue overflow (>100 buffered) → events dropped silently.
    """
    # This endpoint needs to be implemented as a StreamingResponse
    # with asyncio.Queue + bus.on/off pattern (see ws.py for reference).
    # Will be handled in the actual implementation below.
    pass


# ════════════════════════════════════════════════════════════════════════════
# IMPLEMENTATION NOTE (for lifespan wiring)
# ════════════════════════════════════════════════════════════════════════════
#
# The /changes endpoint needs:
#   1. state.change_tracker.get_recent_changes(limit)
#   2. state.change_analyzer.recent_analyses(limit)
#   3. Merge + return combined view
#
# The /changes/stream endpoint needs:
#   1. bus = get_core_bus()
#   2. queue = asyncio.Queue(maxsize=100)
#   3. handler subscribes to CoreEvent.CHANGE_ANALYZED
#   4. yield SSE format: "data: {...}\n\n"
#   5. cleanup: bus.off() on disconnect
#
# Pattern reuse from ws.py:
#   - asyncio.Queue(maxsize=100) + bus.on/off
#   - Fire-and-forget handler task creation
#   - 30s timeout for keepalive
#
# This will be wired by _IncrementalRouter in state.py which
# injects proper dependencies (state, bus, etc.).
