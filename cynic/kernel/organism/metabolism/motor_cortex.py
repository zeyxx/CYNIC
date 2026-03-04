"""
CYNIC Motor Cortex - The Effector Hub.
Respects Robotics & Solutions Architect Lenses.

Bridges the abstract intent of the organism (Event Bus) with the physical reality
of the host system (Actuators). It listens for ACTION_REQUESTED events, applies
them, and broadcasts the result to close the sensory-motor loop.
"""

from __future__ import annotations

import logging

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.organism.metabolism.actuators import UniversalActuator, FileActuator
from cynic.kernel.organism.metabolism.actuators_sovereign import (
    LocalCodeActuator,
    ShellActuator,
)

logger = logging.getLogger("cynic.motor_cortex")


class MotorCortex:
    """
    The central coordinator for physical actions in the host reality.
    """

    def __init__(self, bus: EventBus):
        self.bus = bus
        self.actuator = UniversalActuator()

        # Wire the physical muscles
        self.actuator.register("file", FileActuator())
        self.actuator.register("code_edit", LocalCodeActuator())
        self.actuator.register("shell", ShellActuator())

        # Connect to the nervous system
        self.bus.on(CoreEvent.ACTION_REQUESTED, self._on_action_requested)
        logger.info("Motor Cortex initialized and wired to the Event Bus.")

    async def _on_action_requested(self, event: Event) -> None:
        """Handle abstract intents from the brain and translate them into physical actions."""
        payload = event.payload
        action_id = payload.get("action_id", "unknown")
        action_type = payload.get("type")
        action_args = payload.get("args", {})

        if not action_type:
            logger.error(
                f"MotorCortex received an action request without a type: {event}"
            )
            return

        logger.info(
            f"MotorCortex engaging muscle '{action_type}' for action '{action_id}'"
        )

        # Execute the physical action
        result = await self.actuator.dispatch(action_type, action_args)

        # Close the loop: The organism must 'feel' the outcome of its action
        response_payload = {
            "action_id": action_id,
            "type": action_type,
            "success": result.success,
            "output": result.output,
            "error": result.error,
            "duration_ms": result.duration_ms,
        }

        await self.bus.emit(
            Event.typed(
                CoreEvent.ACTION_COMPLETED, response_payload, source="motor_cortex"
            )
        )

        if result.success:
            logger.debug(
                f"Action '{action_id}' executed successfully in {result.duration_ms:.1f}ms"
            )
        else:
            logger.warning(f"Action '{action_id}' failed: {result.error}")
