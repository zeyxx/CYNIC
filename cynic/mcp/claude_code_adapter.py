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
        self.session = aiohttp.ClientSession(timeout=self.timeout)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        if self.session:
            await self.session.close()

    # ════════════════════════════════════════════════════════════════════════════
    # AUTO-DISCOVERY
    # ════════════════════════════════════════════════════════════════════════════

    async def is_cynic_ready(self, force_refresh: bool = False) -> bool:
        """
        Check if CYNIC is ready to handle requests.

        Performs health check + verifies state is accessible.
        Caches result for 60 seconds to avoid repeated queries.

        Args:
            force_refresh: Skip cache and do fresh check

        Returns:
            True if CYNIC is healthy and ready
        """
        try:
            if not force_refresh and self._state_cache and not self._state_cache.is_stale():
                return self._state_cache.healthy

            # Fresh health check
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

        Args:
            question: The question
            context: Optional context (code snippet, etc)
            reality: Reality dimension (CODE, CYNIC, SOLANA, etc)

        Returns:
            {q_score, verdict, confidence, judgment_id, ...}
        """
        try:
            async with self.session.post(
                f"{self.cynic_url}/judge",
                json={"text": question, "context": context, "reality": reality},
            ) as resp:
                if resp.status != 200:
                    return {"error": f"HTTP {resp.status}"}

                result = await resp.json()
                judgment_id = result.get("judgment_id")

                # Cache judgment for later learning
                if judgment_id:
                    self._judgment_cache[judgment_id] = result

                return result
        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    async def teach_cynic(
        self, judgment_id: str, rating: float, comment: Optional[str] = None
    ) -> dict[str, Any]:
        """
        Teach CYNIC by providing feedback on a judgment.

        Args:
            judgment_id: ID of judgment to learn from
            rating: Feedback (-1.0 to 1.0)
            comment: Optional explanation

        Returns:
            {status, qtable_updated, new_q_score, ...}
        """
        try:
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

        Args:
            count: Number of iterations
            seed: Optional random seed

        Returns:
            {job_id, status, message, count}
        """
        try:
            async with self.session.post(
                f"{self.cynic_url}/empirical/test/start", json={"count": count, "seed": seed}
            ) as resp:
                if resp.status != 200:
                    return {"error": f"HTTP {resp.status}"}

                return await resp.json()
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

                    # Wait before polling again
                    await asyncio.sleep(poll_interval_s)

            except (aiohttp.ClientError, asyncio.TimeoutError) as e:
                return {"error": str(e)}

        return {"error": f"Test timeout after {max_wait_s}s"}

    async def get_test_results(self, job_id: str) -> dict[str, Any]:
        """
        Get completed test results.

        Args:
            job_id: Test job ID

        Returns:
            {q_scores, avg_q, learning_efficiency, emergences, duration_s}
        """
        try:
            async with self.session.get(
                f"{self.cynic_url}/empirical/test/{job_id}/results"
            ) as resp:
                if resp.status != 200:
                    return {"error": f"HTTP {resp.status}"}

                return await resp.json()
        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    async def test_axiom_irreducibility(
        self, axiom: Optional[str] = None
    ) -> dict[str, Any]:
        """
        Test if axioms are irreducible.

        Args:
            axiom: Specific axiom or None for all

        Returns:
            {axiom_impacts: [{name, baseline_q, disabled_q, impact_percent, irreducible}]}
        """
        try:
            async with self.session.post(
                f"{self.cynic_url}/empirical/axioms/test", json={"axiom": axiom}
            ) as resp:
                if resp.status != 200:
                    return {"error": f"HTTP {resp.status}"}

                return await resp.json()
        except (aiohttp.ClientError, asyncio.TimeoutError) as e:
            return {"error": str(e)}

    # ════════════════════════════════════════════════════════════════════════════
    # TELEMETRY
    # ════════════════════════════════════════════════════════════════════════════

    async def query_telemetry(self, metric: str = "uptime_s") -> dict[str, Any]:
        """
        Query SONA telemetry metrics.

        Args:
            metric: Metric name (uptime_s, q_table_entries, total_judgments, learning_rate)

        Returns:
            {metric, value, ...}
        """
        try:
            async with self.session.get(
                f"{self.cynic_url}/empirical/telemetry", params={"metric": metric}
            ) as resp:
                if resp.status != 200:
                    return {"error": f"HTTP {resp.status}"}

                return await resp.json()
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
