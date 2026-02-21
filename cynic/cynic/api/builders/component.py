"""ComponentBuilder — Create base components (dogs, registries)."""
from __future__ import annotations

import logging

from cynic.api.builders.base import BaseBuilder, BuilderContext

logger = logging.getLogger("cynic.api.builders.component")


class ComponentBuilder(BaseBuilder):
    """
    Create base components:
    - Dogs (all 11 neural agents)
    - LLM registry
    - Axiom architecture
    """

    builder_id = "component"
    description = "Dogs + registries + axiom architecture"

    async def build(self, context: BuilderContext) -> None:
        """Build base components."""
        context.log(f"{self.builder_id}: starting")

        try:
            # TODO: Extract _create_components logic
            # For now, placeholder
            context.dogs = {}
            context.axiom_arch = None
            context.log(f"{self.builder_id}: completed")
        except ValidationError as e:
            logger.error(f"{self.builder_id}: failed — {e}")
            raise RuntimeError(f"ComponentBuilder failed: {e}")
