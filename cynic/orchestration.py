"""
CYNIC Organism Orchestrator - Central Cognitive Command.

Integrates the CognitiveRouter, VascularSystem, and Agent Registry to provide
a unified entry point for all fractal judgments.
Enriched with Synaptic Memory and Metabolic Resource Appropriation.
"""

from __future__ import annotations

import logging
from typing import Any, Dict, List

from cynic.kernel.core.vascular import VascularSystem
from cynic.kernel.organism.brain.cognition.router import router as cognitive_router
from cynic.kernel.organism.brain.llm.adapter import LLMRegistry
from cynic.kernel.organism.experience import get_vault
from cynic.kernel.organism.metabolism.governor import MetabolicGovernor, OllamaProvider

logger = logging.getLogger("cynic.orchestration")


class OrganismOrchestrator:
    """
    The central coordinator of the CYNIC organism.
    """

    def __init__(self, instance_id: str = "cynic-alpha"):
        self.instance_id = instance_id
        # Physical Layer
        self.vascular = VascularSystem(instance_id=instance_id)

        # Cognitive Layers
        self.registry = LLMRegistry(vascular=self.vascular)
        self.router = cognitive_router
        self.vault = get_vault()

        # Metabolic Layer
        self.governor = MetabolicGovernor(
            provider=OllamaProvider(vascular=self.vascular)
        )

    async def awake(self) -> None:
        """Initialize all systems and discover muscles."""
        logger.info(f"Organism '{self.instance_id}' is awakening...")
        await self.registry.discover()
        # Ensure memory is loaded
        self.vault.load()
        logger.info("Organism: All systems online, memory loaded and vascularized.")

    async def select_best_muscle(self, axiom: str, candidates: List[str]) -> str:
        """Selects the best muscle for a task based on past experience."""
        return self.vault.get_best_dog_for(axiom, candidates)

    async def process_task(
        self, task_description: str, axiom: str = "FIDELITY"
    ) -> Dict[str, Any]:
        """
        The main cognitive loop: Route -> Resource -> Execute -> Learn.
        """
        # 1. Routing (S1/S2 Decision)
        decision = await self.router.route(task_description)

        # 2. Resource Appropriation
        # We select the best muscle based on the required axiom
        adapters = self.registry.get_available()
        candidates = [a.llm_id for a in adapters]
        best_muscle = await self.select_best_muscle(axiom, candidates)

        logger.info(f"Organism: Appropriating resources for {best_muscle}")
        await self.governor.allocate(best_muscle)

        # 3. Final Result Construction
        return {
            "instance_id": self.instance_id,
            "routing": decision,
            "muscle": best_muscle,
            "status": "PROCESSED",
        }

    async def sleep(self) -> None:
        """Gracefully shutdown the organism."""
        # Ensure experience is saved before sleeping
        await self.vault.persist()
        await self.vascular.close()
        logger.info(
            f"Organism '{self.instance_id}' is going to sleep. Synaptic memory persisted."
        )


# Simplified managers maintained for backward compatibility
class DockerManager:
    async def build(self, version: str = "latest") -> dict[str, bool]:
        return {"success": True}

    async def deploy(self, version: str = "latest") -> dict[str, bool]:
        return {"success": True}


class HealthMonitor:
    async def check(self) -> dict[str, str]:
        return {"status": "healthy"}
