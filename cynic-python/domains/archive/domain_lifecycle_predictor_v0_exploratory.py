#!/usr/bin/env python3
"""
Domain Lifecycle Predictor — Organism-driven domain routing.

Predicts which domain (D1-D6) will be most productive for organ-x's next searches.
Trained on organ-x's actual farming history, observations, and behavioral patterns.

Ground truth: organ-x's productivity, not verdict classifications.
"""

import json
import os
from pathlib import Path
from collections import defaultdict
from datetime import datetime
import numpy as np
from typing import Dict, List, Tuple

# ============================================================================
# 1. LOAD ORGAN-X DATA
# ============================================================================

organ_x_dir = Path.home() / ".cynic/organs/hermes/x"

def load_farming_log() -> List[Dict]:
    """Load farming_log.jsonl — domain farming decisions per cycle."""
    farming = []
    with open(organ_x_dir / "farming_log.jsonl") as f:
        for line in f:
            farming.append(json.loads(line))
    return farming

def load_farming_metrics() -> Dict:
    """Load current farming metrics."""
    with open(organ_x_dir / "farming_metrics.json") as f:
        return json.load(f)

def load_observations() -> List[Dict]:
    """Load observations — discovered high-signal patterns."""
    obs = []
    obs_dir = organ_x_dir / "observations"
    for f in sorted(obs_dir.glob("*.json")):
        try:
            with open(f) as fp:
                data = json.load(fp)
                if data.get("type") == "observation":
                    obs.append(data)
        except:
            pass
    return obs

def load_reflections() -> List[Dict]:
    """Load reflections — learnings per cycle."""
    reflections = []
    datasets_dir = organ_x_dir / "datasets"
    if (datasets_dir / "reflections.jsonl").exists():
        with open(datasets_dir / "reflections.jsonl") as f:
            for line in f:
                reflections.append(json.loads(line))
    return reflections

def load_behavioral_profile() -> Dict:
    """Load behavioral profile — organ-x's interaction patterns."""
    with open(organ_x_dir / "behavioral_profile.json") as f:
        return json.load(f)

# ============================================================================
# 2. EXTRACT SIGNALS
# ============================================================================

def extract_farming_history(farming_log: List[Dict]) -> Dict[str, int]:
    """Count searches per domain across all cycles."""
    domain_count = defaultdict(int)
    for cycle in farming_log:
        domains = cycle.get("domains_farmed", [])
        for domain in domains:
            domain_count[domain] += 1
    return dict(domain_count)

def extract_observation_outcomes(observations: List[Dict]) -> Dict[str, List[int]]:
    """Map observations to domains and signal scores."""
    outcomes = defaultdict(list)
    for obs in observations:
        # Read domain label directly from observation
        domain = obs.get("domain", "unknown").upper()
        # Normalize twitter domain to match D labels
        if domain == "TWITTER":
            domain = "twitter"  # Keep as-is, not a D1-D6 domain
        signal = obs.get("signal_score", 0)
        outcomes[domain].append(signal)
    return dict(outcomes)

def extract_behavioral_features(profile: Dict) -> Dict:
    """Extract key behavioral signals."""
    features = {
        "typing_wpm": profile.get("typing", {}).get("wpm", 0),
        "scroll_up_pct": profile.get("scroll", {}).get("scroll_up_percent", 0),
        "peak_hours": profile.get("temporal", {}).get("peak_hours", []),
        "activity_span": profile.get("temporal", {}).get("activity_end_hour", 0) -
                         profile.get("temporal", {}).get("activity_start_hour", 0),
    }
    return features

def extract_reflection_insights(reflections: List[Dict]) -> Dict[str, int]:
    """
    Extract reflection-level insights per cycle.

    Reflections are system-level feedback (kernel status, turn counts, etc),
    not domain-specific. Return per-cycle data that can be aggregated.
    """
    # Count cycles with healthy vs unhealthy state
    # This indicates when organ-x was productive
    healthy_cycles = 0
    for reflection in reflections:
        diagnosis = reflection.get("diagnosis", {})
        if diagnosis.get("is_healthy", False):
            healthy_cycles += 1
    return {"healthy_cycles": healthy_cycles, "total_cycles": len(reflections)}

# ============================================================================
# 3. BUILD TRAINING DATA
# ============================================================================

def build_training_set(
    farming_history: Dict[str, int],
    observation_outcomes: Dict[str, List[int]],
    behavioral_features: Dict,
    reflection_insights: Dict[str, int]
) -> Tuple[np.ndarray, np.ndarray]:
    """
    Build (X, y) training data.

    X: [farming_count, avg_signal, observation_count, behavioral_alignment]
    y: domain ranking (higher = more productive)

    Key insight: Observations are labeled with SEMANTIC domains (twitter),
    not search domains (D1-D6). This reveals the domain-routing gap.
    """

    domains = ["D1", "D2", "D3", "D4", "D5", "D6"]
    X_list = []
    y_list = []

    # Check if twitter domain has observations (it does)
    twitter_signals = observation_outcomes.get("twitter", [])

    for domain in domains:
        # Feature 1: farming count (how many times did we search this domain?)
        farming_count = farming_history.get(domain, 0)

        # Feature 2: avg signal from observations IN THIS DOMAIN
        # D1-D6 searches yield twitter observations, so signal is indirect
        obs_signals = observation_outcomes.get(domain, [])
        avg_signal = np.mean(obs_signals) if obs_signals else 0

        # Feature 3: observation count (how many discoveries per domain?)
        obs_count = len(obs_signals)

        # Feature 4: behavioral alignment
        # 65.8% scroll-up = researcher reading mode → narrative-heavy domains
        if behavioral_features["scroll_up_pct"] > 60:
            behavioral_score = 1.0 if domain in ["D1"] else 0.7
        else:
            behavioral_score = 0.8

        # Label: productivity = signal quality per search + farming momentum
        # twitter observations (signal_score=7) are what matter
        if domain == "D1" and twitter_signals:
            # D1 farming yielded twitter observations
            productivity = np.mean(twitter_signals) + (farming_count / 100.0)
        else:
            # D2-D6 farming yielded no labeled observations yet
            productivity = (farming_count / 100.0)

        X_list.append([farming_count, avg_signal, obs_count, behavioral_score])
        y_list.append(productivity)

    return np.array(X_list), np.array(y_list)

# ============================================================================
# 4. TRAIN & EVALUATE
# ============================================================================

def train_predictor(X: np.ndarray, y: np.ndarray) -> Tuple[np.ndarray, float]:
    """
    Simple linear regression using numpy: predict domain productivity.

    Returns: (coefficients, r2_score, intercept)
    """
    # Add intercept column
    X_with_intercept = np.column_stack([np.ones(X.shape[0]), X])

    # Normal equation: coef = (X^T X)^-1 X^T y
    try:
        coef = np.linalg.lstsq(X_with_intercept, y, rcond=None)[0]
    except np.linalg.LinAlgError:
        # Fallback for singular matrix
        coef = np.zeros(X.shape[1] + 1)

    intercept = coef[0]
    weights = coef[1:]

    # R² score
    y_pred = X @ weights + intercept
    ss_res = np.sum((y - y_pred) ** 2)
    ss_tot = np.sum((y - np.mean(y)) ** 2)
    r2 = 1 - (ss_res / ss_tot) if ss_tot > 0 else 0

    return weights, r2, intercept

def rank_domains_by_productivity(
    coef: np.ndarray,
    intercept: float,
    X: np.ndarray,
    domains: List[str]
) -> List[Tuple[str, float]]:
    """Rank domains by predicted productivity."""
    scores = X @ coef + intercept
    ranked = sorted(zip(domains, scores), key=lambda x: -x[1])
    return ranked

# ============================================================================
# 5. MAIN
# ============================================================================

def main():
    print("[1/5] Loading organ-x data...")
    farming_log = load_farming_log()
    farming_metrics = load_farming_metrics()
    observations = load_observations()
    reflections = load_reflections()
    behavioral_profile = load_behavioral_profile()

    print(f"  Loaded: {len(farming_log)} farming cycles, {len(observations)} observations, {len(reflections)} reflections")

    print("\n[2/5] Extracting signals...")
    farming_history = extract_farming_history(farming_log)
    observation_outcomes = extract_observation_outcomes(observations)
    behavioral_features = extract_behavioral_features(behavioral_profile)
    reflection_insights = extract_reflection_insights(reflections)

    print(f"  Farming per domain: {farming_history}")
    print(f"  Observations per domain: {len(observation_outcomes)} domains found")
    print(f"  Behavioral: {behavioral_features['scroll_up_pct']:.1f}% scroll-up (researcher mode)")
    print(f"  Reflection mentions: {reflection_insights}")

    print("\n[3/5] Building training set...")
    X, y = build_training_set(farming_history, observation_outcomes, behavioral_features, reflection_insights)
    domains = ["D1", "D2", "D3", "D4", "D5", "D6"]

    print(f"  X shape: {X.shape}")
    print(f"  y shape: {y.shape}")
    print(f"  Domain productivities: {list(zip(domains, y))}")

    print("\n[4/5] Training predictor...")
    coef, r2, intercept = train_predictor(X, y)
    print(f"  R² score: {r2:.3f}")
    print(f"  Coefficients: {dict(zip(['farming_count', 'avg_signal', 'reflection_count', 'behavioral_score'], coef))}")
    print(f"  Intercept: {intercept:.3f}")

    print("\n[5/5] Domain ranking (predicted productivity)...")
    ranked = rank_domains_by_productivity(coef, intercept, X, domains)
    for domain, score in ranked:
        print(f"  {domain}: {score:.3f}")

    # ========================================================================
    # FALSIFICATION TEST
    # ========================================================================
    print("\n[FALSIFICATION TEST]")
    print("Does predictor match observed farming patterns?")

    farming_distribution = {d: farming_history.get(d, 0) for d in domains}
    max_farming = max(farming_distribution.values()) if farming_distribution else 1
    farming_normalized = {d: farming_distribution[d] / max_farming for d in domains}

    predictor_distribution = {d: s / max(s for _, s in ranked) for d, s in ranked}

    # Simple correlation: do high-farming domains match high-productivity predictions?
    farming_ranks = sorted(enumerate(farming_normalized.values()), key=lambda x: -x[1])
    predictor_ranks = sorted(enumerate([predictor_distribution[d] for d in domains]), key=lambda x: -x[1])

    farming_order = [domains[idx] for idx, _ in farming_ranks]
    predictor_order = [domains[idx] for idx, _ in predictor_ranks]

    print(f"  Observed farming order: {farming_order}")
    print(f"  Predicted order: {predictor_order}")
    print(f"  Match: {farming_order == predictor_order}")

    # THRESHOLD CHECK
    print(f"\n[CONCLUSION]")
    if r2 > 0.5:
        print(f"  ✓ Predictor captures structure (R² = {r2:.3f})")
    else:
        print(f"  ✗ Predictor weak (R² = {r2:.3f}) — domains may not be differentiable from this data")

    # Save model
    model_output = {
        "timestamp": datetime.now().isoformat(),
        "r2_score": float(r2),
        "coefficients": {
            "farming_count": float(coef[0]),
            "avg_signal": float(coef[1]),
            "reflection_count": float(coef[2]),
            "behavioral_score": float(coef[3]),
        },
        "intercept": float(intercept),
        "domain_rankings": ranked,
        "training_data": {
            "farming_history": farming_history,
            "observation_outcomes": {k: [float(v) for v in vals] for k, vals in observation_outcomes.items()},
            "behavioral_features": behavioral_features,
        }
    }

    output_path = Path("domain_lifecycle_model.json")
    with open(output_path, "w") as f:
        json.dump(model_output, f, indent=2)
    print(f"\n  Model saved to: {output_path}")

if __name__ == "__main__":
    main()
