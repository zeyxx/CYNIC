"""
CYNIC φ Constants — Single Source of Truth

φ = 1.6180339887498948482... (Golden Ratio)

ALL architecture derives from φ via Fibonacci and Lucas sequences:
  - 5 = F(5)  → 5 Core Axioms (always active)
  - 7 = L(4)  → 7 Reality dims, 7 Analysis dims, 7 Time dims, 7 facets per axiom
  - 11 = L(5) → 11 Dogs (Sefirot)
  - 9 = practical axiom count (5 core + 4 measurable emergent)

IMPORT RULE: NEVER define φ constants elsewhere.
Always: `from cynic.core.phi import PHI, PHI_INV, ...`
"""
from __future__ import annotations

import math
from typing import Dict, List, Tuple


# ════════════════════════════════════════════════════════════════════════════
# PRIMARY CONSTANT (15-decimal precision)
# ════════════════════════════════════════════════════════════════════════════

PHI: float = (1 + math.sqrt(5)) / 2
"""Golden Ratio φ = 1.618033988749895"""

PHI_INV: float = PHI - 1                  # = 1/φ = 0.618033988749895
PHI_INV_2: float = 2 - PHI                # = 1/φ² = 0.381966011250105
PHI_INV_3: float = PHI_INV_2 * PHI_INV   # = 1/φ³ = 0.236067977499790

PHI_2: float = PHI * PHI          # φ² = 2.618033988749895
PHI_3: float = PHI_2 * PHI        # φ³ = 4.236067977499790
PHI_4: float = PHI_3 * PHI        # φ⁴ = 6.854101966249685
PHI_5: float = PHI_4 * PHI        # φ⁵ = 11.090169943749474


# ════════════════════════════════════════════════════════════════════════════
# JUDGMENT THRESHOLDS (φ-aligned)
# ════════════════════════════════════════════════════════════════════════════

# Max confidence in any judgment (Law of Doubt — "φ distrusts φ")
MAX_CONFIDENCE: float = PHI_INV          # 0.618 = 61.8%
MAX_Q_SCORE: float = PHI_INV * 100       # 61.8 (Q-Score scale)

# Verdict thresholds
HOWL_THRESHOLD: float = PHI_2 * PHI_INV * 100  # ≈ 61.8 × 1.618 ≈ 100 → use 82
_HOWL_THRESHOLD_EXACT: float = 82.0
WAG_THRESHOLD: float = MAX_Q_SCORE              # 61.8
GROWL_THRESHOLD: float = PHI_INV_2 * 100        # 38.2
BARK_THRESHOLD: float = 0.0

HOWL_MIN: float = 82.0                   # HOWL: 82-100 (φ² × φ⁻¹ → exceptional)
WAG_MIN: float = MAX_Q_SCORE             # WAG:  ≥61.8 (= φ⁻¹ × 100 → good)
GROWL_MIN: float = PHI_INV_2 * 100      # GROWL: ≥38.2 (= φ⁻² × 100 → needs work)
BARK_MAX: float = PHI_INV_2 * 100       # BARK: <38.2 (= φ⁻² × 100 → critical)

# Trust / Emergence thresholds
MIN_DOUBT: float = PHI_INV_2     # 0.382 = minimum doubt (skepticism floor)
DEEP_UNCERTAINTY: float = PHI_INV_3  # 0.236 = deep uncertainty marker

# PBFT (11 Dogs, f=3 Byzantine faults)
DOGS_TOTAL: int = 11         # L(5) = Lucas(5)
DOGS_BYZANTINE: int = 3      # f = (11 - 1) // 3
DOGS_QUORUM: int = 7         # 2f+1 = 7 (minimum for consensus)

# MCTS budget split
MCTS_LEVEL1_RATIO: float = PHI_INV_2   # 38.2% for Dog combination selection
MCTS_LEVEL2_RATIO: float = PHI_INV     # 61.8% for per-Dog action exploration

# Learning rates (φ-aligned)
LEARNING_RATE: float = PHI_INV_2 / 10   # ≈ 0.038 (conservative)
EWC_PENALTY: float = PHI_INV            # λ = 0.618 (forgetting penalty)
THOMPSON_CONFIDENCE: float = PHI_INV    # β distribution confidence bound


# ════════════════════════════════════════════════════════════════════════════
# FIBONACCI SEQUENCE (for timing, intervals, counts)
# ════════════════════════════════════════════════════════════════════════════

def fibonacci(n: int) -> int:
    """
    Compute F(n) — Fibonacci number.

    F(0)=0, F(1)=1, F(2)=1, F(3)=2, F(4)=3, F(5)=5, ...
    F(6)=8, F(7)=13, F(8)=21, F(9)=34, F(10)=55, F(11)=89, F(12)=144, F(13)=233
    """
    if n < 0:
        raise ValueError(f"F({n}) undefined for negative n")
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a


def lucas(n: int) -> int:
    """
    Compute L(n) — Lucas number.

    L(0)=2, L(1)=1, L(2)=3, L(3)=4, L(4)=7, L(5)=11, L(6)=18, ...
    """
    if n < 0:
        raise ValueError(f"L({n}) undefined for negative n")
    if n == 0:
        return 2
    a, b = 2, 1
    for _ in range(n - 1):
        a, b = b, a + b
    return b


# Precomputed Fibonacci sequence (F(0) to F(20))
FIBONACCI: List[int] = [fibonacci(n) for n in range(21)]

# Precomputed Lucas sequence (L(0) to L(10))
LUCAS: List[int] = [lucas(n) for n in range(11)]

# Architecture derivations from φ
AXIOMS_CORE: int = fibonacci(5)      # F(5) = 5 → 5 core axioms
AXIOMS_FACETS: int = lucas(4)        # L(4) = 7 → 7 facets per axiom
DOGS_COUNT: int = lucas(5)           # L(5) = 11 → 11 Dogs (Sefirot)
REALITY_DIMS: int = lucas(4)         # L(4) = 7 → 7 Reality dimensions
ANALYSIS_DIMS: int = lucas(4)        # L(4) = 7 → 7 Analysis dimensions
TIME_DIMS: int = lucas(4)            # L(4) = 7 → 7 Time dimensions


# ════════════════════════════════════════════════════════════════════════════
# CYCLE TIMING (φ-aligned Fibonacci windows, in seconds)
# ════════════════════════════════════════════════════════════════════════════

# Perception frequencies (Fibonacci minutes × 60)
PERCEIVE_CODE_SEC: int = fibonacci(8) * 60    # F(8)=21 → 1260s (21 min)
PERCEIVE_SOLANA_SEC: int = fibonacci(7) * 60  # F(7)=13 → 780s (13 min)
PERCEIVE_MARKET_SEC: int = fibonacci(6) * 60  # F(6)=8 → 480s (8 min)
PERCEIVE_SOCIAL_SEC: int = fibonacci(9) * 60  # F(9)=34 → 2040s (34 min)

# Learning batch intervals
LEARN_BATCH_SEC: int = fibonacci(10) * 60     # F(10)=55 → 3300s (55 min)
SONA_UPDATE_SEC: int = fibonacci(9) * 60      # F(9)=34 → 2040s (34 min)
KABBALISTIC_ROUTER_SEC: int = fibonacci(11) * 60  # F(11)=89 → 5340s (89 min)

# Economic intervals
E_SCORE_UPDATE_SEC: int = fibonacci(11) * 60  # F(11)=89 → 5340s (89 min)

# Emergence detection
EMERGE_DETECT_SEC: int = fibonacci(12) * 60   # F(12)=144 → 8640s (2.4h)
TRANSCENDENCE_SEC: int = fibonacci(13) * 60   # F(13)=233 → 13980s (3.9h)

# Consciousness check: F(13) = 233 judgments triggers meta-cycle
META_CYCLE_JUDGMENTS: int = fibonacci(13)     # 233

# Meta-cognition: stuck threshold (φ times = ~2-3 repetitions)
STUCK_REPETITIONS: int = max(2, round(PHI))   # 2
STUCK_STAGNATION: int = max(3, round(PHI_2))  # 3


# ════════════════════════════════════════════════════════════════════════════
# φ MATHEMATICS
# ════════════════════════════════════════════════════════════════════════════

def phi_bound(value: float, min_val: float = 0.0, max_val: float = MAX_CONFIDENCE) -> float:
    """Clamp value to φ-aligned range [min_val, max_val]."""
    return max(min_val, min(value, max_val))


def phi_bound_score(q_score: float) -> float:
    """Clamp Q-Score to [0, 61.8]."""
    return max(0.0, min(q_score, MAX_Q_SCORE))


def phi_classify(value: float) -> str:
    """
    Classify normalized value [0.0, 1.0] into φ-aligned categories.

    Returns: 'EXCEPTIONAL', 'GOOD', 'MODERATE', 'POOR', 'CRITICAL'
    """
    if value >= 0.82:
        return "EXCEPTIONAL"
    elif value >= PHI_INV:     # 0.618
        return "GOOD"
    elif value >= PHI_INV_2:   # 0.382
        return "MODERATE"
    elif value >= PHI_INV_3:   # 0.236
        return "POOR"
    else:
        return "CRITICAL"


def geometric_mean(values: List[float]) -> float:
    """
    Geometric mean (φ-punishes outlier failures).

    Q-Score = geometric_mean(axiom_scores) — more conservative than arithmetic.
    One axiom at 0 → Q-Score = 0 (correct: total failure on one axiom = failure overall).
    """
    if not values:
        return 0.0
    if any(v <= 0 for v in values):
        return 0.0

    log_sum = sum(math.log(v) for v in values)
    return math.exp(log_sum / len(values))


def weighted_geometric_mean(values: List[float], weights: List[float]) -> float:
    """
    Weighted geometric mean for Q-Score computation.

    Q = product(v_i ^ w_i) ^ (1 / sum(w_i))
    """
    if not values or len(values) != len(weights):
        return 0.0
    if any(v <= 0 for v in values):
        return 0.0

    total_weight = sum(weights)
    if total_weight == 0:
        return 0.0

    log_sum = sum(w * math.log(v) for v, w in zip(values, weights))
    return math.exp(log_sum / total_weight)


def phi_ratio_split(total: float) -> Tuple[float, float]:
    """
    Split total into φ-aligned ratio.

    Returns (small, large) = (38.2%, 61.8%) × total
    Used for: MCTS budget split, Level 1 vs Level 2
    """
    return (total * PHI_INV_2, total * PHI_INV)


def phi_ucb(q_value: float, visits: int, parent_visits: int,
             exploration: float = math.sqrt(2)) -> float:
    """
    UCB1 formula for MCTS node selection.

    UCB(node) = Q(node) + c × √(ln(N) / n)
    """
    if visits == 0:
        return float("inf")
    exploitation = q_value / visits
    exploration_term = exploration * math.sqrt(math.log(parent_visits) / visits)
    return exploitation + exploration_term


def phi_temporal_ucb(q_value: float, visits: int, parent_visits: int,
                      depth: int, exploration: float = math.sqrt(2)) -> float:
    """
    Temporal UCB1 (Temporal MCTS innovation).

    Deeper nodes are less certain → decay by φ^depth.
    """
    if visits == 0:
        return float("inf")
    exploitation = q_value / visits
    exploration_term = exploration * math.sqrt(math.log(parent_visits) / visits)
    temporal_decay = PHI_INV ** depth  # φ⁻¹ per level of depth
    return exploitation + exploration_term * temporal_decay


# ════════════════════════════════════════════════════════════════════════════
# E-SCORE 7D WEIGHTS (φ-symmetric sequence)
# ════════════════════════════════════════════════════════════════════════════

E_SCORE_WEIGHTS: Dict[str, float] = {
    "BURN":   PHI_3,    # φ³ = 4.236 — Highest (irreversible commitment)
    "BUILD":  PHI_2,    # φ² = 2.618 — Code contribution quality
    "JUDGE":  PHI,      # φ¹ = 1.618 — Judgment accuracy
    "RUN":    1.0,      # φ⁰ = 1.000 — Execution reliability
    "SOCIAL": PHI_INV,  # φ⁻¹ = 0.618 — Community engagement
    "GRAPH":  PHI_INV_2, # φ⁻² = 0.382 — Network connectivity
    "HOLD":   PHI_INV_3, # φ⁻³ = 0.236 — Long-term commitment
}

E_SCORE_TOTAL_WEIGHT: float = sum(E_SCORE_WEIGHTS.values())
# = 4.236 + 2.618 + 1.618 + 1.000 + 0.618 + 0.382 + 0.236 = 10.708


# ════════════════════════════════════════════════════════════════════════════
# VALIDATION (run at import time)
# ════════════════════════════════════════════════════════════════════════════

def validate_phi_constants() -> None:
    """
    Validate φ mathematical relationships at 12-decimal precision.

    Raises AssertionError if any constant is wrong.
    Called automatically on import.
    """
    eps = 1e-12

    # Core identities
    assert abs(PHI * PHI_INV - 1.0) < eps, "φ × φ⁻¹ ≠ 1"
    assert abs(PHI_2 - (PHI + 1)) < eps, "φ² ≠ φ + 1"
    assert abs(PHI_3 - (2 * PHI + 1)) < eps, "φ³ ≠ 2φ + 1"
    assert abs(PHI_INV + PHI_INV_2 - 1.0) < eps, "φ⁻¹ + φ⁻² ≠ 1"

    # Fibonacci/Lucas derivations
    assert fibonacci(5) == 5, "F(5) ≠ 5"
    assert lucas(4) == 7, "L(4) ≠ 7"
    assert lucas(5) == 11, "L(5) ≠ 11"

    # Architecture counts
    assert AXIOMS_CORE == 5, "Core axioms ≠ 5"
    assert AXIOMS_FACETS == 7, "Facets per axiom ≠ 7"
    assert DOGS_COUNT == 11, "Dogs ≠ 11"

    # PBFT
    assert DOGS_QUORUM == 2 * DOGS_BYZANTINE + 1, "PBFT quorum violated"
    assert DOGS_TOTAL == 11, "Total dogs ≠ 11"

    # Fibonacci convergence to φ
    for n in range(10, 18):
        ratio = fibonacci(n) / fibonacci(n - 1)
        assert abs(ratio - PHI) < 0.01, f"F({n})/F({n-1}) doesn't converge to φ"

    # Verdict thresholds sanity
    assert HOWL_MIN > WAG_MIN > GROWL_MIN > 0, "Verdict thresholds out of order"
    assert abs(WAG_MIN - MAX_Q_SCORE) < 1e-10, "WAG threshold ≠ MAX_Q_SCORE"

    # E-Score weights sum
    assert abs(E_SCORE_TOTAL_WEIGHT - 10.708) < 0.001, "E-Score weights sum incorrect"


# Auto-validate on import (LAW 5: φ-bounded checks)
validate_phi_constants()
