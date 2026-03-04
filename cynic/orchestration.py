"""
CYNIC Sovereign Orchestrator - Industrial Production Version.
Managed by the 9 Engineering Lenses.
"""
from __future__ import annotations
import logging
import asyncio
import sys
from typing import Any, Dict, List

from cynic.kernel.core.vascular import VascularSystem
from cynic.kernel.organism.brain.llm.adapter import LLMRegistry, LLMRequest
from cynic.kernel.organism.experience import get_vault
from cynic.kernel.organism.metabolism.watchdog import MetabolicWatchdog
from cynic.kernel.organism.cognition.council import AxiomaticCouncil
from cynic.kernel.organism.metabolism.actuator import SurgicalActuator
from cynic.kernel.organism.metabolism.kpi import KPIManager
from cynic.kernel.core.ledger import EventLedger

logger = logging.getLogger("cynic.orchestration")

class OrganismOrchestrator:
    def __init__(self, instance_id: str = "cynic-industrial"):
        self.instance_id = instance_id
        self.vascular = VascularSystem(instance_id=instance_id)
        self.watchdog = MetabolicWatchdog({
            "Ollama": "http://localhost:11434",
            "Sovereign": "http://localhost:8080"
        })
        
        self.registry = LLMRegistry(vascular=self.vascular)
        self.vault = get_vault()
        self.council = AxiomaticCouncil()
        self.actuator = SurgicalActuator()
        self.kpi = KPIManager()
        self.ledger = EventLedger()

    async def awake(self) -> None:
        logger.info(f"Organism '{self.instance_id}' AWAKENING through 9 Lenses.")
        await self.watchdog.start()
        await self.registry.discover()
        self.vault.load()
        self.ledger.record("SYSTEM_AWAKE", {"instance": self.instance_id})

    async def execute_mission(self, target: str, description: str, axiom: str):
        """Full Industrial Loop: Research -> Act -> Learn -> Predict -> Heal."""
        logger.info(f"--- MISSION START: {target} ---")
        
        # 1. Select Muscle (UCB1)
        adapters = self.registry.get_available()
        candidates = [a.llm_id for a in adapters]
        best_muscle = self.vault.get_best_dog_for(axiom, candidates)
        adapter = next((a for a in adapters if a.llm_id == best_muscle), None)
        
        if not adapter: return
        
        # 2. Inference
        logger.info(f"Inference: Igniting {best_muscle}...")
        response = await adapter.complete_safe(LLMRequest(
            prompt=f"Axiom: {axiom}\nTarget: {target}\nRequest: {description}\nPropose a production-grade fix."
        ))
        
        # 3. Council Review
        reviews = await self.council.review_proposal(response.content, target)
        phi_score = sum(r.score for r in reviews) / len(reviews)
        is_safe = all(r.verdict != "HOWL" for r in reviews)
        
        # 4. Action (if safe)
        success = False
        if is_safe:
            success = await self.actuator.apply_fix(target, response.content)
        else:
            logger.warning(f"Mission ABORTED: Council rejected proposal (Phi: {phi_score:.2f})")

        # 5. Ledger & Learning
        self.ledger.record("MISSION_COMPLETED", {
            "target": target, "muscle": best_muscle, 
            "success": success, "phi": phi_score
        })
        
        await self.vault.record_experience(best_muscle, axiom, success, response.latency_ms)
        self.kpi.record_mission(success, phi_score, response.latency_ms)
        
        logger.info(f"--- MISSION END: Success={success} | Phi={phi_score:.2f} ---")

    async def sleep(self) -> None:
        await self.watchdog.stop()
        await self.vault.persist()
        self.ledger.record("SYSTEM_SLEEP", {"instance": self.instance_id})
        await self.vascular.close()
        logger.info("Organism: Sleeping.")

if __name__ == "__main__":
    async def main():
        orch = OrganismOrchestrator()
        await orch.awake()
        if "--night-cycle" in sys.argv:
            from cynic.kernel.organism.discovery import DiscoveryDaemon
            discovery = DiscoveryDaemon()
            missions = discovery.find_potential_missions()
            for m in missions[:5]:
                await orch.execute_mission(m['target'], m['description'], m['axiom'])
        await orch.sleep()
    asyncio.run(main())
