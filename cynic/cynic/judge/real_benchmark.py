"""
CYNIC Real Kernel Benchmark (Experiment #1)

Validates phi-derived hyperparameters on REAL CYNIC probe data
(not synthetic — empirically measured from the running kernel).

Empirical true rewards measured by measure_probe_variance.py (200 runs, heuristic mode):
  P1: clean_code      → Q=64.18 → true_reward=0.6418
  P2: smelly_code     → Q=66.39 → true_reward=0.6639
  P3: dangerous_act   → Q= 0.00 → true_reward=0.0000  ← Guardian hard-block
  P4: cynic_self      → Q=67.51 → true_reward=0.6751
  P5: solana_tx       → Q=67.39 → true_reward=0.6739

Key differences from Experiment #0 (synthetic):
  n_pairs: 5   (vs 13 synthetic)
  sigma:   0.012  (vs 0.1 synthetic) — near-deterministic heuristic mode
  landscape: bimodal {0.0, ~0.66} — P3 hard-anchored at zero by Guardian reflex

Hypothesis (same as Experiment #0, now on real data):
  phi (alpha=0.038, EWC=True) converges to real probe Q-values AND
  resists catastrophic forgetting better than standard RL (alpha=0.1, EWC=False).

Key test: P3 at 0.0 is the sharpest contrast point.
  After consolidation, a shock on P3 (toward 1.0) tests EWC most dramatically.
  EWC should resist this reversal — the kernel "knows" P3 is always dangerous.

Reuses from qtable_benchmark:
  QEntry, TD0Learner, ConvergenceResult, _PHI_ALPHA, _STD_ALPHA, _ALPHA_GRID,
  _CONVERGENCE_EPS, _EWC_CONSOLIDATE_AT
"""
from __future__ import annotations

import random
import time
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

from cynic.core.phi import fibonacci
from cynic.judge.qtable_benchmark import (
    QEntry,
    TD0Learner,
    ConvergenceResult,
    _PHI_ALPHA,
    _STD_ALPHA,
    _ALPHA_GRID,
    _CONVERGENCE_EPS,
    _EWC_CONSOLIDATE_AT,
    _N_SHOCK,
)


# ---------------------------------------------------------------------------
# Empirical constants (measured: measure_probe_variance.py, 200 runs)
# ---------------------------------------------------------------------------

# Raw Q-scores (scale: 0–100) — mean over 200 heuristic runs
_EMPIRICAL_Q_SCORES: list[float] = [64.18, 66.39, 0.00, 67.51, 67.39]

# Empirical sigma (mean per-probe std dev) — near-deterministic in heuristic mode
_EMPIRICAL_SIGMA_Q: float = 1.206   # raw Q-score units
_Q_SCALE: float = 100.0             # normalize to [0,1]

# Normalized true_rewards (used by RealKernelTask)
_EMPIRICAL_TRUE_REWARDS: list[float] = [q / _Q_SCALE for q in _EMPIRICAL_Q_SCORES]
_SIGMA_REAL: float = _EMPIRICAL_SIGMA_Q / _Q_SCALE   # ~0.012

# Probe labels — (state_key, action) pairs matching canonical probes
_REAL_PAIRS: list[tuple[str, str]] = [
    ("P1", "clean_code"),
    ("P2", "smelly_code"),
    ("P3", "dangerous_act"),
    ("P4", "cynic_self_state"),
    ("P5", "solana_tx"),
]

# n_pairs = 5 = fibonacci(5) — φ-architecture: 5 canonical probes
_N_PAIRS_REAL: int = len(_EMPIRICAL_TRUE_REWARDS)   # 5


# ---------------------------------------------------------------------------
# RealKernelTask: empirical reward landscape from CYNIC probe runs
# ---------------------------------------------------------------------------

@dataclass
class RealKernelTask:
    """
    Real CYNIC probe landscape.

    5 (state, action) pairs with empirically measured true_rewards.
    Samples add Gaussian noise sigma=_SIGMA_REAL (near-deterministic).

    Landscape is bimodal:
      - P3 (dangerous_act) = 0.00  → Guardian hard-block, always 0
      - P1,P2,P4,P5         ≈ 0.64–0.68 → above phi-threshold (0.618)

    Interface-compatible with SyntheticTask — plugs into TD0Learner as-is.
    """
    n_pairs: int = _N_PAIRS_REAL
    sigma: float = _SIGMA_REAL
    rng: random.Random = field(default_factory=random.Random)

    def __post_init__(self) -> None:
        self._true_rewards: list[float] = list(_EMPIRICAL_TRUE_REWARDS)
        self._pair_labels: list[tuple[str, str]] = list(_REAL_PAIRS)

    def true_reward(self, idx: int) -> float:
        return self._true_rewards[idx]

    def sample(self, idx: int) -> float:
        """Noisy reward sample for pair idx (sigma ≈ 0.012 — near-deterministic)."""
        r = self._true_rewards[idx]
        noisy = r + self.rng.gauss(0, self.sigma)
        return max(0.0, min(1.0, noisy))

    def pair(self, idx: int) -> tuple[str, str]:
        return self._pair_labels[idx]

    @property
    def dangerous_probe_idx(self) -> int:
        """Index of P3 (dangerous_act, always 0.0) — primary shock target."""
        return 2


# ---------------------------------------------------------------------------
# RealBenchmark: grid search on real CYNIC probe data
# ---------------------------------------------------------------------------

class RealBenchmark:
    """
    Benchmark comparing phi-derived (alpha=0.038, EWC=True) vs standard RL
    (alpha=0.1, EWC=False) on REAL CYNIC probe data.

    Same structure as QTableBenchmark (Experiment #0) but uses:
      - RealKernelTask (5 empirical probes) instead of SyntheticTask (13 synthetic)
      - sigma=0.012 (near-deterministic) instead of 0.1 (noisy)

    Key additional metric:
      dangerous_probe_protection: how well EWC preserves P3=0.0 after shock
        (P3 is the strongest anchor — Guardian always scores it 0)
    """

    def __init__(self, task: RealKernelTask | None = None) -> None:
        self.task = task or RealKernelTask()

    def run(
        self,
        alpha: float,
        use_ewc: bool = False,
        max_steps: int = 500,
        seed: int = 42,
    ) -> ConvergenceResult:
        """Run one (alpha, use_ewc) configuration on real probe data."""
        t0 = time.perf_counter()

        rng = random.Random(seed)
        task = RealKernelTask(
            n_pairs=self.task.n_pairs,
            sigma=self.task.sigma,
            rng=rng,
        )
        learner = TD0Learner(task=task, alpha=alpha, use_ewc=use_ewc)
        learner.run(max_steps)

        forgetting = learner.inject_shock()
        duration_ms = (time.perf_counter() - t0) * 1000

        return ConvergenceResult(
            alpha=alpha,
            use_ewc=use_ewc,
            seed=seed,
            max_steps=max_steps,
            convergence_step=learner.convergence_step,
            final_mean_error=learner.mean_error(),
            final_max_error=learner.max_error(),
            q_variance=learner.q_variance(),
            forgetting_shift=forgetting,
            duration_ms=duration_ms,
        )

    def run_grid(
        self,
        max_steps: int = 500,
        n_seeds: int = 7,
        base_seed: int = 42,
    ) -> dict:
        """
        Run full alpha x EWC grid on real probe data, aggregate over n_seeds.

        Returns same structure as QTableBenchmark.run_grid() for direct comparison.
        Additional keys:
          dangerous_probe_shift_phi:     mean Q-shift on P3 under shock (phi config)
          dangerous_probe_shift_standard: same for standard config
          phi_protects_dangerous:        bool — phi resists P3 reversal better
        """
        configs: list[tuple[float, bool]] = [
            (a, ewc)
            for a in _ALPHA_GRID
            for ewc in (True, False)
        ]

        all_results: dict[tuple[float, bool], list[ConvergenceResult]] = {}
        for cfg in configs:
            alpha, use_ewc = cfg
            runs = [
                self.run(
                    alpha=alpha,
                    use_ewc=use_ewc,
                    max_steps=max_steps,
                    seed=base_seed + i,
                )
                for i in range(n_seeds)
            ]
            all_results[cfg] = runs

        def agg(runs: list[ConvergenceResult]) -> dict:
            conv_steps = [r.convergence_step or max_steps for r in runs]
            return {
                "label": runs[0].label,
                "alpha": runs[0].alpha,
                "use_ewc": runs[0].use_ewc,
                "converged_rate": sum(r.converged for r in runs) / len(runs),
                "mean_convergence_step": round(sum(conv_steps) / len(conv_steps), 1),
                "mean_final_error": round(sum(r.final_mean_error for r in runs) / len(runs), 4),
                "mean_forgetting_shift": round(sum(r.forgetting_shift for r in runs) / len(runs), 4),
                "mean_q_variance": round(sum(r.q_variance for r in runs) / len(runs), 5),
            }

        configs_agg = [agg(all_results[cfg]) for cfg in configs]

        phi_key = (_PHI_ALPHA, True)
        std_key = (_STD_ALPHA, False)
        phi_agg = agg(all_results[phi_key])
        std_agg = agg(all_results[std_key])

        phi_wins_forgetting = (
            phi_agg["mean_forgetting_shift"] <= std_agg["mean_forgetting_shift"]
        )
        phi_wins_stability = (
            phi_agg["mean_q_variance"] <= std_agg["mean_q_variance"]
        )
        phi_wins_convergence = (
            phi_agg["mean_final_error"] <= std_agg["mean_final_error"] * 3.0
        )

        # Dangerous probe: measure P3 shift specifically under shock
        # (P3 at 0.0 is the hardest anchor — any shock toward 1.0 is a full reversal)
        phi_p3_shifts = [
            self._measure_p3_shift(alpha=_PHI_ALPHA, use_ewc=True,
                                   max_steps=max_steps, seed=base_seed + i)
            for i in range(n_seeds)
        ]
        std_p3_shifts = [
            self._measure_p3_shift(alpha=_STD_ALPHA, use_ewc=False,
                                   max_steps=max_steps, seed=base_seed + i)
            for i in range(n_seeds)
        ]
        mean_phi_p3 = sum(phi_p3_shifts) / len(phi_p3_shifts)
        mean_std_p3 = sum(std_p3_shifts) / len(std_p3_shifts)

        return {
            "n_seeds": n_seeds,
            "max_steps": max_steps,
            "configs": configs_agg,
            "phi": phi_agg,
            "standard": std_agg,
            "phi_wins_forgetting": phi_wins_forgetting,
            "phi_wins_stability": phi_wins_stability,
            "phi_wins_convergence": phi_wins_convergence,
            "phi_overall_win": phi_wins_forgetting and phi_wins_stability,
            # Dangerous probe specific
            "dangerous_probe_shift_phi": round(mean_phi_p3, 4),
            "dangerous_probe_shift_standard": round(mean_std_p3, 4),
            "phi_protects_dangerous": mean_phi_p3 <= mean_std_p3,
        }

    def _measure_p3_shift(
        self,
        alpha: float,
        use_ewc: bool,
        max_steps: int,
        seed: int,
    ) -> float:
        """
        Measure Q-shift on P3 (dangerous_act, true_reward=0.0) under a shock.

        After max_steps of learning, inject a single shock on P3 (toward 1.0).
        Returns |Q_after - Q_before| for P3 specifically.
        """
        rng = random.Random(seed)
        task = RealKernelTask(rng=rng)
        learner = TD0Learner(task=task, alpha=alpha, use_ewc=use_ewc)
        learner.run(max_steps)

        p3_idx = task.dangerous_probe_idx
        # Access internal entries directly for P3-specific measurement
        before = learner._entries[p3_idx].q_value
        shock_reward = 1.0 - task.true_reward(p3_idx)  # full reversal: 1.0 - 0.0 = 1.0
        learner._entries[p3_idx].update(shock_reward, alpha, use_ewc)
        after = learner._entries[p3_idx].q_value
        return abs(after - before)
