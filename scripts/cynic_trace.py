"""
CYNIC End-to-End Trace — Prove stability and peek inside the black box.

This script simulates a real governance proposal and traces its path
through the 7-step cycle, showing dog votes, consensus, and action execution.
"""
import asyncio
import logging
import sys
import os
from pathlib import Path

# Add project root to path
sys.path.append(str(Path(__file__).parent.parent))

from cynic.organism.organism import awaken
from cynic.core.judgment import Cell
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.event_bus import get_core_bus, CoreEvent

# Setup detailed logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s',
    stream=sys.stdout
)
logger = logging.getLogger("cynic.trace")

async def run_trace():
    print("\n" + "="*60)
    print("  CYNIC STABILITY PROOF — END-TO-END TRACE")
    print("="*60 + "\n")

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
            print(f"  [BUS EVENT] {event.type.value}")

        bus = get_core_bus()
        bus.on(CoreEvent.JUDGMENT_CREATED, trace_listener)
        bus.on(CoreEvent.ACT_COMPLETED, trace_listener)

        # 4. Run Judgment through the Orchestrator
        logger.info("Step 3: Running full 7-step cycle...")
        print("\n--- CYCLE START ---")
        
        judgment = await organism.orchestrator.run(
            cell,
            level=ConsciousnessLevel.MICRO,
            budget_usd=0.05
        )

        print("--- CYCLE COMPLETE ---\n")

        # 5. Display the "Black Box" data
        logger.info("Step 4: Decoding the Result")
        print(f"\nVERDICT: {judgment.verdict}")
        print(f"Q-SCORE: {judgment.q_score:.2f}")
        print(f"CONFIDENCE: {judgment.confidence:.2%}")
        
        print("\n[DOG VOTES]")
        if hasattr(judgment, 'dog_votes') and judgment.dog_votes:
            for dog, score in judgment.dog_votes.items():
                print(f"  - {dog:<12}: {score:.2f}")
        else:
            print("  (no dog votes found)")

        print("\n[AXIOM SCORES]")
        if hasattr(judgment, 'axiom_scores') and judgment.axiom_scores:
            for axiom, score in judgment.axiom_scores.items():
                print(f"  - {axiom:<12}: {score:.2f}")
        else:
            print("  (no axiom scores found)")

        # 6. Verify State Persistence
        logger.info("Step 5: Verifying State Manager (awaiting memory consolidation)...")
        # Give the event bus a moment to process the JUDGMENT_CREATED background task
        await asyncio.sleep(0.1) 
        
        recent = organism.state.get_recent_judgments(limit=1)
        if recent and recent[0].get("judgment_id") == judgment.judgment_id:
            logger.info("✓ Success: Judgment consolidated in OrganismState")
        else:
            logger.warning("✗ Failure: Judgment not found in state")

        # 7. Check Guidance File (Feedback Loop)
        guidance_path = Path.home() / ".cynic" / "guidance.json"
        if guidance_path.exists():
            logger.info(f"✓ Success: Feedback loop closed (guidance.json exists)")
        else:
            logger.warning("✗ Failure: guidance.json missing")

    except Exception as e:
        logger.error(f"Trace failed with error: {e}", exc_info=True)
    finally:
        if 'organism' in locals():
            await organism.state.stop_processing()
            # Give EventBus tasks a moment to clear
            await asyncio.sleep(0.5)
        print("\n" + "="*60)
        print("  TRACE FINISHED")
        print("="*60 + "\n")

if __name__ == "__main__":
    asyncio.run(run_trace())
