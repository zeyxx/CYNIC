"""LNSP Manager: Unified coordinator for all 4 protocol layers.

LNSPManager provides a unified interface to the complete Layered Nervous System
Protocol (LNSP) pipeline, coordinating all 4 layers and their subscriptions.

This enables easy setup and execution of the complete observation â’ aggregation â’
judgment â’ action â’ feedback cycle.
"""

from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
from typing import Any

from .layer1 import Layer1
from .layer2 import Layer2
from .layer3 import Layer3
from .layer4 import Layer4
from .types import LNSPMessage


@dataclass
class LNSPManager:
    """Unified manager for all LNSP layers.

    Coordinates the complete observation â’ aggregation â’ judgment â’ action
    â’ feedback cycle. Manages initialization, subscription wiring, and
    execution of the full pipeline.

    Attributes:
        instance_id: Unique organism instance ID (default: "instance:local")
        region: Optional regional identifier for distributed systems
        layer1: Raw observation collection layer
        layer2: Aggregated state synthesis layer
        layer3: Axiom-based judgment layer
        layer4: Action execution and feedback layer
    """

    instance_id: str = "instance:local"
    region: str | None = None
    layer1: Layer1 = field(default_factory=Layer1)
    layer2: Layer2 = field(default_factory=Layer2)
    layer3: Layer3 = field(default_factory=lambda: Layer3(judge_id="judge:primary"))
    layer4: Layer4 = field(default_factory=Layer4)

    def __init__(
        self,
        instance_id: str = "instance:local",
        region: str | None = None,
        layer1: Layer1 | None = None,
        layer2: Layer2 | None = None,
        layer3: Layer3 | None = None,
        layer4: Layer4 | None = None,
    ) -> None:
        """Initialize LNSPManager with all layers.

        Creates or uses provided layer instances. If not provided, creates
        new instances with default configuration.

        Args:
            instance_id: Unique organism instance ID (default: "instance:local")
            region: Optional region identifier
            layer1: Layer1 instance (created if None)
            layer2: Layer2 instance (created if None)
            layer3: Layer3 instance (created if None)
            layer4: Layer4 instance (created if None)
        """
        self.instance_id = instance_id
        self.region = region
        self.layer1 = layer1 if layer1 is not None else Layer1()
        self.layer2 = layer2 if layer2 is not None else Layer2()
        self.layer3 = layer3 if layer3 is not None else Layer3(judge_id="judge:primary")
        self.layer4 = layer4 if layer4 is not None else Layer4()

    def wire_layers(self) -> None:
        """Wire layers together with subscriptions.

        Connects all layers in the LNSP pipeline:
        - Layer 1 â’ Layer 2: Observations trigger aggregation
        - Layer 2 â’ Layer 3: Aggregated state triggers judgment
        - Layer 3 â’ Layer 4: Judgments trigger action execution
        - Layer 4 â’ Layer 1: Action results feed back as observations

        After calling wire_layers(), run_cycle() will execute the complete
        pipeline in order.
        """

        # Layer 1 â’ Layer 2: Observations trigger aggregation
        def on_observation(msg: LNSPMessage) -> None:
            """Forward Layer 1 observations to Layer 2."""
            # Fire and forget - schedule as background task
            asyncio.create_task(self.layer2.process_observation(msg))

        self.layer1.subscribe(on_observation)

        # Layer 2 â’ Layer 3: Aggregated state triggers judgment
        async def on_aggregation(msg: LNSPMessage) -> None:
            """Forward Layer 2 aggregated state to Layer 3 for judgment."""
            await self.layer3.judge(msg)

        self.layer2.subscribe(lambda msg: asyncio.create_task(on_aggregation(msg)))

        # Layer 3 â’ Layer 4: Judgments trigger action execution
        async def on_judgment(msg: LNSPMessage) -> None:
            """Forward Layer 3 judgments to Layer 4 for execution."""
            await self.layer4.execute(msg)

        self.layer3.subscribe(lambda msg: asyncio.create_task(on_judgment(msg)))

        # Layer 4 â’ Layer 1: Feedback observations close the loop
        def on_feedback(msg: LNSPMessage) -> None:
            """Route Layer 4 feedback back to Layer 1."""
            self.layer1.ringbuffer.put(msg)

        self.layer4.on_feedback(on_feedback)

    async def run_cycle(self) -> None:
        """Run one complete observation â’ aggregation â’ judgment â’ action cycle.

        Executes the full LNSP pipeline:
        1. Layer 1: Collect observations from all sensors
        2. Layer 2: Aggregate observations into state
        3. Layer 3: Judge aggregated state (happens via subscription)
        4. Layer 4: Execute verdicts (happens via subscription)

        Note: Layers 3 and 4 are invoked via subscription callbacks from
        layers 2 and 3 respectively. This method returns after all
        subscriptions are triggered.
        """
        # Layer 1: Collect observations
        await self.layer1.observe()

        # Layer 2: Aggregate observations
        await self.layer2.aggregate()

        # Layers 3 and 4 are invoked automatically via subscriptions
        # above, no need to call them directly here

    def stats(self) -> dict[str, Any]:
        """Get statistics from all layers.

        Aggregates stats from all 4 layers into a single dictionary for
        monitoring and debugging.

        Returns:
            Dict with keys:
                - instance_id: Instance identifier
                - region: Regional identifier (if set)
                - layer1: Layer 1 statistics (sensors, buffer, etc.)
                - layer2: Layer 2 statistics (aggregators, windows, etc.)
                - layer3: Layer 3 statistics (axioms, routing rules, etc.)
                - layer4: Layer 4 statistics (handlers, feedback, etc.)
        """
        return {
            "instance_id": self.instance_id,
            "region": self.region,
            "layer1": self.layer1.stats(),
            "layer2": self.layer2.stats(),
            "layer3": self.layer3.stats(),
            "layer4": self.layer4.stats(),
        }
