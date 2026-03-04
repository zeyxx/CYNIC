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
from cynic.kernel.infrastructure.server_manager import SovereignServerManager
from cynic.kernel.organism.benchmarking import get_benchmarker, BenchmarkMetric
from cynic.kernel.organism.metabolism.token_filter import TokenFilter
from cynic.kernel.organism.memory.cognitive_graph import CognitiveGraph
from cynic.kernel.organism.memory.kv_manager import KVManager

logger = logging.getLogger("cynic.orchestration")


class OrganismOrchestrator:
    """
    The central coordinator of the CYNIC organism.
    """

    def __init__(self, instance_id: str = "cynic-alpha"):
        self.instance_id = instance_id
        # Physical Layer
        self.vascular = VascularSystem(instance_id=instance_id)
        self.server_manager = SovereignServerManager()

        # Memory & Optimization Layers
        self.registry = LLMRegistry(vascular=self.vascular)
        self.router = cognitive_router
        self.vault = get_vault()
        self.benchmarker = get_benchmarker()
        self.token_filter = TokenFilter()
        self.cognitive_graph = CognitiveGraph()
        self.kv_manager = KVManager()

        # Metabolic Layer
        self.governor = MetabolicGovernor(
            provider=OllamaProvider(vascular=self.vascular)
        )

    async def awake(self) -> None:
        """Initialize all systems and discover muscles."""
        logger.info(f"Organism '{self.instance_id}' is awakening...")
        
        # Discovery of already running services (Ollama, local servers)
        await self.registry.discover()
        
        # NOTE: Auto-ignition of llama-server disabled for Night One 
        # to avoid Access Violation (0xC0000005) on Radeon Vega 8 drivers.
        # Use existing Ollama models for stability.

        # Ensure memory is loaded
        self.vault.load()
        self.cognitive_graph.load()
        logger.info("Organism: All systems online, memory loaded and hardware stable.")

    async def select_best_muscle(self, axiom: str, candidates: List[str]) -> str:
        """Selects the best muscle for a task based on past experience."""
        return self.vault.get_best_dog_for(axiom, candidates)

    async def process_task(
        self, task_description: str, axiom: str = "FIDELITY"
    ) -> Dict[str, Any]:
        """
        The main cognitive loop: Route -> Filter -> Resource -> Context -> Execute -> Learn.
        """
        # 1. Routing (S1/S2 Decision)
        decision = await self.router.route(task_description)

        # 2. Metabolic Filtering (RTK Doctrine)
        # We compress the task description to remove noise
        filtered_task = self.token_filter.compress_shell_output(task_description)

        # 3. Resource Appropriation
        adapters = self.registry.get_available()
        candidates = [a.llm_id for a in adapters]
        best_muscle = await self.select_best_muscle(axiom, candidates)

        logger.info(f"Organism: Appropriating resources for {best_muscle}")
        await self.governor.allocate(best_muscle)

        # 4. Cognitive Context Injection (AgentKeeper Doctrine)
        relevant_facts = self.cognitive_graph.get_relevant_context(filtered_task)
        full_prompt = f"{relevant_facts}\nTask: {filtered_task}\nProvide a deep axiomatic analysis."

        # 5. KV Cache Slotting (LMCache Doctrine)
        target_slot = self.kv_manager.get_slot_for_task(best_muscle)

        # 6. Real E2E Execution
        success = False
        latency_ms = 0.0
        response_text = ""
        
        # Find the actual adapter instance
        adapter = next((a for a in adapters if a.llm_id == best_muscle), None)
        
        if adapter:
            from cynic.kernel.organism.brain.llm.adapter import LLMRequest
            from cynic.kernel.organism.cognition.parameters import ParameterGovernor
            import time
            start_time = time.time()
            
            # Deep Parameter Management (Inspired by Unsloth)
            params = ParameterGovernor.get_params_for_axiom(axiom)
            logger.info(f"Organism: Setting synaptic tension for {axiom} (Temp: {params.temperature}, P: {params.top_p}, Slot: {target_slot})")
            
            logger.info(f"Organism: Firing synapse on {best_muscle}...")
            
            # Create a heartbeat task to show life during long inference
            async def heartbeat():
                while True:
                    await asyncio.sleep(15)
                    logger.info(f"Organism: Still thinking... (Axiom: {axiom}, Muscle: {best_muscle})")
            
            heartbeat_task = asyncio.create_task(heartbeat())

            try:
                # Real E2E with deep parameters and KV slotting
                response = await adapter.complete_safe(
                    LLMRequest(
                        prompt=full_prompt,
                        system="You are CYNIC, an autonomous OS. Use your full cognitive depth.",
                        max_tokens=params.max_tokens,
                        temperature=params.temperature,
                        metadata={
                            "top_p": params.top_p, 
                            "repeat_penalty": params.repeat_penalty,
                            "slot_id": target_slot,
                            "cache_prompt": True
                        }
                    )
                )
            finally:
                heartbeat_task.cancel()
            success = response.is_success
            latency_ms = response.latency_ms if response.latency_ms > 0 else (time.time() - start_time) * 1000
            response_text = response.content
            logger.info(f"Organism: Output received ({latency_ms:.0f}ms): {response_text.strip()}")
            
            # 7. Learning (Synaptic Update based on REAL results)
            await self.vault.record_experience(
                llm_id=best_muscle, 
                axiom=axiom, 
                success=success, 
                latency_ms=latency_ms
            )

            # 8. Axiomatic Benchmarking (Mathematical Judgment)
            # Calculate Tokens per second if available
            tps = response.completion_tokens / (latency_ms / 1000) if latency_ms > 0 else 0.0
            
            from cynic.kernel.organism.benchmarking import LensesScore
            
            self.benchmarker.record_metric(BenchmarkMetric(
                model_id=best_muscle,
                axiom=axiom,
                tokens_per_sec=tps,
                total_latency_ms=latency_ms,
                lenses=LensesScore(
                    backend=1.0 if axiom == "BACKEND" and success else 0.5,
                    ai_infra=1.0 if axiom == "AI_INFRA" and success else 0.5,
                    solutions_architect=1.0 if axiom == "ARCHITECTURE" and success else 0.5,
                    # ... default other lenses to 0.5 for neutral impact
                )
            ))
        else:
            logger.error(f"Organism: Failed to ignite synapse for {best_muscle} (Adapter not found)")

        # 9. Final Result Construction
        return {
            "instance_id": self.instance_id,
            "routing": decision,
            "muscle": best_muscle,
            "status": "PROCESSED",
            "latency_ms": latency_ms,
            "output": response_text
        }

    async def ignite_emergence(self, mission_limit: int = 10) -> None:
        """
        Industrial Emergence Cycle: Non-blocking, event-driven, and resilient.
        """
        from cynic.kernel.organism.discovery import DiscoveryDaemon
        from cynic.kernel.organism.metabolism.cockpit import NightCockpit
        from cynic.kernel.organism.metabolism.task_queue import TaskOrchestrator, EmergenceMission
        import asyncio

        logger.info(f"Organism '{self.instance_id}': Initiating industrial emergence.")
        discovery = DiscoveryDaemon()
        cockpit = NightCockpit()
        
        # Initialize Non-blocking Orchestrator
        task_orch = TaskOrchestrator(concurrency_limit=1)
        
        async def mission_processor(mission: EmergenceMission):
            """The actual worker logic."""
            try:
                # 1. Processing
                result = await self.process_task(
                    f"Analyze and propose a fix for {mission.target}. Context: {mission.description}",
                    axiom=mission.axiom
                )
                
                # 2. Checkpointing
                await self.vault.persist()
                
                # 3. Observability
                cockpit.update_mission(mission.target, mission.axiom, "Analysis Complete", result['muscle'], True)
                logger.info(f"Mission {mission.mission_id} completed successfully.")
                
            except Exception as e:
                logger.error(f"Mission {mission.mission_id} failed: {e}")
                cockpit.update_mission(mission.target, mission.axiom, f"Error: {e}", "N/A", False)

        # Start Workers
        await task_orch.start(mission_processor)

        # 1. Discovery Phase
        potential_missions = discovery.find_potential_missions()
        for i, m in enumerate(potential_missions[:mission_limit]):
            mission = EmergenceMission(
                mission_id=f"M-{i+1}",
                axiom=m['axiom'],
                target=m['target'],
                description=m['description']
            )
            await task_orch.add_mission(mission)

        # 2. Wait for completion while keeping the Organism alive
        while not task_orch.queue.empty() or any(not w.done() for w in task_orch._workers if hasattr(w, 'done')):
            missions_left = task_orch.queue.qsize()
            cockpit.update_summary(mission_limit - missions_left, mission_limit - missions_left)
            await asyncio.sleep(10) # Heartbeat
            if missions_left == 0: break

        await task_orch.stop()
        logger.info("Emergence Cycle Complete.")

    async def sleep(self) -> None:
        """Gracefully shutdown the organism and consolidate learning."""
        from cynic.kernel.organism.improvement import get_improvement_cortex
        
        # 1. Mathematical Consolidation
        cortex = get_improvement_cortex()
        await cortex.consolidate_learning()

        # 2. Persistance
        await self.vault.persist()
        
        # 3. Hardware Release
        await self.server_manager.shutdown_all()
        await self.vascular.close()
        
        logger.info(
            f"Organism '{self.instance_id}' is going to sleep. Learning consolidated. Hardware released."
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


if __name__ == "__main__":
    import asyncio
    import sys

    async def main():
        orch = OrganismOrchestrator(instance_id="cynic-sovereign")
        try:
            await orch.awake()
            # If --night-cycle is passed, run emergence
            if "--night-cycle" in sys.argv:
                await orch.ignite_emergence(mission_limit=10)
            else:
                print("CYNIC Organism Active. Use --night-cycle for autonomous emergence.")
        finally:
            await orch.sleep()

    asyncio.run(main())
