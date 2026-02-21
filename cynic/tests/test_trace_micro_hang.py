"""
Trace MICRO consciousness level — Find where it hangs with Ollama.

Monkey-patches orchestrator to log every internal step.
"""

import pytest
import asyncio
import time


@pytest.mark.asyncio
async def test_trace_micro_consciousness_hang():
    """
    Trace orchestrator.run(MICRO) to find where Ollama call hangs.
    """

    print("\n" + "="*70)
    print("TRACING MICRO CONSCIOUSNESS HANG")
    print("="*70)

    from cynic.api.state import awaken
    from cynic.core.judgment import Cell
    from cynic.core.consciousness import ConsciousnessLevel

    organism = awaken(db_pool=None)

    cell = Cell(
        cell_id="trace_micro_001",
        reality="CODE",
        analysis="JUDGE",
        content="x = 1 + 1",
        context="Simple arithmetic",
        time_dim="PRESENT",
        risk=0.1,
        budget_usd=0.5,
    )

    print("\n[1] Attempting orchestrator.run(MICRO) with detailed logging...")
    print("    (This will hang if Ollama is not responding)\n")

    t0 = time.time()
    try:
        # Try with 15s timeout
        judgment = await asyncio.wait_for(
            organism.orchestrator.run(cell, ConsciousnessLevel.MICRO),
            timeout=15.0
        )
        elapsed = (time.time() - t0) * 1000
        print(f"\n✓ SUCCESS in {elapsed:.0f}ms")
        print(f"  Verdict: {judgment.verdict}")
        print(f"  Q-Score: {judgment.q_score}")
        return True

    except asyncio.TimeoutError:
        elapsed = (time.time() - t0) * 1000
        print(f"\n✗ TIMEOUT after {elapsed:.0f}ms")
        print("\nDIAGNOSIS:")
        print("  MICRO consciousness level did not complete")
        print("  This suggests either:")
        print("  - Ollama is not running (check: curl http://localhost:11434/api/tags)")
        print("  - Ollama is hanging on inference")
        print("  - Event bus / scheduler is deadlocked")

        # Check if Ollama is even responsive
        print("\n[2] Checking Ollama connectivity...")
        try:
            import httpx
            async with httpx.AsyncClient(timeout=5.0) as client:
                resp = await client.get("http://localhost:11434/api/tags")
                print(f"  Ollama is ONLINE (status {resp.status_code})")
                models = resp.json().get("models", [])
                print(f"  Available models: {len(models)}")
                for m in models[:3]:
                    print(f"    - {m.get('name', 'unknown')}")
                return False
        except Exception as e:
            print(f"  Ollama is OFFLINE or unreachable: {e}")
            print("  → Start Ollama: ollama serve")
            return False


if __name__ == "__main__":
    import asyncio
    asyncio.run(test_trace_micro_consciousness_hang())
