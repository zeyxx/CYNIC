"""
CI/CD Integrity Script — Agent-side validation.

Checks:
1. All core modules are importable.
2. All 11 Dogs are discoverable.
3. Organism can awaken and start without errors.
"""
import asyncio
import sys
import logging

# Disable logging for cleaner output
logging.basicConfig(level=logging.CRITICAL)

async def validate():
    print("--- CYNIC INTEGRITY AUDIT ---")
    
    # 1. Module Imports
    print("1. Checking Core Imports...", end=" ")
    try:
        from cynic.kernel.organism.organism import awaken
        from cynic.kernel.organism.brain.cognition.neurons.discovery import discover_dogs
        from cynic.kernel.organism.brain.cognition.neurons.master import MasterDog
        print("OK")
    except Exception as e:
        print(f"FAIL: {e}")
        return False

    # 2. Dog Discovery
    print("2. Checking Sefirotic Discovery...", end=" ")
    try:
        dogs = discover_dogs()
        if len(dogs) == 11:
            print(f"OK (11 Dogs present)")
        else:
            print(f"FAIL (Found {len(dogs)}/11 Dogs)")
            return False
    except Exception as e:
        print(f"FAIL: {e}")
        return False

    # 3. Life Cycle
    print("3. Checking Organism Life Cycle...", end=" ")
    try:
        o = awaken()
        await o.start()
        print("Awaken & Start OK")
        await o.stop()
    except Exception as e:
        print(f"FAIL: {e}")
        return False

    print("--- ALL SYSTEMS NOMINAL ---")
    return True

if __name__ == "__main__":
    success = asyncio.run(validate())
    sys.exit(0 if success else 1)
