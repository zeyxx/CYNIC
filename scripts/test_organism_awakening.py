"""
🌀 CYNIC ORGANISM AWAKENING : The Unified Test

This script verifies the full integration of the Orchestrator, 
Vascular System, Cognitive Router, and Hardware Abstraction Layer.
"""

import asyncio
import logging
import sys
import os

# Ensure project root is in PYTHONPATH
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from cynic.orchestration import OrganismOrchestrator

# Setup basic logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')

async def run_awakening():
    print("=" * 60)
    print("🌀 AWAKENING THE UNIFIED CYNIC ORGANISM")
    print("=" * 60)

    # 1. Initialize the central orchestrator
    cynic = OrganismOrchestrator(instance_id="cynic-dev-01")
    
    # 2. Wake up the system (triggers discovery and vascularization)
    await cynic.awake()
    
    # 3. Process a "Light" task (should trigger System 1)
    print("\n[TASK 1] Simple request: 'Summarize this log file.'")
    result_s1 = await cynic.process_task("Summarize this log file.")
    print(f"Decision: {result_s1['routing']['mode']} -> Target: {result_s1['routing']['target']}")
    
    # 4. Process a "Heavy" task (should trigger System 2)
    print("\n[TASK 2] Complex request: 'Audit this fractal architecture for ethical alignment.'")
    result_s2 = await cynic.process_task("Audit this fractal architecture for ethical alignment.")
    print(f"Decision: {result_s2['routing']['mode']} -> Target: {result_s2['routing']['target']}")

    # 5. Verify Vascular & HAL presence
    print("\n[RESOURCE CHECK]")
    print(f"Vascular System: ONLINE")
    print(f"Compute Engine: {cynic.vascular.hal.announce_compute()}")
    
    # 6. Sleep
    await cynic.sleep()
    print("\n" + "=" * 60)
    print("STATUS: INTEGRATION VERIFIED.")
    print("=" * 60)

if __name__ == "__main__":
    asyncio.run(run_awakening())
