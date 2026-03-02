"""Layer 4: Action Execution and Feedback Loop

Layer 4 executes verdicts from Layer 3 as actions on the system, and creates a
feedback loop back to Layer 1. This closes the observation â’ judgment â’ action
â’ observation cycle.

Components:
- Handler: Abstract base class for verdict execution
- Layer4: Manager that routes verdicts to handlers and emits feedback
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

from .messages import create_raw_observation
from .types import Layer, LNSPMessage, ObservationType


class Handler(ABC):
    """Abstract base class for verdict handlers.

    Handlers execute Layer 3 verdicts as actions on the system. Each handler
    is responsible for a specific action domain (e.g., system config, external
    API calls, human signaling).

    Attributes:
        handler_id: Unique identifier for this handler
    """

    def __init__(self, handler_id: str) -> None:
        """Initialize handler with unique ID.

        Args:
            handler_id: Unique identifier for this handler
        """
        self.handler_id = handler_id

    @abstractmethod
    async def handle(self, verdict: LNSPMessage) -> tuple[bool, Any]:
        """Execute a verdict as an action.

        Args:
            verdict: Layer 3 verdict message to execute

        Returns:
            Tuple of (success: bool, result_data: Any)
            - success: True if action executed successfully
            - result_data: Handler-specific result data (dict, str, etc.)
        """
        pass


@dataclass
class Layer4:
    """Layer 4 manager for action execution and feedback loop.

    Layer 4 routes verdicts to appropriate handlers, executes them, and emits
    feedback messages back to Layer 1, closing the LNSP loop.

    Attributes:
        handlers: Dict mapping handler IDs to Handler instances
        feedback_callbacks: List of callbacks to emit feedback to
    """

    handlers: dict[str, Handler] = field(default_factory=dict)
    feedback_callbacks: list[Callable[[LNSPMessage], None]] = field(default_factory=list)

    def register_handler(self, handler: Handler) -> None:
        """Register an action handler.

        Args:
            handler: Handler instance to register
        """
        self.handlers[handler.handler_id] = handler

    def on_feedback(self, callback: Callable[[LNSPMessage], None]) -> None:
        """Register a feedback callback.

        Callbacks are called with feedback messages emitted by Layer 4. These
        typically route back to Layer 1 to close the observation loop.

        Args:
            callback: Callable that takes an LNSPMessage
        """
        self.feedback_callbacks.append(callback)

    async def execute(self, verdict: LNSPMessage) -> tuple[bool, LNSPMessage | None]:
        """Execute a verdict through appropriate handler.

        Process:
        1. Check message is Layer 3 (JUDGMENT)
        2. Find handler matching verdict target
        3. Call handler.handle(verdict)
        4. Create feedback message of type ACTION_RESULT
        5. Tag feedback with closes_action_id
        6. Call feedback callbacks
        7. Return (success, feedback_message)

        Args:
            verdict: Layer 3 verdict message to execute

        Returns:
            Tuple of (success: bool, feedback_message: LNSPMessage | None)
            - success: True if verdict was executed
            - feedback_message: Layer 1 ACTION_RESULT observation, or None if
              not executed

        Routing:
            Verdicts are routed to handlers by matching target prefix with
            handler ID prefix. Example: verdict target "system:config" matches
            handler "system:*".
        """
        # Check: only handle Layer 3 verdicts
        if verdict.header.layer != Layer.JUDGMENT:
            return (False, None)

        # Find handler matching target
        handler = self._find_handler_for_target(verdict.header.target)
        if handler is None:
            return (False, None)

        # Execute handler
        try:
            success, result = await handler.handle(verdict)
        except Exception as e:
            # Handler exception converted to failure
            success = False
            result = {"error": str(e)}

        # Create feedback message (Layer 1 observation of action result)
        feedback = create_raw_observation(
            observation_type=ObservationType.ACTION_RESULT,
            data={
                "action_id": verdict.header.message_id,
                "handler_id": handler.handler_id,
                "success": success,
                "result": result,
            },
            source=handler.handler_id,
            instance_id=verdict.metadata.instance_id,
            region=verdict.metadata.region,
        )

        # Tag feedback with closes_action_id
        feedback.metadata.feedback = True
        feedback.metadata.closes_action_id = verdict.header.message_id

        # Call feedback callbacks â€” ensure one failure doesn't block others
        for callback in self.feedback_callbacks:
            try:
                callback(feedback)
            except Exception as e:
                # Log callback failures but continue executing other callbacks
                import logging

                logger = logging.getLogger(__name__)
                logger.warning(
                    f"Feedback callback failed: {callback.__name__}: {type(e).__name__}: {e}"
                )

        return (success, feedback)

    def _find_handler_for_target(self, target: str | None) -> Handler | None:
        """Find handler matching target by prefix matching.

        Simplified routing: match handler ID prefix with target prefix.
        Example: target "system:config" matches handler "system:*"

        Args:
            target: Target component from verdict header

        Returns:
            Handler instance if found, None otherwise
        """
        if target is None:
            return None

        # First try exact match
        if target in self.handlers:
            return self.handlers[target]

        # Then try prefix matching
        target_prefix = target.split(":")[0] if ":" in target else target

        for handler_id, handler in self.handlers.items():
            handler_prefix = handler_id.split(":")[0] if ":" in handler_id else handler_id

            if handler_id == f"{handler_prefix}:*" and target_prefix == handler_prefix:
                return handler

        return None

    def stats(self) -> dict[str, Any]:
        """Return current statistics about Layer 4 state.

        Returns:
            Dict with keys:
                - handler_count: Number of registered handlers
                - feedback_callback_count: Number of registered callbacks
        """
        return {
            "handler_count": len(self.handlers),
            "feedback_callback_count": len(self.feedback_callbacks),
        }
