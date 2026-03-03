"""
CYNIC Fractal Trace Verification — Zero-Leak Multi-instance Validation.

Tests that two concurrent Organism instances in the same process remain
perfectly isolated via ContextVars and instance-specific EventBuses.
"""

import asyncio
import logging
import os
import sys

# Ensure local cynic package is discoverable
sys.path.insert(0, os.getcwd())

from cynic.kernel.organism.factory import awaken
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s [%(levelname)s] [%(name)s] %(message)s"
)
logger = logging.getLogger("verify_fractal_trace")


async def run_isolated_instance(name: str):
    """Run a judgment cycle in an isolated instance and track events."""
    logger.info(f"🧪 Awakening Instance {name}...")

    # 1. Awaken (Each call should generate a new isolated instance)
    org = await awaken()
    instance_id = org.instance_id

    events_captured = []

    async def leak_listener(event):
        # We capture the event and its metadata (which now contains instance_id)
        events_captured.append(event)

    # 2. Start the organism
    await org.start()
    logger.info(f"✅ Instance {name} started with ID: {instance_id}")

    # 3. Setup leak detection on the instance's core bus
    bus = org.cognition.orchestrator.bus
    bus.on("*", leak_listener)

    # 4. Trigger a judgment
    cell = Cell(
        reality="CODE",
        analysis="JUDGE",
        content=f"Validation signal from {name}",
        budget_usd=0.01,
    )

    # Run judgment
    try:
        judgment = await org.cognition.orchestrator.run(
            cell, level=ConsciousnessLevel.MICRO
        )
        logger.info(f"✨ {name} Judgment Complete: {judgment.verdict}")

        # Wait a bit for all async handlers to finish
        await asyncio.sleep(0.5)
    finally:
        await org.stop()

    return {"name": name, "instance_id": instance_id, "events": events_captured}


async def test_traceability():
    """Run ALPHA and BETA concurrently and verify zero leakage."""
    logger.info("🌌 STARTING FRACTAL ISOLATION TEST")

    # Run ALPHA and BETA in parallel tasks
    results = await asyncio.gather(
        run_isolated_instance("ALPHA"),
        run_isolated_instance("BETA"),
        return_exceptions=True,
    )

    for r in results:
        if isinstance(r, Exception):
            logger.error(f"❌ Instance failed: {r}", exc_info=True)
            return

    alpha, beta = results

    # 5. VERIFY ISOLATION
    alpha_id = alpha["instance_id"]
    beta_id = beta["instance_id"]

    logger.info(f"ALPHA Instance ID: {alpha_id}")
    logger.info(f"BETA Instance ID:  {beta_id}")

    # Check for cross-contamination
    # An event is a leak if its metadata.instance_id doesn't match the listener's instance_id
    leaks_in_alpha = [
        e.type for e in alpha["events"] if e.metadata["instance_id"] == beta_id
    ]
    leaks_in_beta = [
        e.type for e in beta["events"] if e.metadata["instance_id"] == alpha_id
    ]

    if not leaks_in_alpha and not leaks_in_beta:
        logger.info("🛡️  SUCCESS: 0 leaks detected. Fractal isolation is total.")
        logger.info(f"  ALPHA captured {len(alpha['events'])} internal events.")
        logger.info(f"  BETA captured {len(beta['events'])} internal events.")
    else:
        logger.error("☢️  FAILURE: Leaks detected!")
        if leaks_in_alpha:
            logger.error(f"  ALPHA received BETA events: {leaks_in_alpha}")
        if leaks_in_beta:
            logger.error(f"  BETA received ALPHA events: {leaks_in_beta}")

    logger.info("🌌 VERIFICATION COMPLETE")


if __name__ == "__main__":
    asyncio.run(test_traceability())
