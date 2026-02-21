"""
Mesure la variance réelle des 5 probes canoniques de CYNIC.

Runs evolve() N fois en mode heuristic (sans DB, sans LLM) et
capture par probe : q_score, pass/fail, duration_ms.

Résultats utilisés pour :
  1. Corriger n (pool size) via proportion test (remplace l'hypothèse σ=15)
  2. Valider ou invalider les constantes φ Catégorie B (α, γ, buffer sizes)
  3. Établir la baseline pass_rate pour le BenchmarkRegistry

Usage:
    cd cynic/
    python measure_probe_variance.py
    python measure_probe_variance.py --runs 200
    python measure_probe_variance.py --runs 50 --verbose
"""
from __future__ import annotations

import argparse
import asyncio
import io
import math
import statistics
import sys
import time
from collections import defaultdict
from typing import Any, Dict, List, Tuple

# Force UTF-8 on Windows (cp1252 doesn't support Greek/box chars)
if hasattr(sys.stdout, "buffer"):
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")


# ── Kernel bootstrap minimal (heuristic mode — pas de DB, pas de LLM) ────────

def build_minimal_orchestrator():
    """
    Construit un JudgeOrchestrator avec les 11 dogs en mode heuristic.
    Pas de DB pool, pas de LLMRegistry → tous les dogs font du scoring
    basé sur règles et heuristiques.
    """
    from cynic.core.axioms import AxiomArchitecture
    from cynic.dogs.base import DogId
    from cynic.dogs.cynic_dog import CynicDog
    from cynic.dogs.guardian import GuardianDog
    from cynic.dogs.analyst import AnalystDog
    from cynic.dogs.janitor import JanitorDog
    from cynic.dogs.architect import ArchitectDog
    from cynic.dogs.oracle import OracleDog
    from cynic.dogs.sage import SageDog
    from cynic.dogs.scholar import ScholarDog
    from cynic.dogs.cartographer import CartographerDog
    from cynic.dogs.deployer import DeployerDog
    from cynic.dogs.scout import ScoutDog
    from cynic.judge.orchestrator import JudgeOrchestrator
    from cynic.learning.qlearning import QTable

    cynic_dog = CynicDog()
    qtable = QTable()
    dogs = {
        DogId.CYNIC:        cynic_dog,
        DogId.SAGE:         SageDog(),
        DogId.GUARDIAN:     GuardianDog(),
        DogId.ANALYST:      AnalystDog(),
        DogId.JANITOR:      JanitorDog(),
        DogId.ARCHITECT:    ArchitectDog(),
        DogId.ORACLE:       OracleDog(qtable=qtable),
        DogId.SCHOLAR:      ScholarDog(),
        DogId.CARTOGRAPHER: CartographerDog(),
        DogId.DEPLOYER:     DeployerDog(),
        DogId.SCOUT:        ScoutDog(),
    }
    return JudgeOrchestrator(
        dogs=dogs,
        axiom_arch=AxiomArchitecture(),
        cynic_dog=cynic_dog,
        residual_detector=None,  # pas nécessaire pour la mesure
    )


# ── Statistiques ───────────────────────────────────────────────────────────────

def compute_stats(values: List[float]) -> Dict[str, float]:
    if len(values) < 2:
        return {"n": len(values), "mean": values[0] if values else 0.0,
                "std": 0.0, "min": values[0] if values else 0.0,
                "p50": values[0] if values else 0.0,
                "p95": values[0] if values else 0.0,
                "max": values[0] if values else 0.0}
    n = len(values)
    sorted_v = sorted(values)
    return {
        "n": n,
        "mean": statistics.mean(values),
        "std": statistics.stdev(values),
        "min": sorted_v[0],
        "p50": sorted_v[n // 2],
        "p95": sorted_v[int(n * 0.95)],
        "max": sorted_v[-1],
    }


def proportion_test_n(p0: float, p1: float,
                      alpha: float = 0.05, power: float = 0.80) -> int:
    """
    Taille d'échantillon nécessaire pour détecter un drop de pass_rate
    de p0 vers p1 (test one-sided, proportion).

    Formule :
        n = (z_α√p₀(1-p₀) + z_β√p₁(1-p₁))² / (p₀ - p₁)²
    """
    from scipy.stats import norm
    z_a = norm.ppf(1 - alpha)   # 1.645 pour α=0.05
    z_b = norm.ppf(power)       # 0.842 pour β=0.80
    num = (z_a * math.sqrt(p0 * (1 - p0)) + z_b * math.sqrt(p1 * (1 - p1))) ** 2
    denom = (p0 - p1) ** 2
    return math.ceil(num / denom)


# ── Run loop ───────────────────────────────────────────────────────────────────

async def run_measurement(n_runs: int, verbose: bool) -> Dict[str, Any]:
    orchestrator = build_minimal_orchestrator()

    # per_probe[probe_name] = {q_scores, passed, durations_ms}
    per_probe: Dict[str, Dict[str, List]] = defaultdict(
        lambda: {"q_scores": [], "passed": [], "durations_ms": []}
    )

    t_start = time.time()
    print(f"\nRunning evolve() × {n_runs}  [heuristic mode — no LLM, no DB]\n")

    for i in range(n_runs):
        summary = await orchestrator.evolve()

        for r in summary["results"]:
            name = r["name"]
            per_probe[name]["q_scores"].append(r["q_score"])
            per_probe[name]["passed"].append(r["passed"])
            per_probe[name]["durations_ms"].append(r["duration_ms"])

            if verbose:
                status = "✓" if r["passed"] else "✗"
                print(f"  run {i+1:3d}  {name:<25} Q={r['q_score']:5.1f}  {status}")

        # Progress every 10 runs (non-verbose)
        if not verbose and (i + 1) % 10 == 0:
            elapsed = time.time() - t_start
            rate = (i + 1) / elapsed
            eta = (n_runs - i - 1) / rate
            filled = (i + 1) * 20 // n_runs
            bar = "#" * filled + "-" * (20 - filled)
            print(f"  [{bar}] {i+1:3d}/{n_runs}  {rate:.1f} runs/s  ETA {eta:.0f}s")

    total_elapsed = time.time() - t_start
    print(f"\n  Done: {total_elapsed:.1f}s total  ({n_runs / total_elapsed:.1f} runs/s)\n")
    return dict(per_probe)


# ── Report ────────────────────────────────────────────────────────────────────

def print_report(per_probe: Dict[str, Any], n_runs: int) -> None:
    SEP = "-" * 72

    print(SEP)
    print("  CYNIC PROBE VARIANCE REPORT")
    print(SEP)

    all_stds: List[float] = []
    all_pass_rates: List[float] = []
    all_q_scores: List[float] = []

    # ── Per-probe table ──────────────────────────────────────────────────────
    for probe_name, data in sorted(per_probe.items()):
        q = compute_stats(data["q_scores"])
        d = compute_stats(data["durations_ms"])
        pass_rate = sum(data["passed"]) / len(data["passed"])

        all_stds.append(q["std"])
        all_pass_rates.append(pass_rate)
        all_q_scores.extend(data["q_scores"])

        verdict = (
            "[STABLE]" if pass_rate >= 0.90 else
            "[ WARN ]" if pass_rate >= 0.70 else
            "[FLAKY ]"
        )

        print(f"\n  {probe_name}")
        print(f"    Q-Score : mean={q['mean']:5.2f}  σ={q['std']:5.2f}  "
              f"range=[{q['min']:.1f}, {q['max']:.1f}]  p95={q['p95']:.1f}")
        print(f"    Pass    : {pass_rate*100:5.1f}%  {verdict}")
        print(f"    Latency : p50={d['p50']:.0f}ms  p95={d['p95']:.0f}ms  "
              f"max={d['max']:.0f}ms")

    # ── Global statistics ────────────────────────────────────────────────────
    global_std = statistics.mean(all_stds) if all_stds else 0.0
    global_pass_rate = statistics.mean(all_pass_rates) if all_pass_rates else 0.0
    global_std_pooled = statistics.stdev(all_q_scores) if len(all_q_scores) > 1 else 0.0

    print(f"\n{SEP}")
    print("  GLOBAL STATISTICS")
    print(SEP)
    print(f"    Runs per probe         : {n_runs}")
    print(f"    Mean σ per probe       : {global_std:.3f}  (φ assumption was 15.0)")
    print(f"    Pooled σ (all scores)  : {global_std_pooled:.3f}")
    print(f"    Overall pass rate      : {global_pass_rate*100:.1f}%")

    # ── Sample size calculation ──────────────────────────────────────────────
    p0 = max(global_pass_rate, 0.01)
    p1 = max(p0 - 0.15, 0.01)   # detect 15% absolute drop

    print(f"\n{SEP}")
    print("  SAMPLE SIZE  (proportion test, one-sided)")
    print(SEP)
    print(f"    Baseline pass rate  p₀  : {p0:.0%}")
    print(f"    Detect drop to      p₁  : {p1:.0%}  (Δ=-15%)")

    try:
        n_req = proportion_test_n(p0, p1, alpha=0.05, power=0.80)
        n_rec = n_req + 5  # margin
        print(f"    Required (α=5%, β=80%) : {n_req}")
        print(f"    Recommended pool size  : {n_rec}  (+5 margin)")
        print(f"\n    Also check: detect Δ=-10%")
        p1_tight = max(p0 - 0.10, 0.01)
        n_tight = proportion_test_n(p0, p1_tight, alpha=0.05, power=0.80)
        print(f"    Required (Δ=-10%)      : {n_tight}  (+5 → {n_tight+5})")
    except ValidationError as e:
        print(f"    [scipy error: {e}]")

    # ── φ validation ─────────────────────────────────────────────────────────
    print(f"\n{SEP}")
    print("  φ-VALIDATION  (Catégorie B constants)")
    print(SEP)

    sigma_assumed = 15.0
    ratio = global_std / sigma_assumed if sigma_assumed > 0 else 0.0

    print(f"    σ assumed (our calc)   : {sigma_assumed:.1f}")
    print(f"    σ measured (per-probe) : {global_std:.3f}")
    print(f"    Ratio measured/assumed : {ratio:.3f}×")

    if abs(ratio - 1.0) < 0.20:
        verdict_phi = "ACCURATE (within 20%)"
    elif ratio < 1.0:
        verdict_phi = f"OVERESTIMATED - real sigma {(1-ratio)*100:.0f}% smaller -> fewer probes needed"
    else:
        verdict_phi = f"UNDERESTIMATED - real sigma {(ratio-1)*100:.0f}% larger -> more probes needed"

    print(f"    Assessment             : {verdict_phi}")

    # Determinism check — are scores deterministic or stochastic?
    print(f"\n    DETERMINISM CHECK:")
    for probe_name, data in sorted(per_probe.items()):
        unique_scores = len(set(round(q, 1) for q in data["q_scores"]))
        pct_unique = unique_scores / len(data["q_scores"]) * 100
        nature = "deterministic" if pct_unique < 5 else "stochastic"
        print(f"      {probe_name:<30} {unique_scores:3d} unique Q-scores / {n_runs}  → {nature}")

    print(f"\n{SEP}")
    print("  NEXT ACTIONS")
    print(SEP)
    print("  1. Pool size : see 'Recommended pool size' above")
    print("  2. If all probes are DETERMINISTIC → σ is near 0 → n can be much smaller")
    print("     (binary pass/fail proportion test still applies)")
    print("  3. Copy these baselines into BenchmarkRegistry as initial snapshot")
    print("  4. Proceed to Experiment #0 (α,γ grid search)")
    print(SEP)


# ── Entry point ───────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Measure CYNIC probe variance — determines correct pool size n"
    )
    parser.add_argument(
        "--runs", type=int, default=100,
        help="Number of evolve() runs (default: 100)"
    )
    parser.add_argument(
        "--verbose", action="store_true",
        help="Print each individual probe result"
    )
    args = parser.parse_args()

    per_probe = asyncio.run(run_measurement(args.runs, args.verbose))
    print_report(per_probe, args.runs)


if __name__ == "__main__":
    main()
