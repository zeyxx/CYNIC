"""
Kernel Diagnostics — Find the hang.

Runs directly (no pytest), writes to file, captures every step.
"""

import asyncio
import time
import sys
from pathlib import Path

# Write to file + stdout
log_file = Path.home() / ".cynic" / "diagnose_kernel.log"
log_file.parent.mkdir(parents=True, exist_ok=True)

def log(msg):
    """Log to both file and stdout."""
    timestamp = time.time()
    line = f"[{timestamp:.1f}] {msg}\n"
    print(line, end="")
    with open(log_file, "a") as f:
        f.write(line)

async def main():
    log("START: Kernel diagnostics")

    try:
        log("[1/5] Importing modules...")
        from cynic.api.state import awaken
        from cynic.core.judgment import Cell, infer_time_dim
        from cynic.core.consciousness import ConsciousnessLevel
        log("  OK: Imports successful")

        log("[2/5] Awakening organism...")
        start = time.time()
        organism = awaken(db_pool=None)
        elapsed = time.time() - start
        log(f"  OK: Organism awakened in {elapsed:.1f}s")
        log(f"  - Orchestrator: {type(organism.orchestrator).__name__}")
        log(f"  - Uptime: {organism.uptime_s:.1f}s")

        log("[3/5] Creating test cell...")
        cell = Cell(
            cell_id="diagnose_001",
            reality="CODE",
            analysis="JUDGE",
            content="def test(): pass",
            context="Simple Python function",
            time_dim=infer_time_dim("def test(): pass", "Simple Python function", "JUDGE"),
            risk=0.1,
            budget_usd=0.5,
        )
        log("  OK: Cell created")

        log("[4/5] Running orchestrator (THIS IS WHERE IT HANGS)...")
        log("  Waiting for orchestrator.run()...")
        start = time.time()

        # Run with timeout
        try:
            judgment = await asyncio.wait_for(
                organism.orchestrator.run(cell, ConsciousnessLevel.REFLEX),
                timeout=30.0  # 30s timeout
            )
            elapsed = time.time() - start
            log(f"  OK: Judgment completed in {elapsed:.1f}s")
            log(f"  - Verdict: {judgment.verdict}")
            log(f"  - Q-Score: {judgment.q_score:.1f}")
            log(f"  - Cost: ${judgment.cost_usd:.4f}")
            log(f"  - LLM calls: {judgment.llm_calls}")
        except asyncio.TimeoutError:
            elapsed = time.time() - start
            log(f"  TIMEOUT: orchestrator.run() did not complete in 30s")
            log(f"  Elapsed: {elapsed:.1f}s")
            log("  DIAGNOSIS: Hang is in orchestrator.run() or one of its dependencies")
            return False

        log("[5/5] SUCCESS: Kernel works end-to-end")
        log("END: All diagnostics passed")
        return True

    except Exception as e:
        import traceback
        log(f"ERROR: {e}")
        log("TRACEBACK:")
        log(traceback.format_exc())
        log("END: Diagnostics failed with exception")
        return False

if __name__ == "__main__":
    log("="*70)
    log("KERNEL DIAGNOSTIC SCRIPT")
    log("="*70)

    success = asyncio.run(main())

    log("="*70)
    if success:
        log("RESULT: Kernel functional")
        sys.exit(0)
    else:
        log("RESULT: Kernel has blocker")
        sys.exit(1)
