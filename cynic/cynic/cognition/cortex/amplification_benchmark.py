"""
CYNIC Amplification Benchmark (Experiment #2)

Measures whether accumulated session knowledge (warm QTable) produces
faster/better convergence than cold start (Q=0.5 neutral).

This is the core amplification hypothesis:
  After W prior judgments, a warm-started learner should outperform
  a cold-started learner at the same step budget.

Design:
  WarmStartLearner:
    Phase 1 — accumulate W judgments on real probe data → snapshot Q-values
    Phase 2 — fresh learner inherits those Q-values + visit counts (EWC uses them)
              runs T test steps on same task
  ColdStartLearner:
    Phase 1 — skip
    Phase 2 — fresh learner starts at Q=0.5, runs same T test steps

Amplification ratio at step T:
  ratio(T) = cold_error(T) / warm_error(T)
  ratio > 1 → amplification confirmed (warm start is better)
  ratio ≈ 1 → no amplification
  ratio < 1 → negative transfer (warm start hurts)

Grid:
  warm_levels: {0, 50, 100, 200, 500}  (session history depth)
  configs: phi (α=0.038, EWC=True) vs standard (α=0.1, EWC=False)
  n_seeds: 7

Key phi advantage:
  phi + EWC = warm values are PROTECTED after consolidation.
  standard RL: warm values get overwritten as fast as cold values.
  → phi amplification_ratio should be systematically higher than standard.
"""
from __future__ import annotations

import random
import time
from dataclasses import dataclass, field

from typing import Optional

from cynic.core.phi import fibonacci
from cynic.cognition.cortex.qtable_benchmark import (
    QEntry,
    TD0Learner,
    ConvergenceResult,
    _PHI_ALPHA,
    _STD_ALPHA,
    _ALPHA_GRID,
    _CONVERGENCE_EPS,
    _EWC_CONSOLIDATE_AT,
)
from cynic.cognition.cortex.real_benchmark import (
    RealKernelTask,
    _N_PAIRS_REAL,
)


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# Warm-up levels: number of prior session judgments
_WARM_LEVELS: list[int] = [0, fibonacci(5), fibonacci(7), 200, 500]
# = [0, 5, 13, 200, 500]  — Fibonacci-anchored levels + practical depths

# Test budget per session (steps after warm-load)
_TEST_BUDGET: int = 200

# Seed offset between warm phase and test phase (avoid data leakage)
_SEED_OFFSET: int = 1000


# ---------------------------------------------------------------------------
# WarmStartLearner: two-phase learner with knowledge transfer
# ---------------------------------------------------------------------------

@dataclass
class WarmStartResult:
    """Result of one warm vs cold comparison."""
    warm_steps: int          # Prior session judgments (0 = cold)
    alpha: float
    use_ewc: bool
    seed: int
    test_budget: int

    # Cold start errors (no prior knowledge)
    cold_final_error: float
    cold_convergence_step: Optional[int]

    # Warm start errors (pre-loaded Q-values)
    warm_final_error: float
    warm_convergence_step: Optional[int]

    # Amplification
    amplification_ratio: float   # cold_error / warm_error (>1 = amplified)
    knowledge_retention: float   # 1 - warm_error/cold_error (>0 = improved)

    duration_ms: float

    @property
    def label(self) -> str:
        ewc_str = "+EWC" if self.use_ewc else ""
        return f"alpha={self.alpha}{ewc_str} warm={self.warm_steps}"

    @property
    def amplified(self) -> bool:
        return self.amplification_ratio > 1.0

    def to_dict(self) -> dict:
        return {
            "warm_steps": self.warm_steps,
            "alpha": self.alpha,
            "use_ewc": self.use_ewc,
            "cold_final_error": round(self.cold_final_error, 4),
            "warm_final_error": round(self.warm_final_error, 4),
            "amplification_ratio": round(self.amplification_ratio, 4),
            "knowledge_retention": round(self.knowledge_retention, 4),
            "cold_convergence_step": self.cold_convergence_step,
            "warm_convergence_step": self.warm_convergence_step,
            "amplified": self.amplified,
        }


def _run_warm_cold(
    alpha: float,
    use_ewc: bool,
    warm_steps: int,
    test_budget: int,
    seed: int,
) -> WarmStartResult:
    """
    Run one warm-vs-cold comparison.

    Phase 1 (warm only): accumulate warm_steps on real task (seed)
    Phase 2 (both): run test_budget steps on fresh task (seed + SEED_OFFSET)
      - cold: Q=0.5 start
      - warm: Q-values inherited from Phase 1
    """
    t0 = time.perf_counter()

    # ── Phase 2 task (shared between warm and cold) ─────────────────────────
    # Same seed → same noisy reward sequence for fair comparison
    task_cold = RealKernelTask(rng=random.Random(seed + _SEED_OFFSET))
    task_warm = RealKernelTask(rng=random.Random(seed + _SEED_OFFSET))

    # ── Cold start ──────────────────────────────────────────────────────────
    cold_learner = TD0Learner(task=task_cold, alpha=alpha, use_ewc=use_ewc)
    cold_learner.run(test_budget)

    # ── Warm start ──────────────────────────────────────────────────────────
    warm_learner = TD0Learner(task=task_warm, alpha=alpha, use_ewc=use_ewc)

    if warm_steps > 0:
        # Phase 1: accumulate knowledge on prior-session task
        task_prior = RealKernelTask(rng=random.Random(seed))
        prior_learner = TD0Learner(task=task_prior, alpha=alpha, use_ewc=use_ewc)
        prior_learner.run(warm_steps)

        # Transfer: copy Q-values + visit counts (EWC uses visits for consolidation)
        for src, dst in zip(prior_learner._entries, warm_learner._entries):
            dst.q_value = src.q_value
            dst.visits = src.visits

    warm_learner.run(test_budget)

    # ── Metrics ─────────────────────────────────────────────────────────────
    cold_err = cold_learner.mean_error()
    warm_err = warm_learner.mean_error()

    # Avoid division by zero (both converged to near-zero error)
    if cold_err < 1e-9:
        ratio = 1.0
    else:
        ratio = cold_err / max(warm_err, 1e-9)

    retention = 1.0 - (warm_err / max(cold_err, 1e-9))

    return WarmStartResult(
        warm_steps=warm_steps,
        alpha=alpha,
        use_ewc=use_ewc,
        seed=seed,
        test_budget=test_budget,
        cold_final_error=cold_err,
        warm_final_error=warm_err,
        amplification_ratio=ratio,
        knowledge_retention=retention,
        cold_convergence_step=cold_learner.convergence_step,
        warm_convergence_step=warm_learner.convergence_step,
        duration_ms=(time.perf_counter() - t0) * 1000,
    )


# ---------------------------------------------------------------------------
# AmplificationBenchmark
# ---------------------------------------------------------------------------

class AmplificationBenchmark:
    """
    Benchmark measuring the amplification effect of accumulated knowledge.

    Runs warm vs cold comparison across:
      - warm_levels: depth of prior session knowledge
      - phi (α=0.038, EWC=True) vs standard (α=0.1, EWC=False)
      - n_seeds: statistical robustness

    Key outputs:
      phi_amplification_curve:   [ratio at each warm_level] for phi config
      std_amplification_curve:   [ratio at each warm_level] for standard config
      phi_beats_standard:        bool — phi amplifies more than standard overall
      warm_advantage_grows:      bool — amplification increases with more warm steps
    """

    def run_level(
        self,
        alpha: float,
        use_ewc: bool,
        warm_steps: int,
        test_budget: int = _TEST_BUDGET,
        n_seeds: int = 7,
        base_seed: int = 42,
    ) -> dict:
        """Run n_seeds comparisons at one warm_level, aggregate results."""
        results = [
            _run_warm_cold(
                alpha=alpha,
                use_ewc=use_ewc,
                warm_steps=warm_steps,
                test_budget=test_budget,
                seed=base_seed + i,
            )
            for i in range(n_seeds)
        ]
        ratios = [r.amplification_ratio for r in results]
        retentions = [r.knowledge_retention for r in results]
        return {
            "warm_steps": warm_steps,
            "alpha": alpha,
            "use_ewc": use_ewc,
            "n_seeds": n_seeds,
            "mean_amplification_ratio": round(sum(ratios) / len(ratios), 4),
            "mean_knowledge_retention": round(sum(retentions) / len(retentions), 4),
            "mean_cold_error": round(
                sum(r.cold_final_error for r in results) / len(results), 4
            ),
            "mean_warm_error": round(
                sum(r.warm_final_error for r in results) / len(results), 4
            ),
            "amplified_rate": sum(1 for r in results if r.amplified) / len(results),
        }

    def run_grid(
        self,
        warm_levels: Optional[list[int]] = None,
        test_budget: int = _TEST_BUDGET,
        n_seeds: int = 7,
        base_seed: int = 42,
    ) -> dict:
        """
        Full grid: phi vs standard across all warm_levels.

        Returns:
          phi_curve:            list of mean_amplification_ratio per warm_level (phi)
          standard_curve:       list of mean_amplification_ratio per warm_level (standard)
          phi_beats_standard:   bool — phi mean ratio > standard mean ratio
          warm_advantage_grows: bool — ratio at warm=500 > ratio at warm=0 (for phi)
          phi_amplification_at_500: float — ratio at highest warm level
        """
        if warm_levels is None:
            warm_levels = _WARM_LEVELS

        phi_levels = []
        std_levels = []

        for w in warm_levels:
            phi_levels.append(
                self.run_level(
                    alpha=_PHI_ALPHA, use_ewc=True,
                    warm_steps=w, test_budget=test_budget,
                    n_seeds=n_seeds, base_seed=base_seed,
                )
            )
            std_levels.append(
                self.run_level(
                    alpha=_STD_ALPHA, use_ewc=False,
                    warm_steps=w, test_budget=test_budget,
                    n_seeds=n_seeds, base_seed=base_seed,
                )
            )

        phi_ratios = [lvl["mean_amplification_ratio"] for lvl in phi_levels]
        std_ratios = [lvl["mean_amplification_ratio"] for lvl in std_levels]

        phi_mean = sum(phi_ratios) / len(phi_ratios)
        std_mean = sum(std_ratios) / len(std_ratios)

        # phi beats standard: higher mean amplification across all warm levels
        phi_beats_standard = phi_mean > std_mean

        # Amplification grows with warm depth (for phi):
        # ratio at max warm > ratio at cold (warm=0)
        warm_advantage_grows = phi_ratios[-1] > phi_ratios[0]

        return {
            "warm_levels": warm_levels,
            "test_budget": test_budget,
            "n_seeds": n_seeds,
            # Per-level results
            "phi_levels": phi_levels,
            "standard_levels": std_levels,
            # Amplification curves
            "phi_curve": phi_ratios,
            "standard_curve": std_ratios,
            # Summary stats
            "phi_mean_ratio": round(phi_mean, 4),
            "standard_mean_ratio": round(std_mean, 4),
            # Hypothesis tests
            "phi_beats_standard": phi_beats_standard,
            "warm_advantage_grows": warm_advantage_grows,
            "phi_amplification_at_max_warm": round(phi_ratios[-1], 4),
            "std_amplification_at_max_warm": round(std_ratios[-1], 4),
        }
