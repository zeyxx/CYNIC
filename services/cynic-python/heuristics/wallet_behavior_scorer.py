#!/usr/bin/env python3
"""
Wallet Behavior Analysis — Human vs Bot Detection

Scores Solana wallets 0.0-1.0 on authenticity heuristic.
Gate: score >= φ⁻¹ = 0.618 → verified human

Four-tier scoring:
  1. Age (25%) — wallet tenure
  2. Diversity (30%) — token/program spread
  3. Temporal (25%) — transaction spread over time
  4. Anomalies (20%) — red flag penalties
"""

import logging
from dataclasses import dataclass, field
from typing import Optional
from datetime import datetime, timedelta

# Constants
PHI_INV = 0.618034  # φ⁻¹ gate for "verified human"
VERIFIED_HUMAN_THRESHOLD = PHI_INV

# Configure logging
logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] wallet_behavior: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


@dataclass
class WalletProfile:
    """On-chain wallet behavior extracted from Helius data."""

    wallet_address: str

    # Tier 1: Age (in days)
    wallet_age_days: int

    # Tier 2: Diversity
    token_count: int              # unique token mints
    program_count: int            # unique programs called
    unique_swap_pairs: int = 0    # distinct [token_a, token_b] pairs

    # Tier 3: Temporal Spread
    activity_span_days: int = 0   # first to last transaction
    total_transactions: int = 0
    transaction_density: float = 0.0  # txs per day
    gap_max_days: int = 0         # longest gap between consecutive txs

    # Tier 4: Anomalies (red flags)
    all_txs_same_hour: bool = False
    single_token_pct: float = 0.0  # % of interactions on single token
    recent_whale_flag: bool = False  # age < 1 day AND balance > threshold
    transaction_frequency_anomaly: bool = False  # > 100 txs in 1 hour

    # Computed outputs
    authenticity_score: float = field(default=0.0, init=False)
    is_verified_human: bool = field(default=False, init=False)


def score_age(profile: WalletProfile) -> float:
    """
    Tier 1: Wallet Age Scoring

    Bot signature: fresh wallets (created daily)
    Human signature: persistent wallets (weeks+)
    """
    age = profile.wallet_age_days

    if age < 7:
        return 0.15  # Fresh
    elif age < 28:
        return 0.40  # Established
    else:
        return 0.65  # Proven


def score_diversity(profile: WalletProfile) -> float:
    """
    Tier 2: Token & Program Diversity Scoring

    Bot signature: single-token concentration (pump & dump)
    Human signature: varied interactions across multiple projects
    """
    # Token diversity
    token_count = profile.token_count
    if token_count < 3:
        token_score = 0.15
    elif token_count < 10:
        token_score = 0.40
    else:
        token_score = 0.65

    # Program diversity multiplier
    program_count = profile.program_count
    if program_count < 2:
        program_factor = 0.7
    elif program_count < 5:
        program_factor = 0.9
    else:
        program_factor = 1.0

    # Single-token concentration penalty
    if profile.single_token_pct > 90:
        concentration_penalty = 0.5
    elif profile.single_token_pct > 75:
        concentration_penalty = 0.7
    else:
        concentration_penalty = 1.0

    combined = token_score * program_factor * concentration_penalty
    return min(0.95, combined)  # Cap at realistic max


def score_temporal(profile: WalletProfile) -> float:
    """
    Tier 3: Temporal Spread Scoring

    Bot signature: concentrated activity (hours/days)
    Human signature: sustained activity (weeks+)
    """
    span = profile.activity_span_days

    # Base span score
    if span < 3:
        span_score = 0.15
    elif span < 14:
        span_score = 0.35
    else:
        span_score = 0.65

    # Adjust by gap (dormancy)
    max_gap = profile.gap_max_days
    if max_gap > 7:
        gap_factor = 0.7  # Dormant periods suggest inactive bot
    elif max_gap > 2:
        gap_factor = 0.9  # Normal sleeping pattern
    else:
        gap_factor = 1.0  # Active engagement

    # Adjust by density (steady state)
    density = profile.transaction_density
    if density < 0.1:  # < 1 tx per 10 days
        density_factor = 0.7
    elif density < 1.0:  # < 1 tx per day
        density_factor = 0.9
    else:
        density_factor = 1.0

    combined = span_score * gap_factor * density_factor
    return min(0.95, combined)


def score_anomalies(profile: WalletProfile) -> float:
    """
    Tier 4: Anomaly Detection (Red Flags)

    Critical failures → instant low score (0.10)
    Otherwise → penalty multiplier

    Returns: penalty factor (1.0 = no penalty, 0.1 = critical)
    """

    # Critical failures (instant BARK)
    if profile.all_txs_same_hour:
        logger.debug(f"{profile.wallet_address}: CRITICAL all_txs_same_hour")
        return 0.10

    if profile.single_token_pct > 95:
        logger.debug(f"{profile.wallet_address}: CRITICAL single_token_pct > 95%")
        return 0.10

    if profile.recent_whale_flag:
        logger.debug(f"{profile.wallet_address}: CRITICAL recent_whale_flag")
        return 0.10

    if profile.transaction_frequency_anomaly:
        logger.debug(f"{profile.wallet_address}: CRITICAL transaction_frequency_anomaly")
        return 0.10

    # Soft penalties
    penalty = 1.0

    if profile.single_token_pct > 85:
        penalty *= 0.85
        logger.debug(f"{profile.wallet_address}: soft penalty single_token_pct > 85%")

    if profile.gap_max_days > 30:
        penalty *= 0.90
        logger.debug(f"{profile.wallet_address}: soft penalty gap_max_days > 30")

    return max(0.10, penalty)


def score_wallet(profile: WalletProfile) -> WalletProfile:
    """
    Compute composite authenticity score for wallet.

    Weights:
      - Age: 25%
      - Diversity: 30%
      - Temporal: 25%
      - Anomalies: 20%

    Returns: WalletProfile with authenticity_score and is_verified_human populated
    """

    # Compute each tier
    age = score_age(profile)
    diversity = score_diversity(profile)
    temporal = score_temporal(profile)
    anomaly_factor = score_anomalies(profile)

    logger.info(
        f"{profile.wallet_address}: "
        f"age={age:.2f} diversity={diversity:.2f} temporal={temporal:.2f} anomaly_factor={anomaly_factor:.2f}"
    )

    # Composite score
    composite = (
        age * 0.25 +
        diversity * 0.30 +
        temporal * 0.25 +
        (1.0 * anomaly_factor) * 0.20
    )

    # Clamp to [0.05, 0.95] to avoid false certainty
    score = max(0.05, min(0.95, composite))

    # Gate: is_verified_human?
    is_verified = score >= VERIFIED_HUMAN_THRESHOLD

    # Populate output
    profile.authenticity_score = score
    profile.is_verified_human = is_verified

    gate_status = "VERIFIED_HUMAN ✓" if is_verified else "NOT_VERIFIED ✗"
    logger.info(f"{profile.wallet_address}: score={score:.3f} → {gate_status}")

    return profile


# ============================================================================
# Test Suite (Synthetic Data)
# ============================================================================

def test_human_profile():
    """Authentic human wallet."""
    profile = WalletProfile(
        wallet_address="test_human_1",
        wallet_age_days=45,
        token_count=22,
        program_count=8,
        unique_swap_pairs=15,
        activity_span_days=40,
        total_transactions=87,
        transaction_density=2.17,
        gap_max_days=3,
        all_txs_same_hour=False,
        single_token_pct=8.5,
        recent_whale_flag=False,
        transaction_frequency_anomaly=False,
    )
    result = score_wallet(profile)
    assert result.is_verified_human, f"Expected verified human, got score={result.authenticity_score}"
    assert result.authenticity_score > 0.6
    print(f"✓ test_human_profile: score={result.authenticity_score:.3f}")


def test_sybil_profile():
    """Pump & dump bot wallet."""
    profile = WalletProfile(
        wallet_address="test_sybil_1",
        wallet_age_days=3,
        token_count=1,
        program_count=2,
        unique_swap_pairs=1,
        activity_span_days=2,
        total_transactions=47,
        transaction_density=23.5,
        gap_max_days=1,
        all_txs_same_hour=True,  # CRITICAL
        single_token_pct=98.0,   # CRITICAL
        recent_whale_flag=False,
        transaction_frequency_anomaly=False,
    )
    result = score_wallet(profile)
    assert not result.is_verified_human, f"Expected NOT verified, got score={result.authenticity_score}"
    assert result.authenticity_score < 0.35
    print(f"✓ test_sybil_profile: score={result.authenticity_score:.3f}")


def test_emerging_profile():
    """New but legitimate wallet (below gate)."""
    profile = WalletProfile(
        wallet_address="test_emerging_1",
        wallet_age_days=8,
        token_count=4,
        program_count=3,
        unique_swap_pairs=3,
        activity_span_days=7,
        total_transactions=12,
        transaction_density=1.71,
        gap_max_days=5,
        all_txs_same_hour=False,
        single_token_pct=35.0,
        recent_whale_flag=False,
        transaction_frequency_anomaly=False,
    )
    result = score_wallet(profile)
    assert not result.is_verified_human, f"Expected NOT verified (below gate), got score={result.authenticity_score}"
    assert 0.30 < result.authenticity_score < 0.60
    print(f"✓ test_emerging_profile: score={result.authenticity_score:.3f}")


def test_whale_sybil():
    """Recent whale (MEV bot signature)."""
    profile = WalletProfile(
        wallet_address="test_whale_sybil",
        wallet_age_days=1,          # < 1 day
        token_count=2,
        program_count=3,
        activity_span_days=1,
        total_transactions=150,
        transaction_density=150.0,
        gap_max_days=0,
        all_txs_same_hour=False,
        single_token_pct=45.0,
        recent_whale_flag=True,     # CRITICAL
        transaction_frequency_anomaly=True,  # CRITICAL
    )
    result = score_wallet(profile)
    assert not result.is_verified_human
    assert result.authenticity_score <= 0.20  # Critical flags → low score
    print(f"✓ test_whale_sybil: score={result.authenticity_score:.3f}")


def run_all_tests():
    """Run all unit tests."""
    print("\n=== Wallet Behavior Scorer Unit Tests ===\n")
    test_human_profile()
    test_sybil_profile()
    test_emerging_profile()
    test_whale_sybil()
    print("\n✓ All tests passed!\n")


if __name__ == "__main__":
    run_all_tests()
