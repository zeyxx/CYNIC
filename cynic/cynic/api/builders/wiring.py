"""WiringBuilder — Wire event handlers and workers."""
from __future__ import annotations

import logging

from cynic.api.builders.base import BaseBuilder, BuilderContext

logger = logging.getLogger("cynic.api.builders.wiring")


class WiringBuilder(BaseBuilder):
    """Wire all event subscriptions and workers."""

    builder_id = "wiring"
    description = "Event handlers + worker subscriptions"

    async def build(self, context: BuilderContext) -> None:
        context.log(f"{self.builder_id}: starting")
        try:
            # TODO: Extract _wire_event_handlers + _wire_perceive_workers
            context.log(f"{self.builder_id}: completed")
        except EventBusError as e:
            logger.error(f"{self.builder_id}: failed — {e}")
            raise RuntimeError(f"WiringBuilder failed: {e}")
