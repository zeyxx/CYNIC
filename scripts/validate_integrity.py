"""
CI/CD Integrity Script — Agent-side validation.
STRICT VERSION: No silent failures.
"""
import asyncio
import logging
import sys
import traceback

# Enable info logging to see where it stops
logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("integrity")

async def validate():
    print("\n--- 🔍 CYNIC STRICT INTEGRITY AUDIT ---")
    
    try:
        # 1. Module Imports
        print("Step 1: Core Imports...", end=" ", flush=True)
        from cynic.kernel.organism.brain.cognition.neurons.discovery import discover_dogs
        from cynic.kernel.organism.organism import awaken
        from cynic.kernel.core.formulas import get_respiration_interval_s
        print("OK")

        # 2. Dog Discovery
        print("Step 2: Sefirotic Discovery...", end=" ", flush=True)
        dogs = discover_dogs()
        if len(dogs) != 11:
            print(f"FAIL: Found {len(dogs)}/11 Dogs")
            return False
        print("OK (11 Dogs)")

        # 3. Life Cycle
        print("Step 3: Organism Life Cycle (Awaken -> Start -> Stop)...", flush=True)
        o = await awaken()
        await o.start()
        print("   - Respiration Active")
        await o.stop()
        print("   - Graceful Stop OK")

        print("\n✅ ALL SYSTEMS NOMINAL")
        return True

    except Exception as e:
        print("\n❌ CRITICAL INTEGRITY FAILURE")
        print("-" * 40)
        traceback.print_exc()
        print("-" * 40)
        return False

if __name__ == "__main__":
    success = asyncio.run(validate())
    sys.exit(0 if success else 1)
