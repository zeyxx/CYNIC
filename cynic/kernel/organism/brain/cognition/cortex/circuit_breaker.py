"""
CYNIC Circuit Breaker â€" Cascade Failure Protection (topology M1)

When the judgment pipeline fails repeatedly, the circuit opens to prevent
resource exhaustion and cascade failures. Closes after a Ï-derived cooldown.

States (3 â€" like the 3 non-HOWL verdict bands):
  CLOSED    â' Normal operation. Track consecutive failures.
  OPEN      â' Fast-fail. No new judgments until cooldown elapses.
  HALF_OPEN â' One probe allowed. Successâ'CLOSED, failureâ'OPEN.

Ï-derived constants:
  _FAILURE_THRESHOLD = fibonacci(5) = 5  consecutive failures to open
  _COOLDOWN_S        = PHI_INV_2 * 60  = 22.9s open before half-open probe

Usage:
    cb = CircuitBreaker()
    if not cb.allow():
        raise RuntimeError("circuit open")
    try:
        result = await do_work()
        cb.record_success()
        return result
    except ValidationError:
        cb.record_failure()
        raise
"""

from __future__ import annotations

import logging
import time

# Python 3.9 compatibility: StrEnum added in Python 3.11
from enum import StrEnum
from typing import Any

from cynic.kernel.core.phi import PHI_INV_2, fibonacci

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.circuit_breaker")

# Ï-derived thresholds
_FAILURE_THRESHOLD: int = fibonacci(5)  # 5 consecutive failures â' OPEN
_COOLDOWN_S: float = PHI_INV_2 * 60  # 22.9s cooldown before HALF_OPEN


class CircuitState(StrEnum):
    CLOSED = "CLOSED"
    OPEN = "OPEN"
    HALF_OPEN = "HALF_OPEN"


class CircuitBreaker:
    """
    3-state circuit breaker for the CYNIC judgment pipeline.

    Thread-safe within a single asyncio event loop (no threading needed).
    """

    def __init__(
        self,
        failure_threshold: int = _FAILURE_THRESHOLD,
        cooldown_s: float = _COOLDOWN_S,
    ) -> None:
        self._failure_threshold = failure_threshold
        self._cooldown_s = cooldown_s
        self._state = CircuitState.CLOSED
        self._failure_count: int = 0  # Consecutive failures
        self._last_opened_at: float = 0.0  # Epoch when circuit was opened
        self._probe_allowed: bool = False  # One probe flag for HALF_OPEN

    # â"€â"€ Public API â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€

    @property
    def state(self) -> CircuitState:
        return self._state

    @property
    def failure_count(self) -> int:
        return self._failure_count

    def allow(self) -> bool:
        """
        Return True if a new operation is permitted.

        CLOSED    â' always True.
        OPEN      â' False until cooldown elapses, then transition to HALF_OPEN.
        HALF_OPEN â' True for exactly one probe request.
        """
        if self._state == CircuitState.CLOSED:
            return True

        if self._state == CircuitState.OPEN:
            elapsed = time.time() - self._last_opened_at
            if elapsed >= self._cooldown_s:
                self._state = CircuitState.HALF_OPEN
                self._probe_allowed = True
                logger.info(
                    "CircuitBreaker: OPEN â' HALF_OPEN (elapsed=%.1fs >= cooldown=%.1fs)",
                    elapsed,
                    self._cooldown_s,
                )
                # Fall through to HALF_OPEN handling (probe consumed below)
            else:
                return False

        # HALF_OPEN: allow exactly one probe
        if self._probe_allowed:
            self._probe_allowed = False
            return True
        return False  # Probe already in-flight

    def record_success(self) -> None:
        """
        Record a successful operation.

        HALF_OPEN â' CLOSED (probe succeeded â€" service recovered).
        Any state  â' reset failure count.
        """
        if self._state == CircuitState.HALF_OPEN:
            logger.info("CircuitBreaker: HALF_OPEN â' CLOSED (probe succeeded)")
        self._state = CircuitState.CLOSED
        self._failure_count = 0

    def record_failure(self) -> None:
        """
        Record a failed operation.

        HALF_OPEN â' OPEN (probe failed â€" service still sick, restart cooldown).
        CLOSED    â' increment counter; open if threshold reached.
        OPEN      â' increment only (already open, cooldown not reset).
        """
        self._failure_count += 1

        if self._state == CircuitState.HALF_OPEN:
            self._state = CircuitState.OPEN
            self._last_opened_at = time.time()
            logger.warning(
                "CircuitBreaker: HALF_OPEN â' OPEN (probe failed, total failures=%d)",
                self._failure_count,
            )
        elif self._state == CircuitState.CLOSED and self._failure_count >= self._failure_threshold:
            self._state = CircuitState.OPEN
            self._last_opened_at = time.time()
            logger.warning(
                "CircuitBreaker: CLOSED â' OPEN (threshold %d reached)",
                self._failure_threshold,
            )

    def stats(self) -> dict[str, Any]:
        """Return circuit breaker health snapshot."""
        elapsed_since_open = (
            round(time.time() - self._last_opened_at, 1) if self._last_opened_at > 0 else 0.0
        )
        return {
            "state": self._state.value,
            "failure_count": self._failure_count,
            "failure_threshold": self._failure_threshold,
            "cooldown_s": round(self._cooldown_s, 1),
            "elapsed_since_open_s": elapsed_since_open,
        }

    def reset(self) -> None:
        """Force-reset to CLOSED. For testing only."""
        self._state = CircuitState.CLOSED
        self._failure_count = 0
        self._last_opened_at = 0.0
        self._probe_allowed = False
