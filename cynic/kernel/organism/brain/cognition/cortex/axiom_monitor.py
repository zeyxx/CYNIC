"""
CYNIC AxiomMonitor " Emergence and Transcendence Tracking (3)

Monitors the activation levels of emergent axioms (A6-A9).
Signals decay over time unless reinforced by system events.
When all A6-A9 axioms are ACTIVE, TRANSCENDENCE (A11) is achieved.
"""

from __future__ import annotations

import logging
import time
from collections import deque
from dataclasses import dataclass, field
from typing import Any

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.formulas import AXIOM_MATURITY_WINDOW_SIZE, SIGNAL_TTL_SEC
from cynic.kernel.core.phi import GROWL_MIN, MAX_Q_SCORE, WAG_MIN

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.axiom_monitor")

# Valid emergent axioms (A6-A9) " signal-able by external events
EMERGENT_AXIOMS = frozenset(
    {
        "AUTONOMY",
        "SYMBIOSIS",
        "EMERGENCE",
        "ANTIFRAGILITY",
        "CONSCIOUSNESS",
        "TRANSCENDENCE",
    }
)

_CORE_EMERGENT = frozenset({"AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY"})

# Axiom state thresholds
_STATE_DORMANT = "DORMANT"
_STATE_STIRRING = "STIRRING"
_STATE_ACTIVE = "ACTIVE"


@dataclass
class AxiomState:
    name: str
    signal_times: deque[float] = field(
        default_factory=lambda: deque(maxlen=AXIOM_MATURITY_WINDOW_SIZE)
    )
    activation_count: int = 0
    last_signal: float = 0.0
    first_activated: float | None = None

    def add_signal(self) -> None:
        now = time.time()
        self.signal_times.append(now)
        self.last_signal = now

    def prune(self) -> None:
        cutoff = time.time() - SIGNAL_TTL_SEC
        while self.signal_times and self.signal_times[0] < cutoff:
            self.signal_times.popleft()

    def maturity(self) -> float:
        self.prune()
        count = len(self.signal_times)
        return (count / AXIOM_MATURITY_WINDOW_SIZE) * MAX_Q_SCORE

    def state(self) -> str:
        m = self.maturity()
        if m >= WAG_MIN:
            return _STATE_ACTIVE
        if m >= GROWL_MIN:
            return _STATE_STIRRING
        return _STATE_DORMANT

    def is_active(self) -> bool:
        return self.state() == _STATE_ACTIVE


class AxiomMonitor:
    """Tracks the evolution of CYNIC's emergent philosophy."""

    def __init__(self, bus: EventBus) -> None:
        self._axioms = {name: AxiomState(name=name) for name in EMERGENT_AXIOMS}
        self._total_signals = 0
        self._transcendence_achieved = False
        self._bus = bus

    async def signal(
        self, axiom: str, source: str = "internal", count: int = 1, **kwargs: Any
    ) -> str | None:
        """Record a signal for an emergent axiom."""
        if axiom not in self._axioms:
            return None

        ax = self._axioms[axiom]
        prev_state = ax.state()

        for _ in range(count):
            ax.add_signal()
            self._total_signals += 1

        new_state = ax.state()
        if new_state != prev_state:
            if new_state == _STATE_ACTIVE:
                logger.info(f"AXIOM ACTIVATED: {axiom} triggered by {source}")
                if axiom in _CORE_EMERGENT:
                    self._maybe_signal_transcendence()

            await self._bus.emit(
                Event.typed(
                    CoreEvent.AXIOM_ACTIVATED,
                    {
                        "axiom": axiom,
                        "source": source,
                        "new_state": new_state,
                        **kwargs,
                    },
                    source="axiom_monitor",
                )
            )

        return new_state

    def _maybe_signal_transcendence(self) -> None:
        if self._transcendence_achieved:
            return
        if all(self._axioms[a].is_active() for a in _CORE_EMERGENT):
            self._transcendence_achieved = True
            logger.warning("!!! TRANSCENDENCE ACHIEVED !!!")

    def active_axioms(self) -> list[str]:
        return [name for name, ax in self._axioms.items() if ax.is_active()]

    def dashboard(self) -> dict[str, Any]:
        return {
            name: {"maturity": ax.maturity(), "state": ax.state()}
            for name, ax in self._axioms.items()
        }
