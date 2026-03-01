import asyncio
import logging
import sys
import os
import uuid

# Ensure project root is in path
sys.path.append(os.getcwd())

from cynic.kernel.organism.factory import awaken
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel, current_consciousness
from cynic.kernel.core.event_bus import CoreEvent, current_instance_id

# Setup high-fidelity logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] [%(name)s] %(message)s",
    stream=sys.stdout
)
logger = logging.getLogger("verify_fractal_trace")

async def run_isolated_instance(name: str):
    logger.info(f"🧪 Awakening Instance {name}...")
    org = await awaken()
    
    # Set the ContextVars for this task
    token_id = current_instance_id.set(org.instance_id)
    token_cons = current_consciousness.set(org.state.consciousness)
    
    try:
        await org.start()
        logger.info(f"✅ Instance {name} started with ID: {org.instance_id}")
        
        # Setup leak detection
        leaks = []
        async def leak_listener(event):
            # If we receive an event that wasn't emitted by our instance...
            if hasattr(event.payload, 'instance_id') and event.payload.instance_id != org.instance_id:
                logger.error(f"🚨 LEAK! {name} received event from another instance!")
                leaks.append(event)
        
        org.bus.on("*", leak_listener)
        
        # Trigger a judgment only for ALPHA
        if name == "ALPHA":
            cell = Cell(
                reality="CODE",
                analysis="JUDGE",
                content=f"Fractal Trace for {name}",
                context="SRE isolation test"
            )
            judgment = await org.cognition.orchestrator.run(cell, level=ConsciousnessLevel.MICRO)
            logger.info(f"✨ {name} Judgment Complete: {judgment.verdict}")
            
        await asyncio.sleep(2)
        return leaks
        
    finally:
        await org.state.stop_processing()
        current_instance_id.reset(token_id)
        current_consciousness.reset(token_cons)

async def test_traceability():
    logger.info("🌌 STARTING FRACTAL ISOLATION TEST")
    
    # Run two instances concurrently in separate context tasks
    results = await asyncio.gather(
        run_isolated_instance("ALPHA"),
        run_isolated_instance("BETA")
    )
    
    leaks_a, leaks_b = results
    if len(leaks_a) == 0 and len(leaks_b) == 0:
        logger.info("🛡️  SUCCESS: 0 leaks detected. Fractal isolation is total.")
    else:
        logger.error(f"❌ FAILURE: Leaks detected! A:{len(leaks_a)} B:{len(leaks_b)}")

    logger.info("🌌 VERIFICATION COMPLETE")

if __name__ == "__main__":
    asyncio.run(test_traceability())
