"""
CYNIC End-to-End Trace — Prove stability and peek inside the black box.

This script simulates a real governance proposal and traces its path
through the 7-step cycle, showing dog votes, consensus, and action execution.
"""
import asyncio
import logging
import sys
from pathlib import Path

# Add project root to path
sys.path.append(str(Path(__file__).parent.parent))

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.event_bus import CoreEvent, get_core_bus
from cynic.kernel.core.judgment import Cell
from cynic.kernel.organism.organism import awaken

# Setup detailed logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
    stream=sys.stdout
)
logger = logging.getLogger("cynic.trace")

async def run_trace():

    try:
        # 1. Awaken the Organism
        logger.info("Step 1: Awakening the Organism...")
        organism = awaken()
        await organism.state.start_processing(db=None)
        logger.info(f"Organism AWAKE and RESPIRING. Level: {organism.state.get_consciousness_level()}")

        # 2. Prepare a "Social" (Governance) Proposal
        proposal_content = "Should we increase the PBFT threshold to 82% (phi^2) for critical security decisions?"
        logger.info(f"Step 2: Preparing Proposal: '{proposal_content}'")
        
        cell = Cell(
            content=proposal_content,
            reality="SOCIAL",
            analysis="JUDGE",
            lod=1
        )

        # 3. Register a listener to prove the event bus is alive
        events_captured = []
        async def trace_listener(event):
            events_captured.append(event.type)
            event.type.value if hasattr(event.type, "value") else str(event.type)

        bus = get_core_bus()
        bus.on(CoreEvent.JUDGMENT_CREATED, trace_listener)
        bus.on(CoreEvent.ACT_COMPLETED, trace_listener)

        # 4. Run Judgment through the Orchestrator
        logger.info("Step 3: Running full 7-step cycle...")
        
        judgment = await organism.orchestrator.run(
            cell,
            level=ConsciousnessLevel.MICRO,
            budget_usd=0.05
        )


        # 5. Display the "Black Box" data
        logger.info("Step 4: Decoding the Result")
        
        if hasattr(judgment, 'dog_votes') and judgment.dog_votes:
            for _dog, _score in judgment.dog_votes.items():
                pass
        else:
            pass

        if hasattr(judgment, 'axiom_scores') and judgment.axiom_scores:
            for _axiom, _score in judgment.axiom_scores.items():
                pass
        else:
            pass

        # 6. Verify State Persistence
        logger.info("Step 5: Verifying State Manager (awaiting memory consolidation)...")
        await asyncio.sleep(0.5) 
        
        recent = organism.state.get_recent_judgments(limit=1)
        if recent and recent[0].judgment_id == judgment.judgment_id:
            logger.info("✓ Success: Judgment consolidated in OrganismState")
        else:
            logger.warning("✗ Failure: Judgment not found in state")

        # 7. Check Guidance File (Feedback Loop)
        guidance_path = Path.home() / ".cynic" / "guidance.json"
        if guidance_path.exists():
            logger.info("✓ Success: Feedback loop closed (guidance.json exists)")
        else:
            logger.warning("✗ Failure: guidance.json missing")

    except Exception as e:
        logger.error(f"Trace failed with error: {e}", exc_info=True)
    finally:
        if 'organism' in locals():
            await organism.state.stop_processing()
            await asyncio.sleep(0.5)

if __name__ == "__main__":
    asyncio.run(run_trace())
