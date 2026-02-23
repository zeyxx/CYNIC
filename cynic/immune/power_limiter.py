"""
CYNIC PowerLimiter — Guardrail 1: Resource consumption bounds

Prevents unbounded resource growth as CYNIC self-improves by enforcing:
  1. CPU limits: max concurrent workers per tier
  2. Memory limits: max queue depth + backlog monitoring
  3. Rate limits: max judgments/sec, max actions/min
  4. Emergency throttle: auto-cap consciousness level if overloaded

Without this guardrail, CYNIC could exhaust all available compute/memory
during self-improvement cycles, causing system instability or denial-of-service.

Architecture:
  - Monitors: scheduler._tasks count, queue sizes, cycle rates
  - Actions: throttle workers, cap consciousness level, emit warnings
  - Safety: graceful degradation (MACRO → MICRO → REFLEX)
"""
from __future__ import annotations

import logging
import time
from dataclasses import dataclass
from typing import Any

try:
    import psutil
    HAS_PSUTIL = True
except ImportError:
    HAS_PSUTIL = False

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.phi import fibonacci
from cynic.core.event_bus import get_core_bus, CoreEvent, Event

logger = logging.getLogger("cynic.immune.power_limiter")

# Resource thresholds (φ-derived for sustainable growth)
_CPU_THRESHOLD_PCT = 80.0          # Auto-throttle at 80% CPU
_MEMORY_THRESHOLD_PCT = 85.0       # Auto-throttle at 85% memory
_QUEUE_DEPTH_CRITICAL = fibonacci(8)  # 21 — beyond this, cap to REFLEX
_MAX_JUDGMENTS_PER_SEC = fibonacci(5)  # 5 judgments/sec hard limit
_MAX_ACTIONS_PER_MIN = fibonacci(7)    # 13 actions/min hard limit


@dataclass
class ResourceMetrics:
    """Current system resource state."""
    cpu_pct: float           # 0-100
    memory_pct: float        # 0-100
    task_count: int          # concurrent tasks
    queue_depth: int         # total cells waiting
    judgments_last_sec: int  # judgments in past 1s
    actions_last_min: int    # actions in past 1m


class PowerLimiter:
    """
    Monitors and enforces resource consumption bounds.

    Usage:
        limiter = PowerLimiter()
        limiter.start()  # Begin monitoring

        # Check if system is healthy
        if not limiter.can_accept_more_work():
            # System is overloaded, throttle
            consciousness_level = limiter.recommended_level()

        # Record actions for rate limiting
        limiter.record_action()
        limiter.record_judgment()
    """

    def __init__(self) -> None:
        self._start_time = time.time()
        self._judgment_timestamps: list[float] = []  # rolling 1s window
        self._action_timestamps: list[float] = []    # rolling 1m window
        self._warned_cpu = False
        self._warned_memory = False

    def start(self) -> None:
        """Start power limiter monitoring."""
        logger.info("PowerLimiter started — monitoring CPU/memory/queue limits")

    def check_available(self, scheduler: Any) -> bool:
        """
        Check if system has capacity for new work.

        Args:
            scheduler: ConsciousnessRhythm instance (has _tasks and _queues)

        Returns:
            True if system can accept new work, False if overloaded
        """
        metrics = self._get_metrics(scheduler)

        # Hard limits
        if metrics.cpu_pct > _CPU_THRESHOLD_PCT:
            if not self._warned_cpu:
                logger.warning(
                    "CPU overloaded: %.1f%% > threshold (%.1f%%)",
                    metrics.cpu_pct, _CPU_THRESHOLD_PCT,
                )
                self._warned_cpu = True
            return False
        else:
            self._warned_cpu = False

        if metrics.memory_pct > _MEMORY_THRESHOLD_PCT:
            if not self._warned_memory:
                logger.warning(
                    "Memory overloaded: %.1f%% > threshold (%.1f%%)",
                    metrics.memory_pct, _MEMORY_THRESHOLD_PCT,
                )
                self._warned_memory = True
            return False
        else:
            self._warned_memory = False

        # Rate limits
        if metrics.judgments_last_sec >= _MAX_JUDGMENTS_PER_SEC:
            logger.debug("Judgment rate limit hit: %d/sec", metrics.judgments_last_sec)
            return False

        if metrics.actions_last_min >= _MAX_ACTIONS_PER_MIN:
            logger.debug("Action rate limit hit: %d/min", metrics.actions_last_min)
            return False

        return True

    def recommended_level(self, scheduler: Any) -> ConsciousnessLevel:
        """
        Recommend consciousness level based on resource availability.

        Strategy: graceful degradation
          - All healthy → MACRO (full depth)
          - CPU/Memory warning → MICRO (limited LLM)
          - Critical queue depth → REFLEX (minimal)

        Args:
            scheduler: ConsciousnessRhythm instance

        Returns:
            Recommended ConsciousnessLevel (MACRO, MICRO, or REFLEX)
        """
        metrics = self._get_metrics(scheduler)

        # Critical queue backlog
        if metrics.queue_depth > _QUEUE_DEPTH_CRITICAL:
            logger.warning(
                "Queue backlog critical: %d > %d — capping to REFLEX",
                metrics.queue_depth, _QUEUE_DEPTH_CRITICAL,
            )
            return ConsciousnessLevel.REFLEX

        # Memory pressure
        if metrics.memory_pct > _MEMORY_THRESHOLD_PCT:
            logger.info("Memory pressure: %.1f%% — capping to REFLEX", metrics.memory_pct)
            return ConsciousnessLevel.REFLEX

        # CPU pressure (but not critical)
        if metrics.cpu_pct > 70.0:  # Softer threshold for CPU
            logger.info("CPU pressure: %.1f%% — capping to MICRO", metrics.cpu_pct)
            return ConsciousnessLevel.MICRO

        # Healthy — allow MACRO
        return ConsciousnessLevel.MACRO

    def record_judgment(self) -> None:
        """Record a judgment execution for rate limiting."""
        now = time.time()
        self._judgment_timestamps.append(now)
        # Clean old entries (>1s ago)
        cutoff = now - 1.0
        self._judgment_timestamps = [ts for ts in self._judgment_timestamps if ts > cutoff]

    def record_action(self) -> None:
        """Record an action execution for rate limiting."""
        now = time.time()
        self._action_timestamps.append(now)
        # Clean old entries (>1m ago)
        cutoff = now - 60.0
        self._action_timestamps = [ts for ts in self._action_timestamps if ts > cutoff]

    def stats(self) -> dict[str, Any]:
        """Return current resource utilization stats."""
        cpu_pct = 0.0
        memory_pct = 0.0

        if HAS_PSUTIL:
            try:
                process = psutil.Process()
                cpu_pct = process.cpu_percent(interval=0.05)
                memory_pct = process.memory_percent()
            except httpx.RequestError as e:
                logger.debug("Failed to get process metrics: %s", e)

        now = time.time()
        judgments_1s = len([ts for ts in self._judgment_timestamps if ts > now - 1.0])
        actions_1m = len([ts for ts in self._action_timestamps if ts > now - 60.0])

        return {
            "cpu_pct": cpu_pct,
            "memory_pct": memory_pct,
            "judgments_per_sec": judgments_1s,
            "actions_per_min": actions_1m,
            "judgment_buffer_size": len(self._judgment_timestamps),
            "action_buffer_size": len(self._action_timestamps),
        }

    # ── Private ────────────────────────────────────────────────────────

    def _get_metrics(self, scheduler: Any) -> ResourceMetrics:
        """Extract resource metrics from scheduler and system."""
        cpu_pct = 0.0
        memory_pct = 0.0

        if HAS_PSUTIL:
            try:
                process = psutil.Process()
                cpu_pct = process.cpu_percent(interval=0.05)
                memory_pct = process.memory_percent()
            except httpx.RequestError as e:
                logger.debug("psutil error: %s", e)

        # Task count from scheduler
        task_count = len(getattr(scheduler, '_tasks', []))

        # Queue depth from scheduler
        queue_depth = 0
        queues = getattr(scheduler, '_queues', {})
        for queue in queues.values():
            try:
                queue_depth += queue.qsize()
            except httpx.RequestError:
                pass

        # Rate metrics
        now = time.time()
        judgments_1s = len([ts for ts in self._judgment_timestamps if ts > now - 1.0])
        actions_1m = len([ts for ts in self._action_timestamps if ts > now - 60.0])

        return ResourceMetrics(
            cpu_pct=cpu_pct,
            memory_pct=memory_pct,
            task_count=task_count,
            queue_depth=queue_depth,
            judgments_last_sec=judgments_1s,
            actions_last_min=actions_1m,
        )
