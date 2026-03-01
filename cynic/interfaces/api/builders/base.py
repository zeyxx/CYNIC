"""
BaseBuilder â€” Interface for organism builders.

All builders share:
- BuilderContext (injected state/progress tracker)
- Logging + observability
- Async build() method
"""
from __future__ import annotations

import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any

logger = logging.getLogger("cynic.interfaces.api.builders")


@dataclass
class BuilderContext:
    """
    Shared state passed between builders during assembly.

    Prevents god constructors â€” builders get exactly what they need.
    """

    # Injected dependencies
    db_pool: Any | None = None
    llm_registry: Any | None = None

    # Created components (filled in by builders)
    dogs: dict[str, Any] = field(default_factory=dict)
    qtable: Any | None = None
    axiom_arch: Any | None = None
    orchestrator: Any | None = None
    scheduler: Any | None = None
    learning_loop: Any | None = None
    residual_detector: Any | None = None
    decide_agent: Any | None = None
    account_agent: Any | None = None
    action_proposer: Any | None = None
    runner: Any | None = None
    service_registry: Any | None = None
    consciousness_scheduler: Any | None = None
    axiom_monitor: Any | None = None
    lod_controller: Any | None = None
    escore_tracker: Any | None = None
    self_prober: Any | None = None
    compressor: Any | None = None
    world_model: Any | None = None
    storage_gc: Any | None = None
    surreal_storage: Any | None = None

    # Services
    services: dict[str, Any] = field(default_factory=dict)
    handler_registry: Any | None = None
    container: Any | None = None

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
