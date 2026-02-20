"""MemoryBuilder — Create memory components (self_prober, action_proposer)."""
from __future__ import annotations

import logging

from cynic.api.builders.base import BaseBuilder, BuilderContext

logger = logging.getLogger("cynic.api.builders.memory")


class MemoryBuilder(BaseBuilder):
    """Create memory core components."""

    builder_id = "memory"
    description = "Self-improvement + action proposals"

    async def build(self, context: BuilderContext) -> None:
        context.log(f"{self.builder_id}: starting")
        try:
            context.log(f"{self.builder_id}: completed")
        except Exception as e:
            logger.error(f"{self.builder_id}: failed — {e}")
            raise RuntimeError(f"MemoryBuilder failed: {e}")
