"""
Trace which Dog blocks in MICRO consciousness.

Instrument each Dog to log when it's called and when it returns.
"""

import pytest
import asyncio
import time
from unittest.mock import patch


@pytest.mark.asyncio
async def test_which_dog_blocks_in_micro():
    """
    Run orchestrator.run(MICRO) and trace which Dog hangs.
    """

    print("\n" + "="*70)
    print("TRACING WHICH DOG BLOCKS IN MICRO")
    print("="*70)

    from cynic.api.state import awaken
    from cynic.core.judgment import Cell
    from cynic.core.consciousness import ConsciousnessLevel

    organism = awaken(db_pool=None)

    cell = Cell(
        cell_id="trace_dogs_001",
        reality="CODE",
        analysis="JUDGE",
        content="def test(): pass",
        context="Simple test",
        time_dim="PRESENT",
        risk=0.1,
        budget_usd=0.5,
    )

    print("\nOrchestrator dogs available:")
    sage_dog = None
    for dog_id, dog in organism.orchestrator.dogs.items():
        is_llm = hasattr(dog, 'llm_id') or 'Ollama' in str(type(dog))
        marker = " [LLM]" if is_llm else ""
        print(f"  - {dog_id}: {type(dog).__name__}{marker}")
        if dog_id == "SAGE":
            sage_dog = dog

    if sage_dog:
        print(f"\n*** SAGE Dog detected: {type(sage_dog).__name__}")
        print("    SAGE is the LLM dog — will call Ollama")

    print("\nAttempting orchestrator.run(MICRO) with Dog tracing...")
    print("(Watch which dog doesn't return)\n")

    # Instrument all dogs to log their execution
    original_analyze_methods = {}
    for dog_id, dog in organism.orchestrator.dogs.items():
        original_analyze_methods[dog_id] = dog.analyze

        async def make_traced_analyze(dog_id_closure, original_method):
            async def traced_analyze(cell, **kwargs):
                print(f"  [{dog_id_closure}] analyze() CALLED")
                t0 = time.time()
                try:
                    result = await asyncio.wait_for(
                        original_method(cell, **kwargs),
                        timeout=5.0
                    )
                    elapsed = (time.time() - t0) * 1000
                    print(f"  [{dog_id_closure}] analyze() RETURNED in {elapsed:.0f}ms")
                    return result
                except asyncio.TimeoutError:
                    elapsed = (time.time() - t0) * 1000
                    print(f"  [{dog_id_closure}] analyze() TIMEOUT after {elapsed:.0f}ms ← BLOCKER")
                    raise
            return traced_analyze

        dog.analyze = await make_traced_analyze(dog_id, original_analyze_methods[dog_id])

    try:
        t0 = time.time()
        judgment = await asyncio.wait_for(
            organism.orchestrator.run(cell, ConsciousnessLevel.MICRO),
            timeout=15.0
        )
        elapsed = (time.time() - t0) * 1000
        print(f"\n✓ orchestrator.run() completed in {elapsed:.0f}ms")
        print(f"  Verdict: {judgment.verdict}")
        return True

    except asyncio.TimeoutError:
        elapsed = (time.time() - t0) * 1000
        print(f"\n✗ orchestrator.run() TIMEOUT after {elapsed:.0f}ms")
        print("\nDIAGNOSIS: One of the Dogs above did not return")
        return False

    except Exception as e:
        print(f"\n✗ Error: {e}")
        import traceback
        traceback.print_exc()
        return False


if __name__ == "__main__":
    import asyncio
    asyncio.run(test_which_dog_blocks_in_micro())
