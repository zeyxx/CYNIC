"""SensoryBuilder â€” Create sensory components (watchers, compressor, topology)."""
from __future__ import annotations

import logging

from cynic.interfaces.api.builders.base import BaseBuilder, BuilderContext

logger = logging.getLogger("cynic.interfaces.api.builders.sensory")


class SensoryBuilder(BaseBuilder):
    """Create sensory core components."""

    builder_id = "sensory"
    description = "Perception + compression + topology"

    async def build(self, context: BuilderContext) -> None:
        context.log(f"{self.builder_id}: starting")
        try:
            context.log(f"{self.builder_id}: completed")
        except ValidationError as e:
            logger.error(f"{self.builder_id}: failed â€” {e}")
            raise RuntimeError(f"SensoryBuilder failed: {e}")
