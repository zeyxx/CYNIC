"""CYNIC SelfWatcher — CYNIC×LEARN/MICRO every F(10)=55s."""
from __future__ import annotations

from collections.abc import Callable

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci
from cynic.senses.workers.base import PerceiveWorker
from typing import Optional


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

    def __init__(self, qtable_getter: Optional[Callable] = None) -> None:
        self._qtable_getter = qtable_getter

    async def sense(self) -> Optional[Cell]:
        if self._qtable_getter is None:
            return None

        try:
            qtable = self._qtable_getter()
            stats = qtable.stats()
        except Exception:
            return None

        # Escalate budget to MACRO if learning health looks poor
        # (few states discovered or low confidence = needs deep reasoning)
        states = stats.get("states", 0)
        max_confidence = stats.get("max_confidence", 0.0)
        is_learning_weak = states < 10 or max_confidence < 0.3
        budget_usd = 0.10 if is_learning_weak else 0.003

        return Cell(
            reality="CYNIC",
            analysis="LEARN",
            time_dim="PRESENT",
            content={
                "states": states,
                "total_updates": stats.get("total_updates", 0),
                "pending_flush": stats.get("pending_flush", 0),
                "max_confidence": max_confidence,
                "unique_states": stats.get("unique_states", 0),
                "learning_health": "WEAK" if is_learning_weak else "NORMAL",
            },
            context=(
                f"Self-watcher: {states} states learned, "
                f"{stats.get('total_updates', 0)} total updates, "
                f"max_confidence={max_confidence:.2f}"
            ),
            risk=0.5 if is_learning_weak else 0.0,
            complexity=0.5 if is_learning_weak else 0.2,
            budget_usd=budget_usd,
            metadata={"source": "self_watcher", "escalation": is_learning_weak},
        )
