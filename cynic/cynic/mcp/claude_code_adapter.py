"""
Claude Code Adapter — WebSocket and HTTP bridge to CYNIC.

Provides methods for Claude Code (via MCP bridge) to stream and poll CYNIC state.
"""
from __future__ import annotations

import asyncio
import json
import logging
import time
from typing import Any, Optional, Callable

import aiohttp

logger = logging.getLogger("cynic.mcp.claude_code_adapter")


class ClaudeCodeAdapter:
    """Bridge between Claude Code (MCP stdio) and CYNIC HTTP/WebSocket."""

    def __init__(self, cynic_url: str = "http://127.0.0.1:8765"):
        """
        Initialize adapter.

        Args:
            cynic_url: Base URL of CYNIC HTTP server (e.g., http://127.0.0.1:8765)
        """
        self.cynic_url = cynic_url
        self.session: Optional[aiohttp.ClientSession] = None

    async def _ensure_session(self) -> aiohttp.ClientSession:
        """Lazy-initialize session."""
        if self.session is None:
            self.session = aiohttp.ClientSession(
                timeout=aiohttp.ClientTimeout(total=30)
            )
        return self.session

    async def stream_telemetry(
        self,
        duration_s: float = 60,
        on_update: Optional[Callable[[dict], None]] = None,
    ) -> dict[str, Any]:
        """
        Watch CYNIC telemetry stream for duration_s seconds.
        Collects events and returns aggregated summary.

        Args:
            duration_s: How many seconds to watch (default: 60)
            on_update: Optional callback invoked for each event (for real-time updates)

        Returns:
            Aggregated summary dict with keys:
              - judgments_seen: int
              - avg_q_score: float
              - verdicts: dict[str, int] (count by verdict type)
              - learning_events_seen: int
              - last_learning_rate: float
              - sona_ticks_seen: int
              - meta_cycles_seen: int
              - duration_s: float (actual duration)
              - error: str (if connection failed)
        """
        try:
            session = await self._ensure_session()
            ws_url = self.cynic_url.replace("http", "ws") + "/ws/telemetry"

            deadline = time.time() + duration_s
            events: list[dict] = []

            async with session.ws_connect(ws_url) as ws:
                async for msg in ws:
                    if time.time() >= deadline:
                        break

                    if msg.type == aiohttp.WSMsgType.TEXT:
                        try:
                            data = json.loads(msg.data)
                            events.append(data)
                            if on_update:
                                on_update(data)
                        except json.JSONDecodeError:
                            logger.warning("Failed to decode WebSocket message: %s", msg.data)
                    elif msg.type in (aiohttp.WSMsgType.ERROR, aiohttp.WSMsgType.CLOSED):
                        break

            actual_duration = time.time() - (deadline - duration_s)
            return _summarize_telemetry_events(events, actual_duration)

        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            logger.error("Telemetry streaming failed: %s", e)
            return {"error": str(e)}

    async def close(self) -> None:
        """Close the session."""
        if self.session:
            await self.session.close()


def _summarize_telemetry_events(events: list[dict], duration_s: float) -> dict[str, Any]:
    """
    Aggregate telemetry events into a summary.

    Returns:
        Summary dict with event counts, averages, and status.
    """
    summary: dict[str, Any] = {
        "judgments_seen": 0,
        "avg_q_score": 0.0,
        "verdicts": {},
        "learning_events_seen": 0,
        "last_learning_rate": 0.0,
        "sona_ticks_seen": 0,
        "meta_cycles_seen": 0,
        "duration_s": duration_s,
    }

    q_scores: list[float] = []
    verdict_counts: dict[str, int] = {}

    for event in events:
        event_type = event.get("type", "")

        if event_type == "judgment":
            summary["judgments_seen"] += 1
            q = event.get("q_score", 0)
            if q > 0:
                q_scores.append(q)
            verdict = event.get("verdict", "UNKNOWN")
            verdict_counts[verdict] = verdict_counts.get(verdict, 0) + 1

        elif event_type == "learning":
            summary["learning_events_seen"] += 1
            summary["last_learning_rate"] = event.get("learning_rate", 0.0)

        elif event_type == "meta_cycle":
            summary["meta_cycles_seen"] += 1

        elif event_type == "sona_tick":
            summary["sona_ticks_seen"] += 1

        elif event_type == "heartbeat":
            # Keepalive heartbeat — not counted as real event
            pass

    # Compute average Q-score
    if q_scores:
        summary["avg_q_score"] = sum(q_scores) / len(q_scores)

    summary["verdicts"] = verdict_counts

    return summary


# Global adapter instance (lazy-initialized)
_adapter: Optional[ClaudeCodeAdapter] = None


def get_adapter() -> ClaudeCodeAdapter:
    """Get or create the global adapter."""
    global _adapter
    if _adapter is None:
        _adapter = ClaudeCodeAdapter()
    return _adapter
