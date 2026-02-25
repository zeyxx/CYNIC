"""
Claude Code Adapter — Specialized CYNIC interface optimized for Claude Code workflows.

Provides:
1. Simplified API (high-level operations, not raw HTTP calls)
2. Caching (avoid repeated queries)
3. Batching (combine multiple operations)
4. Auto-state discovery (detect CYNIC readiness without polling)
5. Progress streaming (real-time test status via callbacks)
6. Intelligent fallbacks (graceful degradation if CYNIC unavailable)

This adapter is used by claude_code_bridge.py to implement all tool calls.
It's optimized for Claude Code's token-constrained environment.
"""
from __future__ import annotations

import asyncio
import json
import logging
import time
from typing import Any, Callable, Optional, Dict
from dataclasses import dataclass, field

import aiohttp

from cynic.mcp.timeouts import TimeoutConfig, TimeoutCategory

logger = logging.getLogger("cynic.mcp.claude_code_adapter")


@dataclass
class CynicState:
    """Cached CYNIC organism state."""

    healthy: bool
    consciousness_level: float
    dogs_active: int
    uptime_s: float
    q_table_entries: int
    total_judgments: int
    last_update: float = field(default_factory=time.time)

    def is_stale(self, max_age_s: float = 60) -> bool:
        """Check if state cache is stale."""
        return time.time() - self.last_update > max_age_s


class ClaudeCodeAdapter:
    """
    High-level CYNIC adapter for Claude Code.

    Wraps low-level HTTP calls with caching, batching, and progress streaming.
    """

    def __init__(self, cynic_url: str = "http://127.0.0.1:8765", timeout_s: float = 30):
        """
        Initialize adapter.

        Args:
            cynic_url: Base URL of CYNIC HTTP server
            timeout_s: Request timeout
        """
        self.cynic_url = cynic_url
        self.timeout = aiohttp.ClientTimeout(total=timeout_s)
        self.session: Optional[aiohttp.ClientSession] = None

        # Caches
        self._state_cache: Optional[CynicState] = None
        self._judgment_cache: Dict[str, Any] = {}  # judgment_id → result

        # Progress callbacks
        self._progress_callbacks: list[Callable[[float, str], None]] = []

    async def __aenter__(self):
        """Context manager entry."""
        # On Windows, use ThreadedResolver to avoid aiodns SelectorEventLoop requirement
        import platform
        if platform.system() == "Windows":
            resolver = aiohttp.ThreadedResolver()
            connector = aiohttp.TCPConnector(resolver=resolver, ssl=False)
            self.session = aiohttp.ClientSession(timeout=self.timeout, connector=connector)
        else:
            self.session = aiohttp.ClientSession(timeout=self.timeout)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        if self.session:
            await self.session.close()

    # ════════════════════════════════════════════════════════════════════════════
    # TIMEOUT HANDLING
    # ════════════════════════════════════════════════════════════════════════════

    def _get_timeout_for_tool(self, tool_name: str) -> Optional[float]:
        """
        Get context-aware timeout for a tool.

        Args:
            tool_name: Name of the MCP tool (e.g., "ask_cynic")

        Returns:
            Timeout in seconds, or None for indefinite (stream tools)
        """
        timeout = TimeoutConfig.get_timeout(tool_name)
        category = TimeoutConfig.get_category(tool_name)
        logger.debug(f"Tool '{tool_name}' → {category.name} ({timeout}s)")
        return timeout

    async def _call_with_timeout(
        self, tool_name: str, coro: Any
    ) -> Any:
        """
        Execute a coroutine with context-aware timeout.

        Args:
            tool_name: Name of the tool (for timeout lookup)
            coro: Coroutine to execute

        Returns:
            Result of the coroutine

        Raises:
            asyncio.TimeoutError: If timeout exceeded
        """
        timeout = self._get_timeout_for_tool(tool_name)

        if timeout is None:
            # Stream tools - no timeout
            return await coro

        try:
            return await asyncio.wait_for(coro, timeout=timeout)
        except asyncio.TimeoutError:
            logger.error(f"Tool '{tool_name}' timed out after {timeout}s")
            raise

    # ════════════════════════════════════════════════════════════════════════════
    # AUTO-DISCOVERY
    # ════════════════════════════════════════════════════════════════════════════

    async def is_cynic_ready(self, force_refresh: bool = False) -> bool:
        """
        Check if CYNIC is ready to handle requests.

        Performs health check + verifies state is accessible.
        Caches result for 60 seconds to avoid repeated queries.
        Uses FAST (2s) timeout for health checks.

        Args:
            force_refresh: Skip cache and do fresh check

        Returns:
            True if CYNIC is healthy and ready
        """
        try:
            if not force_refresh and self._state_cache and not self._state_cache.is_stale():
                return self._state_cache.healthy

            # Fresh health check with FAST timeout
            async def do_health_check():
                async with self.session.get(f"{self.cynic_url}/health") as resp:
                    if resp.status != 200:
                        return False

                    data = await resp.json()
                    health = data.get("health", {})
                    kernel_status = health.get("cynic-kernel", {}).get("status", "unknown")
                    is_healthy = kernel_status == "healthy"

                    if is_healthy:
                        # Cache state
                        self._state_cache = CynicState(
                            healthy=True,
                            consciousness_level=data.get("consciousness", {}).get("level", 0),
                            dogs_active=data.get("consciousness", {}).get("dogs_active", 0),
                            uptime_s=data.get("uptime_s", 0),
                            q_table_entries=data.get("q_table_entries", 0),
                            total_judgments=data.get("total_judgments", 0),
                        )

                    return is_healthy

            return await self._call_with_timeout("cynic_health", do_health_check())

        except (aiohttp.ClientError, asyncio.TimeoutError):
            return False

    async def get_cynic_state(self, force_refresh: bool = False) -> Optional[CynicState]:
        """
        Get cached CYNIC organism state.

        Args:
            force_refresh: Skip cache and fetch fresh state

        Returns:
            CynicState if healthy, None otherwise
        """
        if not force_refresh and self._state_cache and not self._state_cache.is_stale():
            return self._state_cache

        is_ready = await self.is_cynic_ready(force_refresh=True)
        return self._state_cache if is_ready else None

    # ════════════════════════════════════════════════════════════════════════════
    # CONSCIOUSNESS OPERATIONS
    # ════════════════════════════════════════════════════════════════════════════

    async def ask_cynic(
        self, question: str, context: Optional[str] = None, reality: str = "CODE"
    ) -> dict[str, Any]:
        """
        Ask CYNIC a question and get judgment.

        Uses NORMAL (30s) timeout for cognitive operations.

        Args:
            question: The question
            context: Optional context (code snippet, etc)
            reality: Reality dimension (CODE, CYNIC, SOLANA, etc)

        Returns:
            {q_score, verdict, confidence, judgment_id, ...}
        """
        try:
            async def do_ask():
                async with self.session.post(
                    f"{self.cynic_url}/judge",
                    json={"content": question, "context": context, "reality": reality},
                ) as resp:
                    if resp.status != 200:
                        return {"error": f"HTTP {resp.status}"}

                    result = await resp.json()
                    judgment_id = result.get("judgment_id")

                    # Cache judgment for later learning
                    if judgment_id:
                        self._judgment_cache[judgment_id] = result

                    return result

            return await self._call_with_timeout("ask_cynic", do_ask())

        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    async def teach_cynic(
        self, judgment_id: str, rating: float, comment: Optional[str] = None
    ) -> dict[str, Any]:
        """
        Teach CYNIC by providing feedback on a judgment.

        Uses NORMAL (30s) timeout for learning operations.

        Args:
            judgment_id: ID of judgment to learn from
            rating: Feedback (-1.0 to 1.0)
            comment: Optional explanation

        Returns:
            {status, qtable_updated, new_q_score, ...}
        """
        try:
            async def do_teach():
                async with self.session.post(
                    f"{self.cynic_url}/learn",
                    json={
                        "signal": {"judgment_id": judgment_id, "rating": rating, "comment": comment},
                        "update_qtable": True,
                    },
                ) as resp:
                    if resp.status != 200:
                        return {"error": f"HTTP {resp.status}"}

                    return await resp.json()

            return await self._call_with_timeout("learn_cynic", do_teach())

        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    # ════════════════════════════════════════════════════════════════════════════
    # EMPIRICAL TESTING OPERATIONS
    # ════════════════════════════════════════════════════════════════════════════

    async def start_empirical_test(
        self, count: int = 1000, seed: Optional[int] = None
    ) -> dict[str, Any]:
        """
        Start an empirical test job.

        Uses BATCH (300s) timeout for long-running empirical tests.

        Args:
            count: Number of iterations
            seed: Optional random seed

        Returns:
            {job_id, status, message, count}
        """
        try:
            async def do_start():
                async with self.session.post(
                    f"{self.cynic_url}/empirical/test/start", json={"count": count, "seed": seed}
                ) as resp:
                    if resp.status != 200:
                        return {"error": f"HTTP {resp.status}"}

                    return await resp.json()

            return await self._call_with_timeout("cynic_run_empirical_test", do_start())

        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    async def poll_test_progress(
        self,
        job_id: str,
        callback: Optional[Callable[[float, str], None]] = None,
        max_wait_s: float = 3600,
        poll_interval_s: float = 5,
    ) -> dict[str, Any]:
        """
        Poll test progress until completion (with optional progress callback).

        Uses BATCH (300s) timeout for individual poll requests.
        Note: max_wait_s controls total wait time, timeout controls per-request timeout.

        Args:
            job_id: Test job ID
            callback: Optional callback(progress_percent, status_message)
            max_wait_s: Max time to wait
            poll_interval_s: Polling interval

        Returns:
            Final status: {status, progress_percent, iterations_done, ...}
        """
        start_time = time.time()

        while time.time() - start_time < max_wait_s:
            try:
                async def do_poll():
                    async with self.session.get(f"{self.cynic_url}/empirical/test/{job_id}") as resp:
                        if resp.status != 200:
                            return {"error": f"HTTP {resp.status}"}

                        status = await resp.json()

                        if "error" in status:
                            return status

                        progress = status.get("progress_percent", 0)
                        status_str = status.get("status", "unknown")

                        # Call progress callback
                        if callback:
                            callback(progress, status_str)

                        # If complete, return
                        if status_str == "complete":
                            return status

                        return {"status": status_str, "progress_percent": progress}

                result = await self._call_with_timeout("cynic_run_empirical_test", do_poll())

                if "error" in result or result.get("status") == "complete":
                    return result

                # Wait before polling again
                await asyncio.sleep(poll_interval_s)

            except (aiohttp.ClientError, asyncio.TimeoutError) as e:
                return {"error": str(e)}

        return {"error": f"Test timeout after {max_wait_s}s"}

    async def get_test_results(self, job_id: str) -> dict[str, Any]:
        """
        Get completed test results.

        Uses NORMAL (30s) timeout - results are already computed.

        Args:
            job_id: Test job ID

        Returns:
            {q_scores, avg_q, learning_efficiency, emergences, duration_s}
        """
        try:
            async def do_get_results():
                async with self.session.get(
                    f"{self.cynic_url}/empirical/test/{job_id}/results"
                ) as resp:
                    if resp.status != 200:
                        return {"error": f"HTTP {resp.status}"}

                    return await resp.json()

            return await self._call_with_timeout("cynic_query_telemetry", do_get_results())

        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    async def test_axiom_irreducibility(
        self, axiom: Optional[str] = None
    ) -> dict[str, Any]:
        """
        Test if axioms are irreducible.

        Uses BATCH (300s) timeout - comprehensive axiom testing can be slow.

        Args:
            axiom: Specific axiom or None for all

        Returns:
            {axiom_impacts: [{name, baseline_q, disabled_q, impact_percent, irreducible}]}
        """
        try:
            async def do_test():
                async with self.session.post(
                    f"{self.cynic_url}/empirical/axioms/test", json={"axiom": axiom}
                ) as resp:
                    if resp.status != 200:
                        return {"error": f"HTTP {resp.status}"}

                    return await resp.json()

            return await self._call_with_timeout("cynic_test_axiom_irreducibility", do_test())

        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    # ════════════════════════════════════════════════════════════════════════════
    # TELEMETRY
    # ════════════════════════════════════════════════════════════════════════════

    async def query_telemetry(self, metric: str = "uptime_s") -> dict[str, Any]:
        """
        Query SONA telemetry metrics.

        Uses NORMAL (30s) timeout for metric queries.

        Args:
            metric: Metric name (uptime_s, q_table_entries, total_judgments, learning_rate)

        Returns:
            {metric, value, ...}
        """
        try:
            async def do_query():
                async with self.session.get(
                    f"{self.cynic_url}/empirical/telemetry", params={"metric": metric}
                ) as resp:
                    if resp.status != 200:
                        return {"error": f"HTTP {resp.status}"}

                    return await resp.json()

            return await self._call_with_timeout("cynic_query_telemetry", do_query())

        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    # ════════════════════════════════════════════════════════════════════════════
    # CONVENIENCE METHODS
    # ════════════════════════════════════════════════════════════════════════════

    async def run_test_and_wait(
        self,
        count: int = 1000,
        seed: Optional[int] = None,
        progress_callback: Optional[Callable[[float, str], None]] = None,
    ) -> dict[str, Any]:
        """
        High-level operation: Start test, poll until complete, return results.

        Single call that handles the full workflow.

        Args:
            count: Iterations
            seed: Random seed
            progress_callback: Optional progress callback

        Returns:
            {job_id, q_scores, avg_q, learning_efficiency, ...}
        """
        # Start test
        start_result = await self.start_empirical_test(count=count, seed=seed)
        if "error" in start_result:
            return start_result

        job_id = start_result.get("job_id")

        # Poll until complete
        final_status = await self.poll_test_progress(
            job_id, callback=progress_callback
        )
        if "error" in final_status or final_status.get("status") != "complete":
            return final_status

        # Get results
        results = await self.get_test_results(job_id)
        results["job_id"] = job_id
        return results

    def add_progress_callback(self, callback: Callable[[float, str], None]) -> None:
        """Register a progress callback for tests."""
        self._progress_callbacks.append(callback)

    def clear_caches(self) -> None:
        """Clear all internal caches (for testing or reset)."""
        self._state_cache = None
        self._judgment_cache.clear()
        logger.info("Adapter caches cleared")

    async def stream_telemetry(
        self,
        duration_s: float = 60,
        on_update: Optional[Callable[[dict], None]] = None,
    ) -> dict[str, Any]:
        """
        Watch CYNIC telemetry stream for duration_s seconds.
        Collects events and returns aggregated summary.

        Uses STREAM (indefinite) timeout - streaming operations don't have time limits.

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
            if not self.session:
                await self.__aenter__()

            ws_url = self.cynic_url.replace("http", "ws") + "/ws/telemetry"

            deadline = time.time() + duration_s
            events: list[dict] = []

            async def do_stream():
                async with self.session.ws_connect(ws_url) as ws:
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
                return self._summarize_telemetry_events(events, actual_duration)

            return await self._call_with_timeout("cynic_watch_telemetry", do_stream())

        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            logger.error("Telemetry streaming failed: %s", e)
            return {"error": str(e)}

    @staticmethod
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

        # Compute average Q-score
        if q_scores:
            summary["avg_q_score"] = sum(q_scores) / len(q_scores)

        summary["verdicts"] = verdict_counts

        return summary
