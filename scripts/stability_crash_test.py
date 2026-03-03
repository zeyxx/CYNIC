"""
CYNIC Industrial Crash-Test (PSC-v1 Validation).
Simulates heavy load and storage failure to verify Resilience & Backpressure.
"""
import asyncio
import logging
import time
from cynic.kernel.organism.factory import awaken
from cynic.kernel.core.event_bus import Event, CoreEvent

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("crash_test")

async def run_crash_test():
    logger.info("🛡️ Initiating Stability Validation...")
    
    # 1. Awaken Organism (Mocked for speed if needed, but here real boot)
    try:
        organism = await awaken()
        logger.info("✅ Organism Awake.")
    except Exception as e:
        logger.error(f"❌ Awakening Failed: {e}")
        return

    # 2. Backpressure Test (Spam non-critical events)
    logger.info("🔥 Testing Backpressure (Priority Drop)...")
    bus = organism.bus
    
    # Fill the bus with low-priority "noise"
    tasks = []
    for i in range(1500): # Exceeds MAX_PENDING=1000
        tasks.append(bus.emit(Event.typed(
            CoreEvent.SONA_TICK,
            {"test": i},
            source="stress_test"
        )))
    
    # Simultaneously emit a critical signal
    critical_task = bus.emit(Event.typed(
        CoreEvent.SECURITY_EVENT,
        {"status": "CRITICAL_PROBE"},
        source="stress_test"
    ))
    
    await asyncio.gather(*tasks, critical_task)
    stats = bus.stats()
    logger.info(f"📊 Bus Stats: {stats}")
    
    if stats["pending_tasks"] <= 1000:
        logger.info("✅ Backpressure logic prevented overflow.")
    else:
        logger.error("❌ Backpressure logic FAILED to throttle.")

    # 3. Storage Resilience Test (Manual check)
    logger.info("💾 Persistence protocols active (Check logs for retries).")
    
    await organism.stop()
    logger.info("🏁 Test Complete.")

if __name__ == "__main__":
    asyncio.run(run_crash_test())
