"""
Meta-Cognition Handler — Organism self-awareness and adaptive learning

Listens to SONA_TICK events and responds by:
1. Updating Thompson Sampling learning rate (α) based on organism health
2. Adjusting E-Score calculations from SONA telemetry
3. Detecting stagnation patterns in learning
4. Triggering meta-level interventions (escalation, restructuring)

SONA_TICK signal path:
  sona_emitter._run_loop() every F(9)=2040s
  → emit CoreEvent.SONA_TICK with SonaTickPayload
  → MetaCognitionHandler.execute() reads payload
  → Update learning rates + E-Score
  → Organism becomes more self-aware
"""
from __future__ import annotations

import asyncio
import logging
import time
from dataclasses import dataclass
from typing import Any, Optional

from cynic.kernel.core.event_bus import Event, EventBus, CoreEvent, get_core_bus
from cynic.kernel.core.events_schema import SonaTickPayload
from cynic.kernel.core.phi import PHI, PHI_INV, fibonacci
from cynic.brain.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.brain.learning.qlearning import QTable, LearningLoop
from cynic.kernel.core.escore import EScoreTracker

logger = logging.getLogger("cynic.brain.cognition.cortex.meta_cognition")


@dataclass
class OrganismHealthMetrics:
    """Snapshot of organism health from SONA_TICK."""
    uptime_s: float
    q_table_entries: int
    total_judgments: int
    learning_rate: float
    ewc_consolidated: int
    tick_number: int

    def __post_init__(self):
        """Calculate derived metrics."""
        self.judgments_per_minute = (
            (self.total_judgments / self.uptime_s * 60) if self.uptime_s > 0 else 0
        )
        self.q_table_saturation = (
            min(1.0, self.q_table_entries / 1000.0)  # 1000 entries = full
        )


class MetaCognitionHandler(BaseHandler):
    """
    Responds to SONA_TICK heartbeat with adaptive meta-cognition.

    Responsibilities:
    1. Monitor organism health trends
    2. Update Thompson Sampling α (learning_rate) reactively
    3. Adjust E-Score from health metrics
    4. Detect learning stagnation
    5. Log organism state changes
    """

    handler_id = "meta_cognition"
    version = "1.0"
    description = "Adaptive meta-cognition responding to SONA_TICK organism heartbeat"

    def __init__(
        self,
        qtable: Optional[QTable] = None,
        learning_loop: Optional[LearningLoop] = None,
        escore_tracker: Optional[EScoreTracker] = None,
        bus: Optional[EventBus] = None,
    ) -> None:
        """
        Initialize meta-cognition handler.

        Args:
            qtable: Q-Learning table (for introspection)
            learning_loop: Learning loop coordinator (to update α)
            escore_tracker: E-Score tracker (for reputation adjustment)
            bus: Event bus (for subscription)
        """
        self.qtable = qtable
        self.learning_loop = learning_loop
        self.escore_tracker = escore_tracker
        self.bus = bus or get_core_bus()

        # ── Health tracking (rolling window, Fibonacci size) ──
        self._health_window: list[OrganismHealthMetrics] = []
        self._window_size = fibonacci(8)  # F(8) = 21 ticks
        self._ticks_processed = 0
        self._last_α_update = time.time()
        self._α_update_interval_s = 300  # Update α every 5 minutes

    def start(self) -> None:
        """Subscribe this handler to SONA_TICK events."""
        self.bus.on(CoreEvent.SONA_TICK, self._on_sona_tick)
        self._log_execution("started listening to SONA_TICK")

    async def _on_sona_tick(self, event: Event) -> None:
        """
        Event handler called on every SONA_TICK.

        Executes the meta-cognition logic asynchronously.
        """
        try:
            payload = SonaTickPayload.model_validate(event.dict_payload or {})
            await self.execute(sona_tick=payload)
        except Exception as exc:
            self._log_error("_on_sona_tick", exc)

    async def execute(self, **kwargs: Any) -> HandlerResult:
        """
        Main meta-cognition execution.

        Args:
            sona_tick: SonaTickPayload from organism heartbeat

        Returns:
            HandlerResult with health metrics and actions taken
        """
        start_time = time.time()

        sona_tick: Optional[SonaTickPayload] = kwargs.get("sona_tick")
        if not sona_tick:
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error="Missing sona_tick in kwargs",
                duration_ms=(time.time() - start_time) * 1000,
            )

        try:
            # ── Phase 1: Extract health snapshot ──────────────────────────
            health = OrganismHealthMetrics(
                uptime_s=sona_tick.uptime_s,
                q_table_entries=sona_tick.q_table_entries,
                total_judgments=sona_tick.total_judgments,
                learning_rate=sona_tick.learning_rate,
                ewc_consolidated=sona_tick.ewc_consolidated,
                tick_number=sona_tick.tick_number,
            )

            self._health_window.append(health)
            if len(self._health_window) > self._window_size:
                self._health_window.pop(0)
            self._ticks_processed += 1

            # ── Phase 2: Analyze trends ────────────────────────────────────
            trend_analysis = self._analyze_health_trends()

            # ── Phase 3: Update learning rate (α) if needed ────────────────
            actions_taken = []
            if self._should_update_learning_rate():
                α_adjustment = self._compute_α_adjustment(trend_analysis)
                if self.learning_loop and α_adjustment != 0.0:
                    self.learning_loop.adjust_learning_rate(α_adjustment)
                    actions_taken.append(f"Updated α by {α_adjustment:+.3f}")
                    self._last_α_update = time.time()

            # ── Phase 4: Check for stagnation ──────────────────────────────
            if self._detect_stagnation(trend_analysis):
                actions_taken.append("STAGNATION_DETECTED — judgment flow low")
                self._log_execution("stagnation detected", f"JP/m={health.judgments_per_minute:.1f}")

            # ── Phase 5: Adjust E-Score ───────────────────────────────────
            if self.escore_tracker:
                escore_delta = self._compute_escore_delta(trend_analysis)
                # TODO: Wire escore_tracker.adjust(delta) once API defined
                if escore_delta != 0.0:
                    actions_taken.append(f"E-Score delta: {escore_delta:+.2f}")

            # ── Return result ──────────────────────────────────────────────
            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output={
                    "tick_number": health.tick_number,
                    "health_snapshot": {
                        "uptime_s": health.uptime_s,
                        "judgments_per_minute": health.judgments_per_minute,
                        "q_table_saturation": health.q_table_saturation,
                    },
                    "trend_analysis": trend_analysis,
                    "actions_taken": actions_taken,
                },
                duration_ms=(time.time() - start_time) * 1000,
            )

        except Exception as exc:
            self._log_error("execute", exc)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(exc),
                duration_ms=(time.time() - start_time) * 1000,
            )

    # ═════════════════════════════════════════════════════════════════════════
    # ANALYSIS & DECISION LOGIC
    # ═════════════════════════════════════════════════════════════════════════

    def _should_update_learning_rate(self) -> bool:
        """Check if enough time has passed to update α."""
        return (time.time() - self._last_α_update) > self._α_update_interval_s

    def _analyze_health_trends(self) -> dict[str, Any]:
        """
        Analyze organism health from rolling window.

        Returns:
            Dictionary with trend metrics:
            - judgments_trend: "rising", "stable", "falling"
            - q_saturation: current Q-table fill %
            - learning_health: PHI-bounded confidence in learning quality
        """
        if len(self._health_window) < 2:
            return {
                "judgments_trend": "unknown",
                "q_saturation": 0.0,
                "learning_health": 0.5,
                "judgments_per_second": 0.0,
                "uptime_delta": 0.0,
                "window_size": len(self._health_window),
            }

        # Trend analysis (simple slope)
        oldest = self._health_window[0].total_judgments
        newest = self._health_window[-1].total_judgments
        judgments_delta = newest - oldest
        uptime_delta = (
            self._health_window[-1].uptime_s - self._health_window[0].uptime_s
        )

        if uptime_delta > 0:
            judgments_per_second = judgments_delta / uptime_delta
        else:
            judgments_per_second = 0.0

        # Classify trend
        if judgments_per_second > 0.01:  # >36 judgments/hour
            judgments_trend = "rising"
        elif judgments_per_second < -0.001:  # Negative (unlikely but possible)
            judgments_trend = "falling"
        else:
            judgments_trend = "stable"

        # Q-table saturation
        latest = self._health_window[-1]
        q_saturation = latest.q_table_saturation

        # Learning health (how well is the system learning?)
        # φ-bounded: max confidence 61.8% in learning quality
        learning_health = min(
            PHI_INV,  # 61.8% max
            (q_saturation * 0.5) + (min(1.0, latest.judgments_per_minute / 100) * 0.3) +
            (min(1.0, latest.ewc_consolidated / 100) / 1000 * 0.2)  # EWC contribution small
        )

        return {
            "judgments_trend": judgments_trend,
            "q_saturation": q_saturation,
            "judgments_per_second": judgments_per_second,
            "learning_health": learning_health,
            "uptime_delta": uptime_delta,
            "window_size": len(self._health_window),
        }

    def _compute_α_adjustment(self, trend: dict[str, Any]) -> float:
        """
        Compute Thompson Sampling α (learning rate) adjustment.

        Increase α when:
        - System is learning well (rising judgments, high Q-saturation)
        - EWC is consolidated (learned patterns are locked)

        Decrease α when:
        - Learning is stagnant
        - System is overwhelmed
        """
        base_adjustment = 0.0

        if trend["judgments_trend"] == "rising":
            base_adjustment += 0.01  # Encourage more exploration
        elif trend["judgments_trend"] == "falling":
            base_adjustment -= 0.01  # Reduce exploration, focus on depth

        # Saturation adjustment
        saturation = trend["q_saturation"]
        if saturation > 0.7:
            base_adjustment += 0.005  # Mostly learned, explore new areas
        elif saturation < 0.2:
            base_adjustment -= 0.005  # Still learning basics, slow down

        # Learning health adjustment
        health = trend["learning_health"]
        if health > 0.6:  # Good learning
            base_adjustment += 0.005
        elif health < 0.3:  # Poor learning
            base_adjustment -= 0.01

        # Cap adjustment φ-bounded
        return max(-PHI_INV, min(PHI_INV, base_adjustment))

    def _detect_stagnation(self, trend: dict[str, Any]) -> bool:
        """
        Detect if the organism is in a learning plateau.

        Stagnation = stable or falling judgments + no new Q entries + low EWC
        """
        if len(self._health_window) < 5:  # Need history
            return False

        recent = self._health_window[-5:]
        judgment_counts = [h.total_judgments for h in recent]
        q_entries = [h.q_table_entries for h in recent]

        # Check if judgment count is stalled
        judgment_delta = judgment_counts[-1] - judgment_counts[0]
        if judgment_delta < 5:  # <5 judgments in last 5 ticks = stalled
            return True

        # Check if Q-table stopped growing
        q_delta = q_entries[-1] - q_entries[0]
        if q_delta == 0 and q_entries[-1] < 50:  # No growth AND small table
            return True

        return False

    def _compute_escore_delta(self, trend: dict[str, Any]) -> float:
        """
        Compute E-Score adjustment based on health.

        E-Score should increase when:
        - Learning is active (q_saturation growing)
        - Judgments flowing (judgments_per_second > threshold)
        - Health metrics improving

        E-Score should decrease when:
        - Stagnation detected
        - Learning health declining
        """
        delta = 0.0

        # Judgment flow contribution
        if trend["judgments_per_second"] > 0.01:
            delta += 0.1
        elif trend["judgments_per_second"] < 0.001:
            delta -= 0.05

        # Learning health contribution
        health = trend["learning_health"]
        if health > 0.5:
            delta += 0.05 * (health - 0.5)  # Proportional
        else:
            delta -= 0.05 * (0.5 - health)  # Penalty

        return max(-1.0, min(1.0, delta))

    # ═════════════════════════════════════════════════════════════════════════
    # LOGGING & OBSERVABILITY
    # ═════════════════════════════════════════════════════════════════════════

    def stats(self) -> dict[str, Any]:
        """Return meta-cognition statistics."""
        return {
            "handler_id": self.handler_id,
            "ticks_processed": self._ticks_processed,
            "health_window_size": len(self._health_window),
            "last_α_update_s_ago": time.time() - self._last_α_update,
        }
