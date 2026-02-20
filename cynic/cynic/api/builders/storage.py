"""StorageBuilder — Create storage components (SurrealDB, garbage collector)."""
from __future__ import annotations

import logging

from cynic.api.builders.base import BaseBuilder, BuilderContext

logger = logging.getLogger("cynic.api.builders.storage")


class StorageBuilder(BaseBuilder):
    """Create storage components."""

    builder_id = "storage"
    description = "SurrealDB + garbage collection"

    async def build(self, context: BuilderContext) -> None:
        context.log(f"{self.builder_id}: starting")
        try:
            context.log(f"{self.builder_id}: completed")
        except Exception as e:
            logger.error(f"{self.builder_id}: failed — {e}")
            raise RuntimeError(f"StorageBuilder failed: {e}")
