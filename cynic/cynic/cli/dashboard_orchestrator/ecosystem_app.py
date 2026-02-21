"""Dashboard orchestrator — wires 7 layers + WebSocket client."""
from __future__ import annotations

from typing import Optional, Any
import logging

from cynic.cli.tui_layers import (
    EcosystemLayer,
    PerceptionLayer,
    TopologyLayer,
    NervousLayer,
    GuardrailsLayer,
    SelfAwareLayer,
)

logger = logging.getLogger(__name__)


class EcosystemDashboardApp:
    """Orchestrator for 7-layer consciousness dashboard.

    Manages:
    - All 7 layer panels (eco, perception, topology, nervous, guardrails, self_aware)
    - WebSocket connection to /ws/consciousness/ecosystem
    - Message dispatching to layers
    """

    def __init__(self, ws_url: str = "ws://localhost:8000/ws/consciousness/ecosystem"):
        """Initialize orchestrator with all 7 layers.

        Args:
            ws_url: WebSocket URL for consciousness ecosystem stream
        """
        self.ws_url = ws_url
        self.ecosystem = EcosystemLayer()
        self.perception = PerceptionLayer()
        self.topology = TopologyLayer()
        self.nervous = NervousLayer()
        self.guardrails = GuardrailsLayer()
        self.self_aware = SelfAwareLayer()
        self._running = False

    def _dispatch_snapshot(self, snapshot: dict[str, Any]) -> None:
        """Dispatch ecosystem snapshot to all 7 layers.

        Args:
            snapshot: Full ecosystem snapshot from WebSocket or HTTP API

        Raises:
            No exceptions — logs errors and continues
        """
        try:
            if self.ecosystem:
                self.ecosystem.update_from_snapshot(snapshot)
            if self.perception:
                self.perception.update_from_snapshot(snapshot)
            if self.topology:
                self.topology.update_from_snapshot(snapshot)
            if self.nervous:
                self.nervous.update_from_snapshot(snapshot)
            if self.guardrails:
                self.guardrails.update_from_snapshot(snapshot)
            if self.self_aware:
                self.self_aware.update_from_snapshot(snapshot)
        except Exception as e:
            logger.error(f"Error dispatching snapshot: {e}", exc_info=True)

    def get_all_layers(self) -> list:
        """Get all 7 layers as list.

        Returns:
            List of all 7 layer panels in render order
        """
        return [
            self.ecosystem,
            self.perception,
            self.topology,
            self.nervous,
            self.guardrails,
            self.self_aware,
        ]

    def render_all(self) -> list:
        """Render all layers for display.

        Returns:
            List of Rich renderables for each layer
        """
        return [layer.render() for layer in self.get_all_layers()]
