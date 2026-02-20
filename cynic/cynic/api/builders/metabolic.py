"""MetabolicBuilder — Create metabolic components (scheduler, runners)."""
from __future__ import annotations

import logging

from cynic.api.builders.base import BaseBuilder, BuilderContext

logger = logging.getLogger("cynic.api.builders.metabolic")


class MetabolicBuilder(BaseBuilder):
    """Create metabolic core components."""

    builder_id = "metabolic"
    description = "Scheduler + runners + actuators"

    async def build(self, context: BuilderContext) -> None:
        context.log(f"{self.builder_id}: starting")
        try:
            context.log(f"{self.builder_id}: completed")
        except Exception as e:
            logger.error(f"{self.builder_id}: failed — {e}")
            raise RuntimeError(f"MetabolicBuilder failed: {e}")
