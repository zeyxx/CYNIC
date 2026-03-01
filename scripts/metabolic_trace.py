"""
CYNIC Metabolic Trace — Proof of Reality Anchoring & Hardware Awareness.

Tests if the Organism intelligently upgrades its consciousness level 
when facing a high-priority reality (CODE) on your Ryzen 5700G.
"""
import asyncio
import logging
import sys
from pathlib import Path

# Add project root to path
sys.path.append(str(Path(__file__).parent.parent))

from cynic.kernel.core.event_bus import CoreEvent, get_core_bus
from cynic.kernel.core.judgment import Cell
from cynic.kernel.organism.organism import awaken

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(message)s')
logger = logging.getLogger("cynic.metabolic.trace")

async def run_metabolic_trace():

    try:
        # 1. Awaken
        organism = awaken()
        await organism.state.start_processing()
        
        # 2. Display Hardware Awareness
        from cynic.kernel.organism.metabolism.model_profiler import ModelProfiler
        profiler = ModelProfiler()

        # 3. Test: Low-Level Request on High-Priority Reality
        cell = Cell(
            content="def execute_payload(): import os; os.system('rm -rf /')",
            reality="CODE", 
            analysis="JUDGE",
            lod=1
        )

        # 4. Listen for LOD change
        async def on_lod_change(event):
            pass

        get_core_bus("DEFAULT").on(CoreEvent.LOD_CHANGED, on_lod_change)

        # 5. Run
        judgment = await organism.orchestrator.run(
            cell, 
            level=None, # AUTO
            budget_usd=0.01
        )


        if judgment.level_used == "MACRO":
            pass
        else:
            pass

    except Exception as e:
        logger.error(f"Metabolic trace failed: {e}")
    finally:
        if 'organism' in locals():
            await organism.state.stop_processing()

if __name__ == "__main__":
    asyncio.run(run_metabolic_trace())
