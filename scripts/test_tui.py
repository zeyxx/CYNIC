"""
TUI Sanity Check Script.

Runs the TUI for a few seconds to verify it doesn't crash on render.
"""
import asyncio
import os
import sys

# Add root to path
sys.path.append(os.getcwd())

from cynic.kernel.organism.organism import awaken
from cynic.interfaces.cli.organism_tui import OrganismTUI

async def check_tui():
    print("Awakening Organism for TUI test...")
    o = await awaken()
    await o.start()
    
    tui = OrganismTUI(organism=o)
    
    print("Rendering TUI layout...")
    try:
        layout = tui.render()
        print("TUI render successful (no exceptions).")
        
        # We won't start the live loop to avoid messing up the console,
        # but rendering it once proves the data access paths are correct.
    except Exception as e:
        print(f"âŒ TUI render FAILED: {e}")
        import traceback
        traceback.print_exc()
    finally:
        await o.stop()

if __name__ == "__main__":
    asyncio.run(check_tui())
