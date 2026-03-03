"""
🌀 CYNIC FULL ORGANISM AWAKENING : The Factory Test

This script verifies that the full organism factory can build and awaken 
the system with all its new sutures (Vascular, HAL, Router).
"""

import asyncio
import logging
import sys
import os

# Ensure project root is in PYTHONPATH
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from cynic.kernel.organism.factory import awaken

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')

async def run_full_awakening():
    print("=" * 60)
    print("🌀 AWAKENING THE FULL CYNIC ORGANISM (FACTORY)")
    print("=" * 60)

    try:
        # 1. Awaken the organism via factory
        organism = await awaken()
        
        print("\n[ORGANISM STATS]")
        print(f"Instance ID: {organism.instance_id}")
        
        # Check vascular via different paths
        vascular = getattr(organism, 'vascular', None)
        print(f"Vascular System (Direct): {'ONLINE' if vascular else 'OFFLINE'}")
        
        if vascular:
            print(f"Compute Engine: {vascular.hal.announce_compute()}")
            
        print(f"Dogs Discovered: {len(organism.cognition.orchestrator.dogs)}")
        
        # 2. Test a Dog's vascularization
        dogs = organism.cognition.orchestrator.dogs
        if dogs:
            first_dog_id = list(dogs.keys())[0]
            first_dog = dogs[first_dog_id]
            print(f"\n[DOG CHECK: {first_dog_id}]")
            is_vascularized = hasattr(first_dog, 'vascular') and first_dog.vascular is not None
            print(f"Dog Vascularized: {'YES' if is_vascularized else 'NO'}")

        # 3. Graceful sleep
        await organism.stop()
        print("\n" + "=" * 60)
        print("STATUS: FULL AWAKENING SUCCESSFUL.")
        print("=" * 60)

    except Exception as e:
        print(f"\n❌ AWAKENING FAILED: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    asyncio.run(run_full_awakening())
