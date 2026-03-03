"""
CYNIC Foundation Falsifier — The Ordeal of Truth.

This script attempts to prove that the current foundation is UNSTABLE.
If this script PASSES, it means the foundation is FAILED.
If this script FAILS (cannot find errors), the foundation is SOLID.

Lentilles : SRE (Chaos), Data Engineer (Integrity), AI Infra (Awareness).
"""

import asyncio
import logging
import sys
import time
import uuid

from cynic.kernel.organism.factory import awaken
from cynic.kernel.core.event_bus import CoreEvent, Event

# Setup Aggressive Logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("cynic.falsifier")

async def falsify():
    logger.info("🔥 STARTING THE ORDEAL OF TRUTH...")
    
    # 1. Awaken Organism
    try:
        org = await awaken()
        await org.start()
        logger.info(f"Instance {org.instance_id} awakened.")
    except Exception as e:
        logger.error(f"❌ FATAL: Organism failed to awaken: {e}")
        sys.exit(1)

    # --- PILLAR 1: LLM AWARENESS ---
    logger.info("Checking LLM Muscles...")
    available = org.cognition.llm_registry.get_available()
    if not available:
        logger.warning("🚩 Falsification Successful: Organism is BRAIN-DEAD (No LLM adapters).")
    else:
        logger.info(f"✅ Foundation Resisted: {len(available)} LLM adapters found.")

    # --- PILLAR 2: REACTIVE TRUTH (SurrealDB Live) ---
    if org.state.storage:
        logger.info("Checking Reactive Truth (Live Queries)...")
        initial_judgments = org.state.total_judgments
        
        # Manually inject into DB bypassing the bus
        logger.info("Injecting ghost judgment directly into SurrealDB...")
        ghost_judgment = {
            "judgment_id": f"ghost-{uuid.uuid4().hex[:8]}",
            "verdict": "WAG",
            "q_score": 61.8,
            "created_at": time.time()
        }
        
        try:
            await org.state.storage.judgments.save(ghost_judgment)
            # Give Live Query 1s to ripple back to RAM
            await asyncio.sleep(1.0)
            
            if org.state.total_judgments == initial_judgments:
                logger.warning("🚩 Falsification Successful: Organism is BLIND (RAM did not react to DB write).")
            else:
                logger.info("✅ Foundation Resisted: RAM mirrored DB change automatically.")
        except Exception as e:
            logger.error(f"❌ DB Injection error: {e}")
    else:
        logger.warning("🚩 Falsification Successful: Organism is AMNESIC (No storage configured).")

    # --- PILLAR 3: NERVOUS BACKPRESSURE ---
    logger.info("Checking Nervous Backpressure...")
    # Flood the bus
    for i in range(100):
        await org.bus.emit(Event.typed(CoreEvent.ANOMALY_DETECTED, {"n": i}, source="falsifier"))
    
    pending = len(org.bus._pending_tasks)
    logger.info(f"Bus load: {pending} tasks.")
    
    # Stop and check for orphans
    logger.info("Stopping organism...")
    await org.stop()
    
    remaining = len(asyncio.all_tasks()) - 1 # exclude current task
    if remaining > 5: # Some background loops might linger, but not 100s
        logger.warning(f"🚩 Falsification Successful: Organism has LEAKS ({remaining} tasks lingering).")
    else:
        logger.info("✅ Foundation Resisted: Clean shutdown achieved.")

    logger.info("🔥 ORDEAL COMPLETE.")

if __name__ == "__main__":
    asyncio.run(falsify())
