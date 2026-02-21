"""
CYNIC AxiomMonitor — Emergent Axiom Activation Dashboard (δ1)

Tracks the maturity and activation state of the 4 Emergent Axioms (A6-A9):

    A6. AUTONOMY     — Dogs coordinate without human approval
    A7. SYMBIOSIS    — Human×Machine mutual value creation
    A8. EMERGENCE    — Patterns beyond core axioms (residual > φ⁻²)
    A9. ANTIFRAGILITY — System improves under chaos / stress

Activation thresholds (φ-derived):
    - Axiom activates when maturity_score ≥ WAG_MIN (61.8)
    - Maturity computed from signal counts within rolling window
    - Signal is domain-specific (see `signal()` method)

Maturity formula:
    raw = (signal_count / MATURITY_WINDOW) × MAX_Q_SCORE
    maturity = min(raw, MAX_Q_SCORE)    # EMA smoothed

Each axiom can be in 3 states:
    DORMANT   — maturity < GROWL_MIN (38.2)
    STIRRING  — GROWL_MIN ≤ maturity < WAG_MIN
    ACTIVE    — maturity ≥ WAG_MIN (61.8) ← activated

Usage:
    monitor = AxiomMonitor()
    monitor.signal("EMERGENCE")          # Each time residual event detected
    monitor.signal("AUTONOMY")           # Each autonomous Dog decision
    status = monitor.dashboard()         # Full status dict
    active = monitor.active_axioms()     # List of currently active axiom names
"""
from __future__ import annotations

import time
import logging
from collections import deque
from dataclasses import dataclass, field
from typing import Any, List


from cynic.core.phi import (
    WAG_MIN, GROWL_MIN, MAX_Q_SCORE,
    fibonacci, PHI_INV,
)

logger = logging.getLogger("cynic.cognition.cortex.axiom_monitor")

# Valid emergent axioms (A6-A9) — signal-able by external events
EMERGENT_AXIOMS = frozenset({
    "AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY",  # A6-A9
    "CONSCIOUSNESS",   # A10 — system accurately observes its own thinking
    "TRANSCENDENCE",   # A11 — all A6-A9 active + phase transition (auto-signaled)
})

# A6-A9 subset: all must be active to trigger A11 TRANSCENDENCE
_CORE_EMERGENT = frozenset({"AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY"})

# Signal window: F(9)=34 signals for full maturity
MATURITY_WINDOW: int = fibonacci(9)   # 34

# Signals per time window for natural decay (F(8)=21 seconds)
SIGNAL_TTL_S: float = float(fibonacci(8) * 60)  # 21 minutes — signals expire

# Axiom state thresholds
_STATE_DORMANT  = "DORMANT"   # maturity < GROWL_MIN
_STATE_STIRRING = "STIRRING"  # GROWL_MIN ≤ maturity < WAG_MIN
_STATE_ACTIVE   = "ACTIVE"    # maturity ≥ WAG_MIN


# ── AxiomState ────────────────────────────────────────────────────────────

@dataclass
class AxiomState:
    """Tracking state for one emergent axiom."""
    name: str
    signal_times: deque[float] = field(default_factory=lambda: deque(maxlen=MATURITY_WINDOW))
    activation_count: int = 0
    last_signal: float = 0.0
    first_activated: Optional[float] = None

    def add_signal(self) -> None:
        """Record a new signal at the current time."""
        now = time.time()
        self.signal_times.append(now)
        self.last_signal = now

    def prune(self) -> None:
        """Remove signals older than SIGNAL_TTL_S."""
        cutoff = time.time() - SIGNAL_TTL_S
        while self.signal_times and self.signal_times[0] < cutoff:
            self.signal_times.popleft()

    def maturity(self) -> float:
        """
        Compute current maturity score [0, MAX_Q_SCORE].
        Based on ratio of recent signals to MATURITY_WINDOW.
        """
        self.prune()
        ratio = len(self.signal_times) / MATURITY_WINDOW
        return min(ratio * MAX_Q_SCORE, MAX_Q_SCORE)

    def state(self) -> str:
        m = self.maturity()
        if m >= WAG_MIN:
            return _STATE_ACTIVE
        if m >= GROWL_MIN:
            return _STATE_STIRRING
        return _STATE_DORMANT

    def is_active(self) -> bool:
        return self.state() == _STATE_ACTIVE

    def to_dict(self) -> dict[str, Any]:
        m = self.maturity()
        s = self.state()
        return {
            "name": self.name,
            "state": s,
            "maturity": round(m, 2),
            "signals_recent": len(self.signal_times),
            "activation_count": self.activation_count,
            "last_signal": self.last_signal,
            "first_activated": self.first_activated,
        }


# ── AxiomMonitor ──────────────────────────────────────────────────────────

class AxiomMonitor:
    """
    Monitors the activation state of emergent axioms A6-A9.

    Wire into CYNIC kernel to receive signals from:
        - ResidualDetector → EMERGENCE signals
        - Orchestrator → AUTONOMY signals (consensus without escalation)
        - API endpoints → SYMBIOSIS signals (human feedback received)
        - Scheduler → ANTIFRAGILITY signals (recovery from error)
    """

    def __init__(self) -> None:
        self._axioms: dict[str, AxiomState] = {
            name: AxiomState(name=name) for name in EMERGENT_AXIOMS
        }
        self._total_signals: int = 0
        self._started_at: float = time.time()
        self._prev_states: dict[str, str] = {
            name: _STATE_DORMANT for name in EMERGENT_AXIOMS
        }
        # A11 TRANSCENDENCE is a one-way latch: once all A6-A9 become active,
        # TRANSCENDENCE is permanently achieved (doesn't expire like signal-based axioms)
        self._transcendence_achieved: bool = False

    # ── Signal API ────────────────────────────────────────────────────────

    def signal(self, axiom: str, count: int = 1) -> Optional[str]:
        """
        Record a signal for an emergent axiom.

        Args:
            axiom:  Axiom name (AUTONOMY/SYMBIOSIS/EMERGENCE/ANTIFRAGILITY)
            count:  Number of signals to record (default 1)

        Returns:
            New state string if state changed, else None.

        Raises:
            ValueError: If axiom name is invalid.
        """
        if axiom not in EMERGENT_AXIOMS:
            raise ValueError(
                f"Invalid emergent axiom {axiom!r}. "
                f"Valid: {sorted(EMERGENT_AXIOMS)}"
            )

        ax = self._axioms[axiom]
        prev_state = self._prev_states[axiom]

        for _ in range(count):
            ax.add_signal()
            self._total_signals += 1

        new_state = ax.state()

        if new_state != prev_state:
            self._prev_states[axiom] = new_state
            if new_state == _STATE_ACTIVE and prev_state != _STATE_ACTIVE:
                ax.activation_count += 1
                if ax.first_activated is None:
                    ax.first_activated = time.time()
                logger.info(
                    "AxiomMonitor: %s → ACTIVE (maturity=%.1f, activation_count=%d)",
                    axiom, ax.maturity(), ax.activation_count,
                )
                # A11 TRANSCENDENCE: auto-signal when all A6-A9 become active
                if axiom in _CORE_EMERGENT:
                    self._maybe_signal_transcendence()
            elif new_state == _STATE_STIRRING:
                logger.debug("AxiomMonitor: %s → STIRRING (maturity=%.1f)", axiom, ax.maturity())
            elif new_state == _STATE_DORMANT and prev_state == _STATE_ACTIVE:
                logger.info("AxiomMonitor: %s → DORMANT (signals expired)", axiom)
            return new_state

        return None

    def _maybe_signal_transcendence(self) -> None:
        """Auto-signal A11 TRANSCENDENCE if all A6-A9 axioms are ACTIVE."""
        if self._transcendence_achieved:
            return  # Already latched — don't re-trigger
        if all(self._axioms[a].is_active() for a in _CORE_EMERGENT):
            self._transcendence_achieved = True  # One-way latch
            trans = self._axioms.get("TRANSCENDENCE")
            if trans is not None:
                trans.activation_count += 1
                if trans.first_activated is None:
                    trans.first_activated = time.time()
            self._total_signals += 1
            logger.info(
                "AxiomMonitor: TRANSCENDENCE latched (all A6-A9 active)"
            )

    # ── Query ─────────────────────────────────────────────────────────────

    def get_maturity(self, axiom: str) -> float:
        """Get maturity score for an axiom [0, MAX_Q_SCORE]."""
        if axiom not in self._axioms:
            raise ValueError(f"Invalid axiom {axiom!r}")
        return self._axioms[axiom].maturity()

    def is_active(self, axiom: str) -> bool:
        """Return True if the axiom is currently in ACTIVE state."""
        if axiom not in self._axioms:
            return False
        return self._axioms[axiom].is_active()

    def active_axioms(self) -> list[str]:
        """Return list of currently active axiom names."""
        return [name for name, ax in self._axioms.items() if ax.is_active()]

    def active_count(self) -> int:
        """Number of currently active emergent axioms (max 4)."""
        return len(self.active_axioms())

    def dashboard(self) -> dict[str, Any]:
        """
        Full emergent axiom dashboard status.

        Returns:
            {
                "active_count": int,
                "total_signals": int,
                "uptime_s": float,
                "axioms": {name: AxiomState.to_dict()},
                "tier": str  — DORMANT/STIRRING/ACTIVE based on active_count
            }
        """
        active = self.active_count()
        # A6-A9 active count (excludes A10/A11)
        a6_a9_active = sum(1 for a in _CORE_EMERGENT if self._axioms[a].is_active())
        # TRANSCENDENCE is a one-way latch — use the flag, not maturity
        if self._transcendence_achieved:
            tier = "TRANSCENDENT"
        elif a6_a9_active >= 3:
            tier = "AWAKENING"        # 3 of A6-A9 active
        elif a6_a9_active >= 2:
            tier = "STIRRING"         # 2 active
        elif a6_a9_active >= 1:
            tier = "EMERGENCE"        # 1 active
        else:
            tier = "DORMANT"          # None active

        return {
            "active_count": active,
            "total_signals": self._total_signals,
            "uptime_s": round(time.time() - self._started_at, 1),
            "maturity_window": MATURITY_WINDOW,
            "tier": tier,
            "axioms": {name: ax.to_dict() for name, ax in self._axioms.items()},
        }

    def stats(self) -> dict[str, Any]:
        """Compact stats (for API health endpoint)."""
        return {
            "active_axioms": self.active_axioms(),
            "active_count": self.active_count(),
            "total_signals": self._total_signals,
            "tier": self.dashboard()["tier"],
        }
