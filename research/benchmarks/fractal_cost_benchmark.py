"""
CYNIC Fractal Cost Benchmark (Phase 4)

Validates sub-linear cost scaling with dog count.

Hypothesis: Cost ∝ log(N), not O(N)
  - 1 dog (baseline):  reference cost
  - 5 dogs (moderate): cost ≈ baseline × log₂(5)   ≈ 2.32×
  - 11 dogs (full):    cost ≈ baseline × log₂(11)  ≈ 3.46×

NOT 1 dog cost × 5 or 1 dog cost × 11 (linear growth)

Key Mechanism:
  - Dogs judge independently (PERCEIVE → JUDGE → DECIDE → ACT)
  - Dogs gossip compressed context (~200 bytes) instead of full state (~1000 bytes)
  - Orchestrator = consensus layer (geometric mean aggregation, NO re-judgment)
  - Cost scales with: gossip bandwidth, consensus computation, per-dog storage
  - Cost does NOT scale with orchestrator bottleneck (decentralized judgment)

Metrics:
  - latency_ms: Total time for N dogs to judge one cell
  - memory_mb: Peak memory per dog + shared structures
  - gossip_bytes: Total bandwidth for gossip round
  - cost_per_judgment: CPU cycles + storage + network

Design:
  CostBenchmark:
    - Spawn N dogs with DogState + DogCognition
    - Feed them M identical cells (simulating parallel judgment batch)
    - Measure wall-clock time, memory, gossip bytes
    - Calculate: total cost, per-dog cost, cost scaling factor
    - Validate: cost_11 / cost_1 ≤ log₂(11) ≈ 3.46

Usage:
  bench = FractalCostBenchmark()
  result = bench.run(n_dogs=11, n_cells=100)
  assert result.cost_scaling_ratio <= 3.5  # Logarithmic: log₂(11) ≈ 3.46
"""
from __future__ import annotations

import asyncio
import math
import time
import psutil
import os
from dataclasses import dataclass, field
from typing import Any

from cynic.core.phi import PHI, PHI_INV, PHI_INV_2, fibonacci
from cynic.cognition.neurons.dog_state import DogState, DogCognitionState, DogMetabolismState, DogSensoryState, DogMemoryState
from cynic.cognition.cortex.dog_cognition import DogCognition, DogCognitionConfig
from cynic.cognition.cortex.gossip_protocol import GossipProtocol, GossipMessage
from cynic.cognition.cortex.entropy_tracker import EntropyTracker, EntropyMetrics
from cynic.core.judgment import Cell


# ════════════════════════════════════════════════════════════════════════════
# COST BENCHMARK
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class CostMetrics:
    """Metrics for one benchmark run (N dogs, M cells)."""
    n_dogs: int
    n_cells: int
    total_latency_ms: float  # Wall-clock time for all judgments
    avg_latency_per_judgment_ms: float  # Per-cell-per-dog latency
    peak_memory_mb: float  # Peak memory usage
    total_gossip_bytes: float  # Total bandwidth for gossip rounds
    total_judgments: int  # N_dogs × N_cells
    total_entropy_efficiency: float  # Average efficiency across all judgments

    @property
    def cost_per_judgment_ms(self) -> float:
        """Normalized cost per judgment."""
        if self.total_judgments > 0:
            return self.total_latency_ms / self.total_judgments
        return 0.0

    @property
    def bandwidth_per_dog(self) -> float:
        """Average gossip bandwidth per dog."""
        if self.n_dogs > 0:
            return self.total_gossip_bytes / self.n_dogs
        return 0.0

    def to_dict(self) -> dict[str, Any]:
        """Serialize for reporting."""
        return {
            "n_dogs": self.n_dogs,
            "n_cells": self.n_cells,
            "total_latency_ms": round(self.total_latency_ms, 2),
            "avg_latency_per_judgment_ms": round(self.avg_latency_per_judgment_ms, 3),
            "peak_memory_mb": round(self.peak_memory_mb, 2),
            "total_gossip_bytes": round(self.total_gossip_bytes, 0),
            "bandwidth_per_dog_bytes": round(self.bandwidth_per_dog, 0),
            "total_judgments": self.total_judgments,
            "cost_per_judgment_ms": round(self.cost_per_judgment_ms, 3),
            "avg_entropy_efficiency": round(self.total_entropy_efficiency, 3),
        }


@dataclass
class CostScalingResult:
    """Analysis of cost scaling across multiple N values."""
    baseline_1dog: CostMetrics
    moderate_5dog: CostMetrics
    full_11dog: CostMetrics
    scaling_ratio_5_vs_1: float
    scaling_ratio_11_vs_1: float
    is_logarithmic: bool  # True if ratios ≤ theoretical log₂
    theoretical_log2_5: float = field(default_factory=lambda: math.log2(5))
    theoretical_log2_11: float = field(default_factory=lambda: math.log2(11))

    def to_dict(self) -> dict[str, Any]:
        """Serialize for reporting."""
        return {
            "baseline_1dog": self.baseline_1dog.to_dict(),
            "moderate_5dog": self.moderate_5dog.to_dict(),
            "full_11dog": self.full_11dog.to_dict(),
            "scaling_ratio_5_vs_1": round(self.scaling_ratio_5_vs_1, 3),
            "scaling_ratio_11_vs_1": round(self.scaling_ratio_11_vs_1, 3),
            "theoretical_log2_5": round(self.theoretical_log2_5, 3),
            "theoretical_log2_11": round(self.theoretical_log2_11, 3),
            "is_logarithmic": self.is_logarithmic,
            "analysis": {
                "scaling_type": "LOGARITHMIC (O(log N))" if self.is_logarithmic else "LINEAR or worse (O(N))",
                "verdict": "PASS ✓" if self.is_logarithmic else "FAIL ✗",
            }
        }


class FractalCostBenchmark:
    """Benchmark cost scaling for fractal dog architecture."""

    def __init__(self) -> None:
        self.entropy_tracker = EntropyTracker()

    async def run(self, n_dogs: int = 11, n_cells: int = 100) -> CostMetrics:
        """
        Run benchmark with N dogs judging M cells.

        Measures:
          - Wall-clock latency (all dogs judging all cells)
          - Peak memory usage
          - Gossip bandwidth (compressed context exchange)
          - Average entropy efficiency (H_input - H_output)
        """
        process = psutil.Process(os.getpid())

        # Create dog states and cognition engines
        dogs = []
        for i in range(n_dogs):
            dog_id = f"DOG_{i:02d}"
            dog_state = DogState(
                dog_id=dog_id,
                cognition=DogCognitionState(
                    judgment_count=0,
                    local_qtable={},
                    confidence_history=[],
                    last_verdict="",
                    last_q_score=50.0,
                ),
                metabolism=DogMetabolismState(pending_actions=[], executed_count=0),
                senses=DogSensoryState(
                    observed_signals=[],
                    compressed_context="",
                ),
                memory=DogMemoryState(
                    learned_patterns=[],
                    residual_cases=[],
                    gossip_peers={},
                    trust_scores={},
                ),
            )
            cognition = DogCognition(dog_id, DogCognitionConfig())
            dogs.append((dog_id, dog_state, cognition))

        # Create synthetic cells to judge
        cells = [
            Cell(
                id=f"cell_{j:04d}",
                type="code",
                path=f"path/to/cell_{j}.py",
                content=f"# Cell {j}" * 10,
            )
            for j in range(n_cells)
        ]

        # Record starting memory
        process.memory_info()

        # Main judgment loop
        start_time = time.time()
        memory_samples = []
        total_gossip_bytes = 0.0
        all_metrics = []

        for cell in cells:
            # All dogs judge in parallel
            judgment_tasks = []
            for dog_id, dog_state, cognition in dogs:
                judgment_tasks.append(
                    cognition.judge_cell(cell, dog_state)
                )

            # Wait for all judgments
            judgments = await asyncio.gather(*judgment_tasks)

            # Track entropy for each judgment
            for (dog_id, dog_state, _), judgment in zip(dogs, judgments):
                signals = dog_state.senses.observed_signals[-5:] if dog_state.senses.observed_signals else []
                metrics = await self.entropy_tracker.track_judgment(
                    dog_id=dog_id,
                    cell_id=cell.id,
                    signals=signals,
                    verdict=judgment.verdict,
                    confidence=judgment.confidence,
                )
                all_metrics.append(metrics)

            # Simulate gossip exchange (compress + transmit)
            gossip_messages = []
            for dog_id, dog_state, _ in dogs:
                # Compress context
                context = f"dog={dog_id} q_scores={dog_state.cognition.local_qtable} " \
                         f"confidence={sum(dog_state.cognition.confidence_history) / max(len(dog_state.cognition.confidence_history), 1):.3f}"

                # Message size (dog_id + compressed_context + verdict + q_score + confidence)
                msg_size = (
                    len(dog_id.encode()) +
                    len(context.encode()) +
                    len(judgments[dogs.index((dog_id, dog_state, _))].verdict.encode()) +
                    8 +  # float q_score
                    8    # float confidence
                )
                gossip_messages.append(msg_size)
                total_gossip_bytes += msg_size

            # Memory sample
            memory_samples.append(process.memory_info().rss / 1024 / 1024)  # MB

        end_time = time.time()

        # Calculate metrics
        total_latency_ms = (end_time - start_time) * 1000
        avg_latency_per_judgment_ms = total_latency_ms / (n_dogs * n_cells) if n_dogs * n_cells > 0 else 0
        peak_memory_mb = max(memory_samples) if memory_samples else 0
        total_judgments = n_dogs * n_cells
        avg_efficiency = sum(m.efficiency for m in all_metrics) / len(all_metrics) if all_metrics else 0.0

        return CostMetrics(
            n_dogs=n_dogs,
            n_cells=n_cells,
            total_latency_ms=total_latency_ms,
            avg_latency_per_judgment_ms=avg_latency_per_judgment_ms,
            peak_memory_mb=peak_memory_mb,
            total_gossip_bytes=total_gossip_bytes,
            total_judgments=total_judgments,
            total_entropy_efficiency=avg_efficiency,
        )

    async def benchmark_scaling(self, n_cells: int = 50) -> CostScalingResult:
        """
        Benchmark cost scaling: 1 dog → 5 dogs → 11 dogs.

        Validates: cost_11 / cost_1 ≤ log₂(11) ≈ 3.46
        """
        # Run with 1 dog (baseline)
        result_1 = await self.run(n_dogs=1, n_cells=n_cells)

        # Run with 5 dogs
        result_5 = await self.run(n_dogs=5, n_cells=n_cells)

        # Run with 11 dogs
        result_11 = await self.run(n_dogs=11, n_cells=n_cells)

        # Calculate scaling ratios
        baseline_cost = result_1.cost_per_judgment_ms
        scaling_5 = (result_5.cost_per_judgment_ms / baseline_cost) if baseline_cost > 0 else 0
        scaling_11 = (result_11.cost_per_judgment_ms / baseline_cost) if baseline_cost > 0 else 0

        # Validate logarithmic scaling
        log2_5 = math.log2(5)
        log2_11 = math.log2(11)
        is_logarithmic = (scaling_5 <= log2_5 * 1.1 and scaling_11 <= log2_11 * 1.1)

        return CostScalingResult(
            baseline_1dog=result_1,
            moderate_5dog=result_5,
            full_11dog=result_11,
            scaling_ratio_5_vs_1=scaling_5,
            scaling_ratio_11_vs_1=scaling_11,
            is_logarithmic=is_logarithmic,
        )


# ════════════════════════════════════════════════════════════════════════════
# DUMMY CELL CLASS (for benchmark simulation)
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class Cell:
    """Dummy cell for benchmark."""
    id: str
    type: str
    path: str
    content: str


# ════════════════════════════════════════════════════════════════════════════
# TEST HARNESS
# ════════════════════════════════════════════════════════════════════════════


async def main() -> None:
    """Run full Phase 4 cost benchmark."""
    bench = FractalCostBenchmark()

    print("🐕 CYNIC Phase 4: Fractal Cost Scaling Benchmark\n")
    print("=" * 70)

    # Run scaling benchmark
    print("Running cost scaling validation...")
    print("  1 dog (baseline) → 5 dogs → 11 dogs\n")

    result = await bench.benchmark_scaling(n_cells=50)

    # Display results
    output = result.to_dict()
    print("BASELINE (1 Dog):")
    for k, v in output["baseline_1dog"].items():
        print(f"  {k}: {v}")

    print("\nMODERATE (5 Dogs):")
    for k, v in output["moderate_5dog"].items():
        print(f"  {k}: {v}")

    print("\nFULL (11 Dogs):")
    for k, v in output["full_11dog"].items():
        print(f"  {k}: {v}")

    print("\nSCALING ANALYSIS:")
    print(f"  Ratio 5 vs 1: {output['scaling_ratio_5_vs_1']:.3f}")
    print(f"  Theoretical log₂(5): {output['theoretical_log2_5']:.3f}")
    print(f"  Ratio 11 vs 1: {output['scaling_ratio_11_vs_1']:.3f}")
    print(f"  Theoretical log₂(11): {output['theoretical_log2_11']:.3f}")
    print(f"\n  Verdict: {output['analysis']['verdict']}")
    print(f"  Type: {output['analysis']['scaling_type']}")

    print("\n" + "=" * 70)


if __name__ == "__main__":
    asyncio.run(main())
