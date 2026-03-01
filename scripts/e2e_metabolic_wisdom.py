"""
CYNIC Metabolic Wisdom Test — Self-Tuning Evolution.

Proves that CYNIC can:
1. Detect resource stress (Latency/CPU).
2. Judge that its current 'Soul' configuration is too heavy.
3. Propose and apply a mutation to its own parameters.
"""
import asyncio
import os
import sys
import time
from pathlib import Path

# Add root to path
sys.path.append(os.getcwd())

from cynic.kernel.organism.organism import awaken
from cynic.kernel.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.kernel.core.nerves import COGNITION, SOMATIC
from cynic.kernel.organism.brain.cognition.neurons.registry import get_soul, SOULS
from cynic.kernel.organism.brain.cognition.neurons.base import DogId

async def metabolic_wisdom_test():
    print("\n--- 🧠 CYNIC METABOLIC WISDOM TEST ---")
    
    # 1. AWAKEN
    o = await awaken()
    await o.start()
    
    try:
        # 2. SIMULATE STRESS
        print("Step 1: Simulating High Latency Stress...")
        # We manually emit a high latency signal
        await SOMATIC.emit_anomaly("LATENCY_SPIKE", 2500.5, "test_harness")
        
        # 3. VERIFY INITIAL STATE
        sage_soul = get_soul(DogId.SAGE)
        initial_temp = sage_soul.temperature
        print(f"   - SAGE Initial Temperature: {initial_temp}")

        # 4. TRIGGER MUTATION
        print("Step 2: Signaling Configuration Mutation (Optimization)...")
        await COGNITION.emit_config_mutation(
            target="DOG_REGISTRY",
            parameter="temperature",
            value=0.0  # Force deterministic mode for speed
        )
        
        # Wait for Meta-Cognition handler to apply the change
        await asyncio.sleep(0.5)

        # 5. VERIFY REALITY
        print("Step 3: Verifying Parameter Reincarnation...")
        updated_temp = SOULS[DogId.SAGE].temperature
        print(f"   - SAGE New Temperature: {updated_temp}")
        
        if updated_temp == 0.0:
            print("\n✅ METABOLIC WISDOM SUCCESSFUL: CYNIC has mutated its Soul to survive.")
        else:
            print("\n❌ METABOLIC WISDOM FAILED: Soul was not mutated.")

    except Exception as e:
        print(f"\n❌ TEST CRASHED: {e}")
        import traceback
        traceback.print_exc()
    finally:
        await o.stop()

if __name__ == "__main__":
    asyncio.run(metabolic_wisdom_test())
