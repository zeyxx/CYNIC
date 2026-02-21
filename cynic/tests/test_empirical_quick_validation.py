"""
Quick Empirical Validation (10 judgments)

Phase 7 Blocker #3 - Minimal viable empirical test.

This is the REAL TEST: 10 real Ollama inferences through the full kernel.
If this passes, we know the cycle works. If it fails, we find the blocker.

Results saved to: ~/.cynic/empirical/{timestamp}.json
"""

import pytest
import asyncio
import json
import time
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Any

from cynic.core.judgment import Cell, infer_time_dim
from cynic.core.consciousness import ConsciousnessLevel
from cynic.api.state import awaken


@pytest.mark.asyncio
async def test_empirical_quick_10_judgments():
    """
    Run 10 real judgments. Quick validation of the kernel.

    This answers: Does the kernel actually work end-to-end?
    - Can it boot?
    - Can it initialize Ollama?
    - Can it run a judgment?
    - Does ConsciousState record the result?
    - What's the actual latency?
    """

    print("\n" + "="*70)
    print("EMPIRICAL QUICK VALIDATION: 10 REAL JUDGMENTS")
    print("="*70)

    # Initialize organism
    print("[INIT] Awakening organism...")
    organism = awaken(db_pool=None)
    print(f"[OK] Organism awakened")
    print(f"  Uptime: {organism.uptime_s:.1f}s")
    print(f"  Orchestrator: {type(organism.orchestrator).__name__}")
    print(f"  QTable: {type(organism.qtable).__name__}")
    print(f"  Memory: {type(organism.memory).__name__}")

    # Test data
    test_cases = [
        ("CODE", "Python function review", "def hello(): pass"),
        ("CODE", "Error handling", "try: x = 1/0\nexcept: pass"),
        ("SOLANA", "Transaction validation", "transfer 100 SOL"),
        ("MARKET", "Price analysis", "BTC 50k, trend up"),
        ("HUMAN", "User psychology", "User tired, making mistakes"),
        ("CODE", "Security review", "execute user input"),
        ("CODE", "Performance", "large loop iteration"),
        ("SOLANA", "SPL token", "mint 1M USDC tokens"),
        ("MARKET", "Liquidity", "10M volume, low spread"),
        ("HUMAN", "Burnout risk", "Working 14h days"),
    ]

    results = []
    errors = []

    for idx, (reality, description, content) in enumerate(test_cases):
        try:
            print(f"\n[{idx+1}/10] {reality}:{description[:30]}")

            # Create cell
            cell = Cell(
                cell_id=f"quick_empirical_{idx:02d}",
                reality=reality,
                analysis="JUDGE",
                content=content,
                context=description,
                time_dim=infer_time_dim(content, description, "JUDGE"),
                risk=0.1 + (idx % 5) * 0.15,
                budget_usd=1.0,
            )

            # Run judgment
            start = time.time()
            judgment = await organism.orchestrator.run(cell, ConsciousnessLevel.MICRO)
            elapsed_ms = (time.time() - start) * 1000

            print(f"  Result: {judgment.verdict} (Q={judgment.q_score:.1f}, cost=${judgment.cost_usd:.4f})")
            print(f"  Latency: {elapsed_ms:.0f}ms, LLM calls: {judgment.llm_calls}")

            results.append({
                "idx": idx,
                "reality": reality,
                "verdict": judgment.verdict,
                "q_score": judgment.q_score,
                "latency_ms": elapsed_ms,
                "cost_usd": judgment.cost_usd,
                "llm_calls": judgment.llm_calls,
                "confidence": judgment.confidence,
            })

        except Exception as e:
            print(f"  ERROR: {e}")
            errors.append({"idx": idx, "error": str(e)})

    # Summary
    print(f"\n{'='*70}")
    print(f"RESULTS: {len(results)} passed, {len(errors)} failed")
    print(f"{'='*70}")

    if results:
        latencies = [r["latency_ms"] for r in results]
        costs = [r["cost_usd"] for r in results]
        print(f"\nLatency:")
        print(f"  Mean: {sum(latencies)/len(latencies):.0f}ms")
        print(f"  Min: {min(latencies):.0f}ms")
        print(f"  Max: {max(latencies):.0f}ms")
        print(f"\nCosts:")
        print(f"  Mean: ${sum(costs)/len(costs):.4f}")
        print(f"  Total: ${sum(costs):.4f}")
        print(f"\nVerdicts:")
        verdict_counts = {}
        for r in results:
            v = r["verdict"]
            verdict_counts[v] = verdict_counts.get(v, 0) + 1
        for v, count in sorted(verdict_counts.items()):
            print(f"  {v}: {count}")

    if errors:
        print(f"\nErrors:")
        for e in errors:
            print(f"  [{e['idx']}] {e['error'][:80]}")

    # Save results
    empirical_dir = Path.home() / ".cynic" / "empirical"
    empirical_dir.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now().strftime("%Y-%m-%dT%H-%M-%S")
    filepath = empirical_dir / f"{timestamp}_quick.json"

    with open(filepath, "w") as f:
        json.dump({
            "test_type": "quick_validation",
            "timestamp": timestamp,
            "results": results,
            "errors": errors,
            "summary": {
                "passed": len(results),
                "failed": len(errors),
                "mean_latency_ms": sum([r["latency_ms"] for r in results]) / len(results) if results else 0,
                "total_cost_usd": sum([r["cost_usd"] for r in results]),
            }
        }, f, indent=2)

    print(f"\nResults saved to: {filepath}")
    print(f"{'='*70}\n")

    # Assertions
    assert len(results) > 0, "No judgments completed"
    assert len(results) >= 5, f"Too many failures: {len(errors)}"


if __name__ == "__main__":
    asyncio.run(test_empirical_quick_10_judgments())
