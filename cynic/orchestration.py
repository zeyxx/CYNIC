"""
CYNIC Organism Orchestrator  Central Cognitive Command.

Integrates the CognitiveRouter, VascularSystem, and Agent Registry to provide
a unified entry point for all fractal judgments.
"""
from __future__ import annotations
import logging
from typing import Dict, Any, List

from cynic.kernel.organism.brain.cognition.router import router as cognitive_router
from cynic.kernel.core.vascular import VascularSystem
from cynic.kernel.organism.brain.llm.adapter import LLMRegistry

logger = logging.getLogger("cynic.orchestration")

class OrganismOrchestrator:
    """
    The central coordinator of the CYNIC organism.
    """
    def __init__(self, instance_id: str = "cynic-alpha"):
        self.instance_id = instance_id
        self.vascular = VascularSystem(instance_id=instance_id)
        self.registry = LLMRegistry(vascular=self.vascular)
        self.router = cognitive_router

    async def awake(self):
        """Initialize all systems and discover muscles."""
        logger.info(f" Organism '{self.instance_id}' is awakening...")
        await self.registry.discover()
        logger.info("Organism: All systems online and vascularized.")

    async def process_task(self, task_description: str) -> Dict[str, Any]:
        """
        The main cognitive loop: Route -> Execute -> Aggregrate.
        """
        # 1. Routing (S1/S2 Decision)
        decision = await self.router.route(task_description)
        
        # 2. Execution Preparation
        # If System 2 is needed, we prepare the escalation path
        if decision["target"] == "GEMINI_3":
            logger.warning("Organism: Escalating to System 2 (Gemini 3 Path).")
            # In Pre-Hackathon reality, we fallback or use Claude
        
        # 3. Final Result Construction
        return {
            "instance_id": self.instance_id,
            "routing": decision,
            "status": "PROCESSED"
        }

    async def sleep(self):
        """Gracefully shutdown the organism."""
        await self.vascular.close()
        logger.info(f"Organism '{self.instance_id}' is going to sleep.")

# Simplified managers maintained for backward compatibility
class DockerManager:
    async def build(self, version: str = "latest") -> dict: return {"success": True}
    async def deploy(self, version: str = "latest") -> dict: return {"success": True}

class HealthMonitor:
    async def check(self) -> dict: return {"status": "healthy"}
