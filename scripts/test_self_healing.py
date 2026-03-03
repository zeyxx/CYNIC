"""
🌀 CYNIC SELF-HEALING AUDIT : The Organism Observes Itself

This script simulates a scenario where CYNIC analyzes its own architecture
and performance bottlenecks to propose self-optimizations.
"""

import asyncio
import logging
import sys
import os

# Ensure project root is in PYTHONPATH
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from cynic.kernel.organism.factory import awaken
from cynic.kernel.core.judgment import Cell

# Setup logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')

async def run_self_healing_audit():
    print("=" * 60)
    print("🌀 STARTING CYNIC SELF-HEALING AUDIT")
    print("=" * 60)

    organism = None
    try:
        # 1. Awaken the organism
        organism = await awaken()
        
        # We will use ANALYST (Code Quality) and SAGE (Wisdom/Architecture)
        analyst = organism.cognition.orchestrator.dogs.get("ANALYST")
        sage = organism.cognition.orchestrator.dogs.get("SAGE")
        
        if not analyst or not sage:
            print("❌ CRITICAL: Required Dogs (ANALYST or SAGE) not found in the organism.")
            return

        # 2. Prepare the Pain Point (The Bottleneck we discovered earlier)
        pain_point_content = """
        [PERFORMANCE BOTTLENECK DETECTED]
        Location: cynic.kernel.core.phi.weighted_geometric_mean
        Symptom: CPU saturation during deep fractal judgments (N=8).
        Current Latency: ~27 seconds per judgment on Ryzen 5700G.
        Target Latency: < 5ms for 10k TPS.
        
        Code Snapshot:
        def weighted_geometric_mean(values: list[float], weights: list[float]) -> float:
            # Current Python Implementation
            log_sum = sum(w * math.log(v) for v, w in zip(values, weights, strict=False))
            return math.exp(log_sum / sum(weights))
        """

        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            time_dim="FUTURE", # We want a forward-looking solution
            content=pain_point_content,
            context="The organism is experiencing metabolic pain during high-frequency reasoning. Propose an architectural remedy."
        )
        
        print(f"\n[DIAGNOSIS] Organism is analyzing its own pain point: Cell {cell.cell_id}")

        # 3. Parallel Judgment (The Dogs evaluate the pain)
        print("\n[COGNITION] Waking up ANALYST and SAGE to evaluate the pain point...")
        
        # We run them concurrently. Note: on local LLMs this might be slow or queue up.
        # For a 5700G, we might want to do it sequentially if RAM is tight, 
        # but let's try parallel first to see if Ollama queues them nicely.
        
        judgments = await asyncio.gather(
            analyst.analyze(cell),
            sage.analyze(cell)
        )
        
        analyst_judgment = judgments[0]
        sage_judgment = judgments[1]

        # 4. Results
        print("\n" + "-" * 60)
        print("🧠 COGNITIVE REPORTS")
        print("-" * 60)
        
        print(f"\n🐶 ANALYST (Score: {analyst_judgment.q_score:.2f}, Confidence: {analyst_judgment.confidence:.2f})")
        print(f"Reasoning: {analyst_judgment.reasoning}")
        print(f"Latency: {analyst_judgment.latency_ms:.0f}ms")
        
        print(f"\n🐶 SAGE (Score: {sage_judgment.q_score:.2f}, Confidence: {sage_judgment.confidence:.2f})")
        print(f"Reasoning: {sage_judgment.reasoning}")
        print(f"Latency: {sage_judgment.latency_ms:.0f}ms")

        # 5. Consensus (The Organism decides)
        # We simulate the PBFT run
        cynic_coordinator = organism.cognition.orchestrator.cynic_dog
        consensus = await cynic_coordinator.phi_bft_run(cell, [analyst_judgment, sage_judgment])
        
        print("\n" + "=" * 60)
        print(f"⚖️ ORGANISM VERDICT: {consensus.final_verdict}")
        print(f"FINAL Q-SCORE: {consensus.final_q_score:.2f}")
        print("=" * 60)

    except Exception as e:
        print(f"\n❌ AUDIT FAILED: {e}")
        import traceback
        traceback.print_exc()
    finally:
        if organism:
            await organism.stop()

if __name__ == "__main__":
    asyncio.run(run_self_healing_audit())
