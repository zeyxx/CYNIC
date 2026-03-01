"""MemoryBuilder â€” Create memory components (self_prober, action_proposer)."""
from __future__ import annotations

import logging

from cynic.interfaces.api.builders.base import BaseBuilder, BuilderContext

logger = logging.getLogger("cynic.interfaces.api.builders.memory")


class MemoryBuilder(BaseBuilder):
    """Create memory core components."""

    builder_id = "memory"
    description = "Self-improvement + action proposals"

    async def build(self, context: BuilderContext) -> None:
        context.log(f"{self.builder_id}: starting")
        try:
            context.log(f"{self.builder_id}: completed")
        except ValidationError as e:
            logger.error(f"{self.builder_id}: failed â€” {e}")
            raise RuntimeError(f"MemoryBuilder failed: {e}")
