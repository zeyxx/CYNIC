import asyncio
import logging
import sys

# Configure strict logging
logging.basicConfig(level=logging.DEBUG)

async def test_awaken():
    try:
        from cynic.kernel.organism.factory import awaken
        print("✅ Imports successful.")
        
        print("🧠 Attempting to awaken the organism...")
        organism = awaken()
        print("✅ Organism successfully built.")
        print(f"🧬 Anatomy: {organism.cognition}, {organism.metabolism}")
        return 0
    except Exception as e:
        print(f"❌ FATAL ERROR DURING AWAKENING: {type(e).__name__}: {e}")
        import traceback
        traceback.print_exc()
        return 1

if __name__ == "__main__":
    sys.exit(asyncio.run(test_awaken()))
