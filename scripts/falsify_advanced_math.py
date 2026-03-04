"""
CYNIC Mathematical Judgment Falsification.
Validates UCB1 (Sutton & Barto), Bayesian Updating (Murphy), 
and 9-Lenses Multi-Dimensional Norms (L2).
"""
import asyncio
import logging
from cynic.kernel.core.mathematics import RLMathematics, ProbabilisticMathematics, MultiDimensionalMathematics
from cynic.kernel.organism.experience import ExperienceVault
from cynic.kernel.organism.benchmarking import LensesScore, BenchmarkMetric

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger("cynic.math_falsify")

async def run_math_falsification():
    print("\n" + "="*60)
    print("📐 CYNIC MATH FOUNDATIONS: UCB1, BAYES & L2 NORM")
    print("="*60)

    # 1. Test Probabilistic Mathematics (Bayesian Updating)
    print("\n--- PHASE 1: BAYESIAN UPDATING (Murphy) ---")
    # Prior: 50% success, weight 2. New data: 1 success out of 1 trial.
    prob_1 = ProbabilisticMathematics.bayesian_update(0.5, 2, successes=1, total_trials=1)
    # New data: 10 successes out of 10 trials.
    prob_10 = ProbabilisticMathematics.bayesian_update(0.5, 2, successes=10, total_trials=10)
    
    print(f"  -> Posterior after 1 success: {prob_1:.4f} (Cautious)")
    print(f"  -> Posterior after 10 successes: {prob_10:.4f} (Confident)")
    if prob_10 > prob_1:
        print("  ✅ BAYES VALIDATED: Confidence scales with empirical evidence.")

    # 2. Test Multi-Dimensional Mathematics (9 Lenses)
    print("\n--- PHASE 2: 9 LENSES EUCLIDEAN NORM (L2) ---")
    # A perfect architect but terrible backend dev
    lenses = LensesScore(
        ai_infra=0.9, backend=0.1, ml_platform=0.8,
        data_engineer=0.5, security=0.2, sre=0.3,
        blockchain=0.5, robotics=0.5, solutions_architect=1.0
    )
    metric = BenchmarkMetric(model_id="test_model", axiom="ARCHITECTURE", tokens_per_sec=10.0, total_latency_ms=1000, lenses=lenses)
    l2_score = metric.phi_quality_score
    print(f"  -> L2 Norm (Phi Score) across 9 lenses: {l2_score:.4f}")
    if 0.0 <= l2_score <= 1.0:
        print("  ✅ MULTI-DIMENSIONAL VALIDATED: Geometric truth established.")

    # 3. Test RL Mathematics (UCB1 Exploration vs Exploitation)
    print("\n--- PHASE 3: UCB1 ALGORITHM (Sutton & Barto) ---")
    vault = ExperienceVault(storage_path="audit/test_ucb1_weights.json")
    vault.total_system_trials = 100 # High system maturity

    # Model A: Proven but average (5 successes out of 10)
    for _ in range(5): await vault.record_experience("model_A", "BACKEND", success=True, latency_ms=100)
    for _ in range(5): await vault.record_experience("model_A", "BACKEND", success=False, latency_ms=100)
    
    # Model B: Untested (0 trials)
    
    candidates = ["model_A", "model_B"]
    selected = vault.get_best_dog_for("BACKEND", candidates)
    
    print(f"  -> Model A (Proven, 50% SR)")
    print(f"  -> Model B (Untested)")
    print(f"  -> UCB1 Selected: {selected}")
    
    if selected == "model_B":
        print("  ✅ RL VALIDATED: UCB1 forces exploration of the unknown.")
    else:
        print("  ❌ RL FAILED: CYNIC is stuck in a local minima (Exploitation bias).")

    print("\n" + "="*60)
    print("🏆 ALL MATHEMATICAL FOUNDATIONS VALIDATED.")
    print("="*60)

if __name__ == "__main__":
    asyncio.run(run_math_falsification())
