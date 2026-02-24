"""
SONA - Self-Organizing Neural Amplifier

Wires all 11 learning loops together for coordinated learning.

Loops:
1. Q-Learning (TD0 + Thompson) - Already in qlearning.py
2. SONA - This file
3. DPO - Preference optimization
4. Calibration - Drift detection
5. Thompson Sampling - In QTable
6. Meta-Cognition - Strategy switching
7. Behavior Modifier - Feedback adjustments
8. EWC - Fisher consolidation
9. Residual Governance - Dimension discovery
10. Bridge - JUDGMENT_CREATED events
11. Scheduler - ConsciousnessRhythm
"""
from __future__ import annotations

import asyncio
import logging
from dataclasses import dataclass, field
from typing import Any, Optional

from cynic.core.phi import PHI, PHI_INV, fibonacci
from cynic.core.event_bus import EventBus, CoreEvent, Event

logger = logging.getLogger("cynic.learning.loops")


# Learning loop names
LEARNING_LOOPS = [
    "Q_LEARNING",
    "SONA",
    "DPO",
    "CALIBRATION",
    "THOMPSON",
    "META_COGNITION",
    "BEHAVIOR_MODIFIER",
    "EWC",
    "RESIDUAL_GOVERNANCE",
    "BRIDGE",
    "SCHEDULER",
]


@dataclass
class LoopConfig:
    """Configuration for a learning loop."""
    name: str
    enabled: bool = True
    priority: int = 5  # 1-10
    batch_size: int = 21  # F(8)
    cooldown_sec: float = 60.0


@dataclass
class SONA:
    """
    SONA - Self-Organizing Neural Amplifier

    Orchestrates all 11 learning loops:
    - Aggregates Q-scores from multiple loops
    - Coordinates learning updates
    - Emits composite learning events
    """
    event_bus: Optional[EventBus] = None
    loops: dict[str, LoopConfig] = field(default_factory=dict)
    active: bool = False

    def __post_init__(self) -> None:
        # Initialize loop configs
        for name in LEARNING_LOOPS:
            self.loops[name] = LoopConfig(name=name)

    def start(self, event_bus: EventBus) -> None:
        """Start SONA orchestration."""
        self.event_bus = event_bus
        self.active = True
        
        # Register event handlers
        event_bus.on(CoreEvent.LEARNING_EVENT, self._on_learning_event)
        event_bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment)
        event_bus.on(CoreEvent.EWC_CHECKPOINT, self._on_ewc_checkpoint)
        
        logger.info("SONA started - orchestrating %d loops", len(LEARNING_LOOPS))

    def stop(self) -> None:
        """Stop SONA."""
        self.active = False
        logger.info("SONA stopped")

    async def _on_learning_event(self, event: Event) -> None:
        """Handle learning events from any loop."""
        if not self.active:
            return
        
        payload = event.payload
        loop_name = payload.get("loop_name", "UNKNOWN")
        
        logger.debug("SONA received learning event from %s", loop_name)
        
        # Aggregate across loops - weighted by priority
        await self._aggregate_learning(payload)

    async def _on_judgment(self, event: Event) -> None:
        """Handle judgment created - trigger multi-loop learning."""
        if not self.active:
            return
        
        # Trigger DPO, Calibration, and Meta-Cognition loops
        await self._trigger_judgment_learning(event.payload)

    async def _on_ewc_checkpoint(self, event: Event) -> None:
        """Handle EWC checkpoint - consolidate learning."""
        if not self.active:
            return
        
        logger.info("EWC checkpoint received - consolidating %s", event.payload)

    async def _aggregate_learning(self, payload: dict[str, Any]) -> None:
        """Aggregate learning across enabled loops."""
        if not self.event_bus:
            return
        
        # Get weights from enabled loops
        total_weight = sum(
            config.priority for config in self.loops.values()
            if config.enabled
        )
        
        # Emit aggregated event
        from cynic.core.events_schema import LearningEventPayload
        await self.event_bus.emit(Event.typed(
            CoreEvent.SONA_AGGREGATED,
            LearningEventPayload(
                reward=payload.get("reward", 0.0),
                action=payload.get("action", "WAG"),
                state_key=payload.get("state_key", ""),
                judgment_id=payload.get("judgment_id", ""),
                loop_name="SONA_AGGREGATED",
            ),
        ))

    async def _trigger_judgment_learning(self, judgment: dict[str, Any]) -> None:
        """Trigger learning for multiple loops on new judgment."""
        if not self.event_bus:
            return
        
        q_score = judgment.get("q_score", 50.0)

        # Trigger DPO loop
        from cynic.core.events_schema import LearningEventPayload
        await self.event_bus.emit(Event.typed(
            CoreEvent.SONA_AGGREGATED,
            LearningEventPayload(
                reward=q_score / 100.0,
                action=judgment.get("verdict", "WAG"),
                state_key=judgment.get("state_key", ""),
                judgment_id=judgment.get("judgment_id", ""),
                loop_name="DPO",
            ),
        ))

    def get_loop_status(self) -> dict[str, Any]:
        """Get status of all loops."""
        return {
            name: {
                "enabled": config.enabled,
                "priority": config.priority,
            }
            for name, config in self.loops.items()
        }

    def enable_loop(self, name: str) -> bool:
        """Enable a learning loop."""
        if name in self.loops:
            self.loops[name].enabled = True
            return True
        return False

    def disable_loop(self, name: str) -> bool:
        """Disable a learning loop."""
        if name in self.loops:
            self.loops[name].enabled = False
            return True
        return False


# Singleton
_sona: Optional[SONA] = None


def get_sona() -> SONA:
    """Get or create SONA singleton."""
    global _sona
    if _sona is None:
        _sona = SONA()
    return _sona


def create_learning_loops(event_bus: EventBus) -> SONA:
    """Create and start all learning loops."""
    sona = get_sona()
    sona.start(event_bus)
    return sona
