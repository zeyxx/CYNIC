"""
OrganismAssembler — Orchestrate builder sequence to create CynicOrganism.

This replaces _OrganismAwakener.build() with a composable, observable sequence.
"""
from __future__ import annotations

import logging
from typing import Optional, TYPE_CHECKING, Any

if TYPE_CHECKING:
    from cynic.api.state import CynicOrganism

from cynic.api.builders.base import BuilderContext
from cynic.api.builders.component import ComponentBuilder
from cynic.api.builders.cognition import CognitionBuilder
from cynic.api.builders.metabolic import MetabolicBuilder
from cynic.api.builders.sensory import SensoryBuilder
from cynic.api.builders.memory import MemoryBuilder
from cynic.api.builders.storage import StorageBuilder
from cynic.api.builders.wiring import WiringBuilder
from cynic.api.state import (
    CognitionCore, MetabolicCore, SensoryCore, MemoryCore, CynicOrganism, KernelMirror
)
from cynic.metabolism.telemetry import TelemetryStore

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
        self.context: Optional[BuilderContext] = None
        self.builders = [
            ComponentBuilder(),
            StorageBuilder(),
            CognitionBuilder(),
            MetabolicBuilder(),
            SensoryBuilder(),
            MemoryBuilder(),
            WiringBuilder(),
        ]

    def _create_organism_from_context(self, context: BuilderContext) -> CynicOrganism:
        """Assemble the final CynicOrganism from builder context."""
        # Extract all components from context
        logger.info(
            "Kernel ready: %d dogs, learning + residual detector active",
            len(context.dogs),
        )

        # Build 4 Cores from context components
        cognition = CognitionCore(
            orchestrator=context.orchestrator,
            qtable=context.qtable,
            learning_loop=context.learning_loop,
            residual_detector=context.residual_detector,
            decide_agent=context.decide_agent,
            account_agent=context.account_agent,
            axiom_monitor=context.axiom_monitor,
            lod_controller=context.lod_controller,
            escore_tracker=context.escore_tracker,
            power_limiter=context.power_limiter,
            alignment_checker=context.alignment_checker,
            human_gate=context.human_gate,
            audit_trail=context.audit_trail,
            decision_validator=context.decision_validator,
        )

        metabolism = MetabolicCore(
            scheduler=context.scheduler,
            runner=context.runner,
            llm_router=context.llm_router,
            telemetry_store=TelemetryStore(),
            universal_actuator=context.universal_actuator,
            auto_benchmark=None,
        )

        senses = SensoryCore(
            context_compressor=context.compressor,
            service_registry=context.service_registry,
            event_journal=context.event_journal,
            decision_tracer=context.decision_tracer,
            loop_closure_validator=context.loop_closure_validator,
            world_model=context.world_model,
            source_watcher=context.source_watcher,
            topology_builder=context.topology_builder,
            hot_reload_coordinator=context.hot_reload_coordinator,
            topology_mirror=context.topology_mirror,
            change_tracker=context.change_tracker,
            change_analyzer=context.change_analyzer,
            convergence_validator=context.convergence_validator,
        )

        memory = MemoryCore(
            kernel_mirror=KernelMirror(),
            action_proposer=context.action_proposer,
            self_prober=context.self_prober,
        )

        # Create final organism
        return CynicOrganism(
            cognition=cognition,
            metabolism=metabolism,
            senses=senses,
            memory=memory,
            _pool=self.db_pool,
            container=context.dependency_container,
            _handler_registry=context.handler_registry,
        )

    async def assemble(self) -> "CynicOrganism":
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
            # Run all builders in sequence
            for builder in self.builders:
                logger.info(f"[assembler] running {builder.builder_id}")
                await builder.build(context)

            # Create final organism from context
            organism = self._create_organism_from_context(context)
            self.context = context

            logger.info("*tail wag* Organism assembly complete")
            context.log("assembly: complete")

            # Print stats
            stats = context.stats()
            logger.info(f"Build stats: {stats}")

            return organism

        except Exception as e:
            logger.error(f"Organism assembly failed: {e}")
            context.log(f"assembly: failed — {e}")
            raise RuntimeError(f"Failed to assemble organism: {e}")

    def get_context(self) -> Optional[BuilderContext]:
        """Return the current build context (for introspection/testing)."""
        return self.context
