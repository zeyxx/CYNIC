"""
CYNIC 10-Minute Reality Observation - Controlled Life Cycle.
Observes the organism in action: Learning, Resource Appropriation, and Judging.
"""
import asyncio
import logging
import time
from cynic.orchestration import OrganismOrchestrator

logging.basicConfig(
    level=logging.INFO, 
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s"
)
logger = logging.getLogger("cynic.observation")

TASKS = [
    ("BACKEND", "Fixing a silent failure in the event forwarder logic."),
    ("ARCHITECTURE", "Refactoring the kernel to use decentralized event sourcing."),
    ("SECURITY", "Auditing the multi-tenant isolation of the RAG pipeline."),
    ("FIDELITY", "Ensuring the training-serving consistency of the inference engine."),
    ("BURN", "Optimizing memory usage by pruning unused synapses."),
    ("PHI", "Calculating the Phi-bounded confidence score for a new model."),
    ("VERIFY", "Implementing a new pre-commit hook for commit message length."), # Meta!
    ("CULTURE", "Checking if the code follows Chip Huyen's ML design principles."),
    ("BACKEND", "Implementing a shared connection pool for the vascular system."),
    ("ARCHITECTURE", "Mapping the fractal dependencies of the nervous system.")
]

async def observe_life():
    print("\n" + "!"*60)
    print("🚀 CYNIC REALITY OBSERVATION: THE 10-MINUTE LIFE CYCLE")
    print("!"*60)
    
    orch = OrganismOrchestrator(instance_id="cynic-observatory")
    await orch.awake()
    
    start_time = time.time()
    task_count = 0
    
    try:
        for axiom, desc in TASKS:
            current_elapsed = (time.time() - start_time) / 60
            print(f"\n--- [T+{current_elapsed:.1f} min] Task {task_count+1}: {axiom} ---")
            
            # Execute with full organic loop
            result = await orch.process_task(desc, axiom=axiom)
            
            print(f"  -> Muscle Used: {result['muscle']}")
            print(f"  -> Routing Decision: {result['routing']['target']}")
            
            # Simulate real work delay (average 60 seconds per task for a 10min run)
            # We'll do it faster for the CLI demo, but keep the logic
            await asyncio.sleep(2) 
            
            task_count += 1
            
    except KeyboardInterrupt:
        logger.info("Observation interrupted by user.")
    finally:
        await orch.sleep()
        print("\n" + "!"*60)
        print(f"🏁 OBSERVATION COMPLETE. {task_count} tasks processed.")
        print("!"*60)

if __name__ == "__main__":
    asyncio.run(observe_life())
