"""
Trace awaken() startup — Find the hang.

Instruments awaken() with detailed timing for each component.
"""

import pytest
import time
import sys
from pathlib import Path


def log_timing(step: str, elapsed_ms: float):
    """Log timing to both stdout and file."""
    msg = f"[{elapsed_ms:7.0f}ms] {step}"
    print(msg)
    with open(Path.home() / ".cynic" / "trace_awaken.log", "a") as f:
        f.write(msg + "\n")


@pytest.mark.asyncio
async def test_trace_awaken_startup():
    """
    Trace awaken() initialization to find hangs.

    Instruments:
    - Import time
    - Component initialization time
    - First orchestrator.run() call
    """

    # Clear log
    log_file = Path.home() / ".cynic" / "trace_awaken.log"
    log_file.parent.mkdir(parents=True, exist_ok=True)
    log_file.write_text("")

    print("\n" + "="*70)
    print("TRACING AWAKEN() STARTUP")
    print("="*70 + "\n")

    t0_total = time.time()

    # Step 1: Import
    print("Step 1: Importing modules...")
    t0 = time.time()
    from cynic.api.state import awaken
    from cynic.core.judgment import Cell, infer_time_dim
    from cynic.core.consciousness import ConsciousnessLevel
    log_timing("Imports", (time.time() - t0) * 1000)

    # Step 2: Awaken
    print("\nStep 2: Calling awaken(db_pool=None)...")
    t0 = time.time()

    # Instrument awaken by monkey-patching
    import cynic.api.state as state_module
    orig_OrganismAwakener = state_module._OrganismAwakener

    class InstrumentedAwakener(orig_OrganismAwakener):
        def build(self):
            print("  [awaken] Starting build...")
            t_build_start = time.time()

            # Call parent's build method
            result = super().build()

            elapsed = (time.time() - t_build_start) * 1000
            log_timing("  awaken().build()", elapsed)
            return result

    # Monkey patch
    state_module._OrganismAwakener = InstrumentedAwakener

    try:
        organism = awaken(db_pool=None)
        log_timing("Awaken complete", (time.time() - t0_total) * 1000)
        print(f"  Organism uptime: {organism.uptime_s:.2f}s")
    finally:
        # Restore
        state_module._OrganismAwakener = orig_OrganismAwakener

    # Step 3: First orchestrator.run()
    print("\nStep 3: Creating test cell...")
    t0 = time.time()
    cell = Cell(
        cell_id="trace_001",
        reality="CODE",
        analysis="JUDGE",
        content="x = 1",
        context="Minimal test",
        time_dim="PRESENT",
        risk=0.1,
        budget_usd=0.1,
    )
    log_timing("Cell creation", (time.time() - t0) * 1000)

    print("\nStep 4: Running orchestrator.run(MICRO level) with 30s timeout...")
    print("  (MICRO should call Ollama — expect 1000+ ms)")
    t0 = time.time()

    try:
        import asyncio
        judgment = await asyncio.wait_for(
            organism.orchestrator.run(cell, ConsciousnessLevel.MICRO),
            timeout=30.0
        )
        elapsed_total = (time.time() - t0) * 1000
        log_timing(f"orchestrator.run() COMPLETE", elapsed_total)
        log_timing(f"  Verdict: {judgment.verdict}", 0)
        log_timing(f"  Q-Score: {judgment.q_score:.1f}", 0)
        print(f"\n✓ SUCCESS: Orchestrator completed in {elapsed_total:.0f}ms")

    except asyncio.TimeoutError:
        elapsed = (time.time() - t0) * 1000
        log_timing(f"orchestrator.run() TIMEOUT after {elapsed:.0f}ms", elapsed)
        print(f"\n✗ TIMEOUT: orchestrator.run() did not complete in 10s")
        raise

    elapsed_total = (time.time() - t0_total) * 1000
    log_timing(f"TOTAL TIME", elapsed_total)

    print(f"\n" + "="*70)
    print(f"Trace saved to: {log_file}")
    print("="*70 + "\n")


if __name__ == "__main__":
    import asyncio
    asyncio.run(test_trace_awaken_startup())
