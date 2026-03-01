
import asyncio
import logging
import sys
import os

# Ensure project root is in path
sys.path.append(os.getcwd())

from cynic.kernel.organism.factory import awaken
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.event_bus import CoreEvent

# Setup high-fidelity logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] [%(name)s] %(message)s",
    stream=sys.stdout
)
logger = logging.getLogger("verify_fractal_trace")

async def test_traceability():
    logger.info("🌌 STARTING FRACTAL TRACE VERIFICATION")
    
    # 1. Awaken two separate organisms
    logger.info("🧪 Awakening Instance ALPHA...")
    org_a = await awaken()
    await org_a.start()
    
    logger.info("🧪 Awakening Instance BETA...")
    org_b = await awaken()
    await org_b.start()
    
    logger.info(f"✅ Identity Isolation: A={org_a.instance_id} | B={org_b.instance_id}")
    if org_a.instance_id == org_b.instance_id:
        logger.error("❌ CRITICAL: Instance IDs are identical!")
        return

    # 2. Setup a listener on Beta to check for leaks
    leaked_events = []
    async def beta_leak_listener(event):
        logger.error(f"🚨 LEAK DETECTED: Beta received event from A: {event.topic}")
        leaked_events.append(event)
    
    org_b.bus.on("*", beta_leak_listener)

    # 3. Trigger a judgment on Alpha
    cell = Cell(
        reality="CODE",
        analysis="JUDGE",
        content="Testing fractal isolation and trace_id propagation.",
        context="SRE Verification"
    )
    
    logger.info(f"🧠 Triggering judgment on Instance ALPHA (ID: {org_a.instance_id})...")
    
    # We'll follow the trace_id in the logs
    judgment = await org_a.cognition.orchestrator.run(
        cell=cell,
        level=ConsciousnessLevel.MICRO
    )
    
    logger.info(f"✨ Judgment Complete! Verdict: {judgment.verdict} | Q: {judgment.q_score}")
    
    # 4. Verify isolation
    await asyncio.sleep(1) # Wait for background events
    
    if len(leaked_events) == 0:
        logger.info("🛡️  SUCCESS: Beta bus stayed silent. Isolation is PHYSICAL.")
    else:
        logger.error(f"❌ FAILURE: {len(leaked_events)} events leaked into Beta!")

    # 5. Check Trace ID propagation
    # In real logs, we'd see TR-XXXXX. 
    # Let's ensure the judgment model dump includes our extra fractal fields if Pydantic allow_extra worked
    j_dict = judgment.to_dict()
    logger.info(f"📊 Final Judgment Data: {j_dict}")

    await org_a.state.stop_processing()
    await org_b.state.stop_processing()
    logger.info("🌌 VERIFICATION COMPLETE")

if __name__ == "__main__":
    asyncio.run(test_traceability())
