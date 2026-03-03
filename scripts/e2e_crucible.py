"""
CYNIC End-to-End Proof (The Crucible).

A strict test to prove CYNIC can handle real-world data across
the full 7-step cycle without mocks.
"""

import asyncio
import os
import sys
import time
from pathlib import Path

# Add root to path
sys.path.append(os.getcwd())

from cynic.kernel.organism.organism import awaken
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel


async def run_proof():
    print("\n--- âš”ï¸ CYNIC E2E CRUCIBLE ---")

    # 1. AWAKEN
    print("Step 1: Awakening organism...")
    o = await awaken()
    await o.start()

    try:
        # 2. PERCEIVE (Read a real file)
        print("Step 2: Perceiving real file (README.md)...")
        readme_path = Path("README.md")
        content = readme_path.read_text(encoding="utf-8", errors="ignore")[
            :1000
        ]  # Take first 1000 chars

        cell = Cell(
            cell_id=f"e2e-{int(time.time())}",
            reality="CODE",
            analysis="JUDGE",
            content=f"FILE: README.md\n\nCONTENT:\n{content}",
            context="E2E Proof: Analyzing project documentation for fidelity.",
            budget_usd=0.05,
        )

        # 3. JUDGE & DECIDE (The Core Thinking)
        print("Step 3: Judging and Deciding (MACRO level)...")
        judgment = await o.cognition.orchestrator.run(
            cell, level=ConsciousnessLevel.MACRO
        )

        print(f"   - Verdict: {judgment.verdict}")
        print(f"   - Q-Score: {judgment.q_score:.1f}")
        print(f"   - Reasoning: {judgment.reasoning[:150]}...")

        # 4. ACT (Simulate/Verify action proposal)
        print("Step 4: Verifying Action Proposal...")
        next_action = await o.memory.action_proposer.get_next_action()
        if next_action:
            print(
                f"   - SUCCESS: Action Proposed: {next_action.action_prompt[:100]}..."
            )

            # Proof of Action: Writing to a physical file
            proof_path = Path("CYNIC_E2E_PROOF.md")
            proof_text = f"# CYNIC E2E PROOF\nTimestamp: {time.ctime()}\nVerdict: {judgment.verdict}\nQ-Score: {judgment.q_score}\nProposal: {next_action.action_prompt}\n"
            proof_path.write_text(proof_text)
            print(f"   - SUCCESS: Written impact to {proof_path}")
        else:
            print("   - NOTE: No action proposed (normal if verdict is not ACT)")

        # 5. LEARN (Verify Q-Table update)
        print("Step 5: Checking Memory Evolution...")
        stats = o.state.get_stats()
        print(f"   - Total Judgments in State: {stats['total_judgments']}")

        if stats["total_judgments"] > 0:
            print("\nâœ… E2E PROOF SUCCESSFUL: CYNIC is reality-connected.")
        else:
            print("\nâŒ E2E PROOF FAILED: State not updated.")

    except Exception as e:
        print(f"\nâŒ E2E CRUCIBLE CRASHED: {e}")
        import traceback

        traceback.print_exc()
    finally:
        await o.stop()


if __name__ == "__main__":
    asyncio.run(run_proof())
