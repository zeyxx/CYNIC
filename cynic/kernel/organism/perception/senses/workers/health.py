"""CYNIC HealthWatcher â€" CYNICÃ-PERCEIVE/REFLEX every F(8)=21s."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any, Optional

from cynic.kernel.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.phi import fibonacci
from cynic.kernel.organism.perception.senses.workers.base import PerceiveWorker


class HealthWatcher(PerceiveWorker):
    """
    Monitors CycleTimer health across all 4 consciousness levels.

    Submits CYNICÃ-PERCEIVE at REFLEX level only when a timer is
    DEGRADED or CRITICAL (not on every tick â€" only on bad news).
    This closes the self-monitoring loop: "CYNIC sees its own slowness."

    interval: F(8)=21s â€" enough resolution to detect degradation early.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(8))  # 21.0s
    name = "health_watcher"

    def __init__(self, get_consciousness_fn: Optional[Callable] = None) -> None:
        self._get_consciousness = get_consciousness_fn or get_consciousness

    async def sense(self) -> Optional[Cell]:
        consciousness = self._get_consciousness()
        degraded: dict[str, Any] = {
            name: timer.to_dict()
            for name, timer in consciousness.timers.items()
            if timer.health in ("DEGRADED", "CRITICAL")
        }

        if not degraded:
            return None

        severity_rank = {"EXCELLENT": 0, "GOOD": 1, "UNKNOWN": 1, "DEGRADED": 2, "CRITICAL": 3}
        worst = max(
            consciousness.timers.values(),
            key=lambda t: severity_rank.get(t.health, 0),
        )

        return Cell(
            reality="CYNIC",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={
                "degraded_levels": degraded,
                "worst_health": worst.health,
                "worst_p95_ms": worst.p95_ms,
                "total_cycles": consciousness.total_cycles,
            },
            context=(
                f"Health watcher: {len(degraded)} level(s) degraded "
                f"â€" worst={worst.health} p95={worst.p95_ms:.0f}ms"
            ),
            risk=0.2 if worst.health == "DEGRADED" else 0.5,
            complexity=0.3,
            budget_usd=0.001,
            metadata={"source": "health_watcher", "degraded_count": len(degraded)},
        )
