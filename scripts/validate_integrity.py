"""
CI/CD Integrity Script — Agent-side validation.

Checks:
1. All core modules are importable.
2. All 11 Dogs are discoverable.
3. Organism can awaken and start without errors.
"""
import asyncio
import logging
import sys

# Disable logging for cleaner output
logging.basicConfig(level=logging.CRITICAL)

async def validate():
    
    # 1. Module Imports
    try:
        from cynic.kernel.organism.brain.cognition.neurons.discovery import discover_dogs
        from cynic.kernel.organism.organism import awaken
    except Exception:
        return False

    # 2. Dog Discovery
    try:
        dogs = discover_dogs()
        if len(dogs) == 11:
            pass
        else:
            return False
    except Exception:
        return False

    # 3. Life Cycle
    try:
        o = awaken()
        await o.start()
        await o.stop()
    except Exception:
        return False

    return True

if __name__ == "__main__":
    success = asyncio.run(validate())
    sys.exit(0 if success else 1)
