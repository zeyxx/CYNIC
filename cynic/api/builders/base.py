"""
BaseBuilder — Interface for organism builders.

All builders share:
- BuilderContext (injected state/progress tracker)
- Logging + observability
- Async build() method
"""
from __future__ import annotations

import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Optional

logger = logging.getLogger("cynic.api.builders")


@dataclass
class BuilderContext:
    """
    Shared state passed between builders during assembly.

    Prevents god constructors — builders get exactly what they need.
    """

    # Injected dependencies
    db_pool: Optional[Any] = None
    llm_registry: Optional[Any] = None

    # Created components (filled in by builders)
    dogs: dict[str, Any] = field(default_factory=dict)
    qtable: Optional[Any] = None
    axiom_arch: Optional[Any] = None
    orchestrator: Optional[Any] = None
    scheduler: Optional[Any] = None
    learning_loop: Optional[Any] = None
    residual_detector: Optional[Any] = None
    decide_agent: Optional[Any] = None
    account_agent: Optional[Any] = None
    action_proposer: Optional[Any] = None
    runner: Optional[Any] = None
    service_registry: Optional[Any] = None
    consciousness_scheduler: Optional[Any] = None
    axiom_monitor: Optional[Any] = None
    lod_controller: Optional[Any] = None
    escore_tracker: Optional[Any] = None
    self_prober: Optional[Any] = None
    compressor: Optional[Any] = None
    world_model: Optional[Any] = None
    storage_gc: Optional[Any] = None
    surreal_storage: Optional[Any] = None

    # Services
    services: dict[str, Any] = field(default_factory=dict)
    handler_registry: Optional[Any] = None
    container: Optional[Any] = None

    # Metadata
    build_log: list[str] = field(default_factory=list)

    def log(self, message: str) -> None:
        """Log builder progress."""
        self.build_log.append(message)
        logger.info(f"[BUILD] {message}")

    def stats(self) -> dict[str, Any]:
        """Return build statistics."""
        return {
            "components_created": len([v for v in [
                self.dogs, self.qtable, self.orchestrator, self.scheduler,
                self.learning_loop, self.residual_detector, self.decide_agent,
                self.account_agent, self.action_proposer, self.runner,
                self.service_registry, self.axiom_monitor, self.lod_controller,
                self.escore_tracker, self.self_prober, self.compressor,
                self.world_model, self.storage_gc, self.surreal_storage
            ] if v is not None]),
            "build_steps": len(self.build_log),
            "services_registered": len(self.services),
            "build_log": self.build_log,
        }


class BaseBuilder(ABC):
    """
    Abstract base class for all organism builders.

    Subclasses implement build(context) to create their slice of the organism.
    """

    builder_id: str = "base"
    description: str = "Base builder (abstract)"

    @abstractmethod
    async def build(self, context: BuilderContext) -> None:
        """
        Build and populate context with this builder's components.

        Args:
            context: Shared builder state to populate

        Raises:
            RuntimeError: If build fails
        """
        raise NotImplementedError

    def _log(self, message: str) -> None:
        """Log builder action."""
        logger.info(f"[{self.builder_id}] {message}")
