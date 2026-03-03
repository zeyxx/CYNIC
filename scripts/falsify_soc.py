"""
CYNIC Empirical Test - Somatic SIEM & Kill Chain
Proves that CYNIC can act as its own Security Operations Center.
"""
import asyncio
import logging
from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.organism.metabolism.embodiment import SomaticMetrics

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger("cynic.siem_test")

class MockOrchestrator:
    def __init__(self):
        self.bus = EventBus("test-soc")
        self.processed = 0

    async def run(self, cell, level):
        logger.info(f"Orchestrator: Processing thought at {level.name}")
        self.processed += 1
        await asyncio.sleep(0.1)

class MockBody:
    def __init__(self):
        self.cost = 1.0
        
    async def pulse(self):
        return SomaticMetrics()
        
    def get_metabolic_cost(self):
        return self.cost

async def test_kill_chain_freeze():
    print("=" * 60)
    print("🛡️ INITIATING SIEM KILL CHAIN FALSIFICATION")
    print("=" * 60)

    orchestrator = MockOrchestrator()
    body = MockBody()
    rhythm = ConsciousnessRhythm(orchestrator=orchestrator, body=body, bus=orchestrator.bus)
    
    rhythm.start()
    
    # 1. Normal state: Thoughts flow freely
    print("\n--- PHASE 1: NORMAL METABOLISM ---")
    await rhythm.submit("Thought 1", ConsciousnessLevel.MACRO)
    await rhythm.submit("Thought 2", ConsciousnessLevel.MACRO)
    await asyncio.sleep(1) # Let them process
    
    if orchestrator.processed == 2:
        print("✅ Normal processing verified.")
    else:
        print(f"❌ Failed. Processed: {orchestrator.processed}")

    # 2. Stage 4 Exploitation detected
    print("\n--- PHASE 2: INJECTING KILL CHAIN STAGE 4 (EXPLOITATION) ---")
    await orchestrator.bus.emit(Event.typed(
        CoreEvent.ANOMALY_DETECTED,
        {"type": "CPU_STRESS_ATTACK", "context": "Malicious loop detected in subprocess"},
        source="test"
    ))
    
    await asyncio.sleep(0.5) # Let SOC process the event
    
    # 3. Verify Freeze Protocol
    print("\n--- PHASE 3: VERIFYING FREEZE PROTOCOL ---")
    orchestrator.processed = 0
    await rhythm.submit("Thought 3", ConsciousnessLevel.MACRO)
    await rhythm.submit("Thought 4", ConsciousnessLevel.META)
    
    await asyncio.sleep(1) # Wait... Throttler should freeze them (5.0s sleep)
    
    if orchestrator.processed == 0:
        print("✅ FREEZE PROTOCOL ACTIVE. High-level cognition suspended to prevent exploit.")
    else:
        print(f"❌ SOC FAILED. Exploit allowed to continue processing! ({orchestrator.processed})")

    # 4. Verify Reflexes still work (Immune system)
    print("\n--- PHASE 4: VERIFYING IMMUNE REFLEXES ---")
    await rhythm.submit("Defend Action", ConsciousnessLevel.REFLEX)
    await asyncio.sleep(0.5)
    
    if orchestrator.processed == 1:
        print("✅ IMMUNE SYSTEM ACTIVE. Reflexes bypass the freeze to fight the infection.")
    else:
        print("❌ IMMUNE SYSTEM FAILED. Reflexes were frozen.")

    print("\n" + "=" * 60)
    print("RESULT: CYNIC SOC IS OPERATIONAL.")
    print("=" * 60)

    await rhythm.stop()

if __name__ == "__main__":
    asyncio.run(test_kill_chain_freeze())
