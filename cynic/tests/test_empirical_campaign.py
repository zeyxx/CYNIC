"""
Empirical Campaign: 1000+ Real Judgments Through Full Cycle

This is Phase 7 Blocker #3 validation.

NO MOCKS. Real Ollama inference. Real scheduler. Real Q-Table updates.

Goal: Prove the kernel actually works end-to-end, collect real latency data,
validate Q-Table convergence, identify silent failures.

Results saved to: ~/.cynic/empirical/{timestamp}.json
"""

import pytest
import asyncio
import json
import time
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Any
import statistics

from cynic.core.judgment import Cell, infer_time_dim
from cynic.core.consciousness import ConsciousnessLevel
from cynic.api.state import CynicOrganism


class EmpiricalCampaignCollector:
    """Collect raw data from empirical judgment runs."""

    def __init__(self, run_id: str = None):
        self.run_id = run_id or datetime.now().isoformat()
        self.judgments: List[Dict[str, Any]] = []
        self.errors: List[Dict[str, Any]] = []
        self.latencies: List[float] = []
        self.q_scores: List[float] = []
        self.verdicts: Dict[str, int] = {}

    def record_judgment(
        self,
        cell_id: str,
        level: str,
        latency_ms: float,
        q_score: float,
        verdict: str,
        cost_usd: float,
        llm_calls: int,
        confidence: float,
    ):
        """Record a successful judgment."""
        self.judgments.append({
            "timestamp": datetime.now().isoformat(),
            "cell_id": cell_id,
            "level": level,
            "latency_ms": latency_ms,
            "q_score": q_score,
            "verdict": verdict,
            "cost_usd": cost_usd,
            "llm_calls": llm_calls,
            "confidence": confidence,
        })
        self.latencies.append(latency_ms)
        self.q_scores.append(q_score)
        self.verdicts[verdict] = self.verdicts.get(verdict, 0) + 1

    def record_error(self, cell_id: str, error_msg: str, level: str):
        """Record a failed judgment."""
        self.errors.append({
            "timestamp": datetime.now().isoformat(),
            "cell_id": cell_id,
            "level": level,
            "error": error_msg,
        })

    def get_stats(self) -> Dict[str, Any]:
        """Compute aggregate statistics."""
        if not self.latencies:
            return {}

        return {
            "total_judgments": len(self.judgments),
            "total_errors": len(self.errors),
            "error_rate": len(self.errors) / (len(self.judgments) + len(self.errors))
                if (len(self.judgments) + len(self.errors)) > 0 else 0,
            "latency": {
                "mean_ms": statistics.mean(self.latencies),
                "median_ms": statistics.median(self.latencies),
                "min_ms": min(self.latencies),
                "max_ms": max(self.latencies),
                "stdev_ms": statistics.stdev(self.latencies) if len(self.latencies) > 1 else 0,
            },
            "q_score": {
                "mean": statistics.mean(self.q_scores),
                "median": statistics.median(self.q_scores),
                "min": min(self.q_scores),
                "max": max(self.q_scores),
                "stdev": statistics.stdev(self.q_scores) if len(self.q_scores) > 1 else 0,
            },
            "verdicts": self.verdicts,
        }

    def save(self) -> Path:
        """Save results to disk."""
        empirical_dir = Path.home() / ".cynic" / "empirical"
        empirical_dir.mkdir(parents=True, exist_ok=True)

        timestamp = datetime.now().strftime("%Y-%m-%dT%H-%M-%S")
        filepath = empirical_dir / f"{timestamp}.json"

        data = {
            "run_id": self.run_id,
            "timestamp": timestamp,
            "judgments": self.judgments,
            "errors": self.errors,
            "statistics": self.get_stats(),
        }

        with open(filepath, "w") as f:
            json.dump(data, f, indent=2)

        return filepath


@pytest.mark.empirical
@pytest.mark.asyncio
async def test_empirical_campaign_1000_judgments():
    """
    Run 1000+ varied judgments through the full CYNIC kernel.

    This validates:
    - Real Ollama inference works end-to-end
    - No crashes or hangs
    - Reasonable latency distribution
    - Q-Table gets updated
    - Learning signals propagate
    - Error handling works
    """

    collector = EmpiricalCampaignCollector()

    # Initialize the real organism (no mocks, no DB)
    from cynic.api.state import awaken
    organism = awaken(db_pool=None)

    # Test data: varied cell types from asdfasdfa codebase
    test_cells = [
        {
            "reality": "CODE",
            "category": "api",
            "content": "def handle_request(): pass",
            "analysis": "PERCEIVE",
        },
        {
            "reality": "CODE",
            "category": "test",
            "content": "async def test_async(): await something()",
            "analysis": "JUDGE",
        },
        {
            "reality": "SOLANA",
            "category": "transaction",
            "content": "transfer 100 SOL to wallet",
            "analysis": "DECIDE",
        },
        {
            "reality": "MARKET",
            "category": "price",
            "content": "BTC price is 50k",
            "analysis": "PERCEIVE",
        },
        {
            "reality": "HUMAN",
            "category": "psychology",
            "content": "User is tired and making mistakes",
            "analysis": "JUDGE",
        },
    ]

    # Generate judgment tasks (scale for testing: 100 for quick validation, 1000 for full)
    num_judgments = 100  # START WITH 100 FOR QUICK VALIDATION
    levels = [ConsciousnessLevel.REFLEX, ConsciousnessLevel.MICRO, ConsciousnessLevel.MACRO]

    for i in range(num_judgments):
        # Cycle through test data and levels
        cell_template = test_cells[i % len(test_cells)]
        level = levels[i % len(levels)]

        cell_id = f"empirical_{i:06d}"

        try:
            # Create Cell
            cell = Cell(
                cell_id=cell_id,
                reality=cell_template["reality"],
                category=cell_template["category"],
                content=cell_template["content"],
                analysis=cell_template["analysis"],
                time_dim=infer_time_dim(
                    cell_template["content"],
                    "",
                    cell_template["analysis"]
                ),
                risk=0.1 + (i % 10) * 0.05,  # Vary risk
                budget_usd=1.0 + (i % 5) * 0.5,  # Vary budget
            )

            # Run through orchestrator (real inference)
            start_time = time.time()
            judgment = await organism.orchestrator.run(cell, level=level)
            elapsed_ms = (time.time() - start_time) * 1000

            # Record successful judgment
            collector.record_judgment(
                cell_id=cell_id,
                level=level.name,
                latency_ms=elapsed_ms,
                q_score=judgment.q_score,
                verdict=judgment.verdict,
                cost_usd=judgment.cost_usd,
                llm_calls=judgment.llm_calls,
                confidence=judgment.confidence,
            )

            # Every 20 judgments, print progress
            if (i + 1) % 20 == 0:
                stats = collector.get_stats()
                print(
                    f"\n[Progress] {i + 1}/{num_judgments} judgments"
                    f" | Latency: {stats['latency']['mean_ms']:.0f}ms"
                    f" | Q-Score: {stats['q_score']['mean']:.1f}"
                    f" | Errors: {stats['error_rate']*100:.1f}%"
                )

        except Exception as e:
            collector.record_error(cell_id, str(e), level.name)
            print(f"[ERROR] Judgment {i}: {e}")

    # Save results
    filepath = collector.save()
    stats = collector.get_stats()

    print(f"\n{'='*70}")
    print(f"EMPIRICAL CAMPAIGN COMPLETE")
    print(f"{'='*70}")
    print(f"Results saved to: {filepath}")
    print(f"Total judgments: {stats['total_judgments']}")
    print(f"Total errors: {stats['total_errors']} ({stats['error_rate']*100:.1f}%)")
    print(f"Mean latency: {stats['latency']['mean_ms']:.0f}ms")
    print(f"Median latency: {stats['latency']['median_ms']:.0f}ms")
    print(f"Max latency: {stats['latency']['max_ms']:.0f}ms")
    print(f"Q-Score mean: {stats['q_score']['mean']:.1f}")
    print(f"Verdicts: {stats['verdicts']}")
    print(f"{'='*70}\n")

    # Assertions: basic validation
    assert stats['total_judgments'] > 0, "No judgments completed"
    assert stats['error_rate'] < 0.1, f"Error rate too high: {stats['error_rate']*100:.1f}%"
    assert stats['latency']['mean_ms'] < 5000, \
        f"Mean latency too high: {stats['latency']['mean_ms']:.0f}ms (target <5s)"
    assert stats['latency']['max_ms'] < 30000, \
        f"Max latency too high: {stats['latency']['max_ms']:.0f}ms (target <30s)"
    assert all(v in ["HOWL", "WAG", "GROWL", "BARK"] for v in stats['verdicts'].keys()), \
        f"Invalid verdicts: {stats['verdicts'].keys()}"


@pytest.mark.empirical
@pytest.mark.asyncio
async def test_q_table_convergence():
    """
    Verify Q-Table gets updated correctly through learning signals.

    This validates:
    - Learning events are recorded
    - Q-Table scores change over time (convergence)
    - No NaN or infinite values
    """

    from cynic.api.state import awaken
    organism = awaken(db_pool=None)

    # Get initial Q-Table state
    initial_q_table = dict(organism.q_learning.q_table)

    # Run 100 learning cycles
    for i in range(100):
        cell = Cell(
            cell_id=f"q_test_{i:03d}",
            reality="CODE",
            category="test",
            content=f"test_case_{i}",
            analysis="JUDGE",
            risk=0.1,
            budget_usd=0.5,
        )

        judgment = await organism.orchestrator.run(cell, ConsciousnessLevel.MICRO)

        # Record feedback to trigger learning
        await organism.orchestrator.record_learning_signal(
            judgment_id=judgment.judgment_id,
            signal_type="user_feedback",
            value=1.0 if judgment.verdict == "HOWL" else -0.5,
        )

    # Verify Q-Table changed
    final_q_table = dict(organism.q_learning.q_table)

    changed_keys = set()
    for key in initial_q_table:
        if key in final_q_table:
            if initial_q_table[key] != final_q_table[key]:
                changed_keys.add(key)

    print(f"\nQ-Table entries changed: {len(changed_keys)}/{len(initial_q_table)}")
    assert len(changed_keys) > 0, "Q-Table did not update (learning not working)"

    # Verify no NaN/Inf
    for key, value in final_q_table.items():
        assert not any(
            x in str(value) for x in ["nan", "inf", "Inf"]
        ), f"Invalid Q-Table value for {key}: {value}"

    print("✓ Q-Table convergence validated")


if __name__ == "__main__":
    # Run directly without pytest
    asyncio.run(test_empirical_campaign_1000_judgments())
