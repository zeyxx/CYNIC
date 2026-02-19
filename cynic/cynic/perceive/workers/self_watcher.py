"""CYNIC SelfWatcher — CYNIC×LEARN/MICRO every F(10)=55s."""
from __future__ import annotations

from collections.abc import Callable

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci
from cynic.perceive.workers.base import PerceiveWorker


class SelfWatcher(PerceiveWorker):
    """
    CYNIC observes its own Q-Table learning health.

    Submits CYNIC×LEARN at MICRO level every ~55s.
    Creates a self-judgment loop: "How well am I learning?"
    The judgment system judges its own learning state → feeds more Q-Learning.

    interval: F(10)=55s — ~1 minute is the right granularity for learning checks.
    """

    level = ConsciousnessLevel.MICRO
    interval_s = float(fibonacci(10))  # 55.0s
    name = "self_watcher"

    def __init__(self, qtable_getter: Callable | None = None) -> None:
        self._qtable_getter = qtable_getter

    async def sense(self) -> Cell | None:
        if self._qtable_getter is None:
            return None

        try:
            qtable = self._qtable_getter()
            stats = qtable.stats()
        except Exception:
            return None

        return Cell(
            reality="CYNIC",
            analysis="LEARN",
            time_dim="PRESENT",
            content={
                "states": stats.get("states", 0),
                "total_updates": stats.get("total_updates", 0),
                "pending_flush": stats.get("pending_flush", 0),
                "max_confidence": stats.get("max_confidence", 0.0),
                "unique_states": stats.get("unique_states", 0),
            },
            context=(
                f"Self-watcher: {stats.get('states', 0)} states learned, "
                f"{stats.get('total_updates', 0)} total updates"
            ),
            risk=0.0,
            complexity=0.2,
            budget_usd=0.003,
            metadata={"source": "self_watcher"},
        )
