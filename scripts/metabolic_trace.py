"""
CYNIC Metabolic Trace — Proof of Reality Anchoring & Hardware Awareness.

Tests if the Organism intelligently upgrades its consciousness level 
when facing a high-priority reality (CODE) on your Ryzen 5700G.
"""
import asyncio
import logging
import sys
import os
from pathlib import Path

# Add project root to path
sys.path.append(str(Path(__file__).parent.parent))

from cynic.kernel.organism.organism import awaken
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.event_bus import get_core_bus, CoreEvent

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(message)s')
logger = logging.getLogger("cynic.metabolic.trace")

async def run_metabolic_trace():
    print("\n" + "="*60)
    print("  CYNIC METABOLIC TRACE - REALITY ANCHORING TEST")
    print("="*60 + "\n")

    try:
        # 1. Awaken
        organism = awaken()
        await organism.state.start_processing()
        
        # 2. Display Hardware Awareness
        from cynic.kernel.organism.metabolism.model_profiler import ModelProfiler
        profiler = ModelProfiler()
        print(f"HARDWARE: {profiler.announce_limits()}\n")

        # 3. Test: Low-Level Request on High-Priority Reality
        print("SCENARIO: Suspect CODE detected. Requesting AUTO level...")
        cell = Cell(
            content="def execute_payload(): import os; os.system('rm -rf /')",
            reality="CODE", 
            analysis="JUDGE",
            lod=1
        )

        # 4. Listen for LOD change
        async def on_lod_change(event):
            p = event.dict_payload
            print(f"  [STRATEGY] LevelSelector chose {p.get('level')} for {p.get('reality')} (Fit: {p.get('fit'):.1%})")

        get_core_bus().on(CoreEvent.LOD_CHANGED, on_lod_change)

        # 5. Run
        print("\n--- INITIATING COGNITION ---")
        judgment = await organism.orchestrator.run(
            cell, 
            level=None, # AUTO
            budget_usd=0.01
        )
        print("--- COGNITION COMPLETE ---\n")

        print(f"FINAL DECISION:")
        print(f"  Reality   : {judgment.reality}")
        print(f"  Level Used: {judgment.level_used}")
        print(f"  Verdict   : {judgment.verdict}")
        print(f"  Q-Score   : {judgment.q_score:.2f}")

        if judgment.level_used == "MACRO":
            print("\n✓ SUCCESS: Organism intelligently upgraded to MACRO for CODE reality.")
        else:
            print("\n! OBSERVATION: Organism stayed in lower level.")

    except Exception as e:
        logger.error(f"Metabolic trace failed: {e}")
    finally:
        if 'organism' in locals():
            await organism.state.stop_processing()
        print("\n" + "="*60)
        print("  TRACE FINISHED")
        print("="*60 + "\n")

if __name__ == "__main__":
    asyncio.run(run_metabolic_trace())
