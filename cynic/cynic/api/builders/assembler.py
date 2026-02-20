"""
OrganismAssembler — Orchestrate builder sequence to create CynicOrganism.

This replaces _OrganismAwakener.build() with a composable, observable sequence.
"""
from __future__ import annotations

import logging
from typing import Any, Optional

from cynic.api.builders.base import BuilderContext
from cynic.api.builders.component import ComponentBuilder
from cynic.api.builders.cognition import CognitionBuilder
from cynic.api.builders.metabolic import MetabolicBuilder
from cynic.api.builders.sensory import SensoryBuilder
from cynic.api.builders.memory import MemoryBuilder
from cynic.api.builders.storage import StorageBuilder
from cynic.api.builders.wiring import WiringBuilder

logger = logging.getLogger("cynic.api.builders.assembler")


class OrganismAssembler:
    """
    Orchestrate the builder sequence to create a fully-wired CynicOrganism.

    Builder order is CRITICAL (dependencies between builders):
    1. ComponentBuilder (creates dogs + axiom_arch)
    2. StorageBuilder (creates storage, needed by learning)
    3. CognitionBuilder (creates orchestrator, depends on dogs + storage)
    4. MetabolicBuilder (creates scheduler + runners)
    5. SensoryBuilder (creates compressor + watchers)
    6. MemoryBuilder (creates self_prober + action_proposer)
    7. WiringBuilder (wires everything together)
    """

    def __init__(
        self,
        db_pool: Optional[Any] = None,
        llm_registry: Optional[Any] = None,
    ) -> None:
        self.db_pool = db_pool
        self.llm_registry = llm_registry
        self.builders = [
            ComponentBuilder(),
            StorageBuilder(),
            CognitionBuilder(),
            MetabolicBuilder(),
            SensoryBuilder(),
            MemoryBuilder(),
            WiringBuilder(),
        ]

    async def assemble(self) -> Any:
        """
        Run all builders in sequence to create CynicOrganism.

        Returns:
            CynicOrganism (fully wired, ready to use)

        Raises:
            RuntimeError: If any builder fails
        """
        context = BuilderContext(db_pool=self.db_pool, llm_registry=self.llm_registry)

        logger.info("*sniff* Organism assembly starting")
        context.log("assembly: starting")

        try:
            for builder in self.builders:
                logger.info(f"[assembler] running {builder.builder_id}")
                await builder.build(context)

            # TODO: Create CynicOrganism from context
            logger.info("*tail wag* Organism assembly complete")
            context.log("assembly: complete")

            # Print stats
            stats = context.stats()
            logger.info(f"Build stats: {stats}")

            return None  # Would return CynicOrganism

        except Exception as e:
            logger.error(f"Organism assembly failed: {e}")
            context.log(f"assembly: failed — {e}")
            raise RuntimeError(f"Failed to assemble organism: {e}")

    def get_context(self) -> Optional[BuilderContext]:
        """Return the current build context (for introspection/testing)."""
        # TODO: Store context as instance variable during assemble()
        return None
