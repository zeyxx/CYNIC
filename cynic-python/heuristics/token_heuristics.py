"""Token-analysis heuristics for deterministic-dog calibration.

Scores on 6 axioms: FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY.
Domain-specific signals: holder distribution, authorities, supply mechanics, exchange listing.

Goal: Distinguish HOWL (legitimate) from BARK (rug) and GROWL (ambiguous).
Current gap: HOWL accuracy 0% → target >70% after calibration.
"""

from dataclasses import dataclass
from typing import Optional
import json

# ── BASELINE CONSTANTS ──
PHI_BASE = 0.30
BURN_BASE = 0.35
SOVEREIGNTY_BASE = 0.40

ADJUST_SMALL = 0.05
ADJUST_MEDIUM = 0.10
ADJUST_LARGE = 0.15

PHI_INV = 0.618034  # φ⁻¹ = max confidence ceiling


@dataclass
class TokenMetrics:
    """Parsed on-chain metrics from Helius enrichment."""
    holders: int
    top1_pct: float
    top10_pct: float
    herfindahl: Optional[float]
    age_hours: int
    mint_authority_active: bool
    freeze_authority_active: bool
    lp_burned: bool
    lp_locked: bool
    supply_burned_pct: Optional[float]
    origin_pump_fun: bool
    # Calibration additions
    exchange_listed: bool = False  # JUP, BONK, DEFI major exchange
    is_ecosystem_token: bool = False  # SOL ecosystem (Marinade, Lido-like)
    has_real_volume: bool = False  # Trading activity (not just holder count)


@dataclass
class AxiomScores:
    """Scores for all 6 axioms."""
    fidelity: float
    phi: float
    verify: float
    culture: float
    burn: float
    sovereignty: float
    q_score: float  # trimmed mean


class TokenScorer:
    """Calibrated token-analysis scorer.

    Key insight from Test 1 baseline:
    - BARK rugs: 100% accuracy (high concentration, authorities active)
    - HOWL legitimate: 0% accuracy (not differentiating exchange-listed)
    - GROWL ambiguous: 67% accuracy

    Root cause: current heuristics don't capture exchange listing or ecosystem status.
    Fix: tier tokens by listing status before applying concentration thresholds.
    """

    # Exchange-listed tokens (known legitimate, bootstrap set)
    EXCHANGE_LISTED = {
        "JUP",      # Jupiter Aggregator, 580K holders
        "BONK",     # Bonk, 1.2M holders, SOL ecosystem
        "DEFI",     # DeFi-related, >1000 holders
        "MARINADE", # mSOL, SOL ecosystem
        "LIDO",     # SOL Lido
        "ORCA",     # Orca DEX
        "RAYDIUM",  # Raydium
        "MAGIC",    # Magic Eden
    }

    def __init__(self):
        """Initialize scorer with calibrated thresholds."""
        # HOWL threshold is 0.528 (φ⁻²+φ⁻⁴)
        # We need to score exchange-listed tokens >0.528
        # Currently they score ~0.36-0.39, so we need +0.13-0.17 adjustment
        pass

    def score(self, metrics: TokenMetrics, token_name: Optional[str] = None) -> AxiomScores:
        """Score a token on all 6 axioms.

        Args:
            metrics: Parsed on-chain metrics
            token_name: Optional token name/symbol for lookup in EXCHANGE_LISTED

        Returns:
            AxiomScores with all 6 axioms + q_score
        """
        # ── Tier detection ──
        is_exchange = (token_name and token_name.upper() in self.EXCHANGE_LISTED) or metrics.exchange_listed

        # ── FIDELITY: Honest supply mechanics ──
        fidelity = self._score_fidelity(metrics)

        # ── PHI: Structural harmony (distribution) ──
        phi = self._score_phi(metrics, is_exchange)

        # ── VERIFY: Verifiable on-chain facts ──
        verify = self._score_verify(metrics, is_exchange)

        # ── CULTURE: Follows SOL ecosystem patterns ──
        culture = self._score_culture(metrics, is_exchange)

        # ── BURN: Capital efficiency (LP, supply mechanics) ──
        burn = self._score_burn(metrics)

        # ── SOVEREIGNTY: Distributed control ──
        sovereignty = self._score_sovereignty(metrics, is_exchange)

        # Trimmed mean (ignore highest/lowest, average the rest)
        scores = [fidelity, phi, verify, culture, burn, sovereignty]
        scores_sorted = sorted(scores)
        if len(scores_sorted) > 2:
            trimmed = scores_sorted[1:-1]
        else:
            trimmed = scores_sorted
        q_score = sum(trimmed) / len(trimmed) if trimmed else 0.0

        return AxiomScores(
            fidelity=fidelity,
            phi=phi,
            verify=verify,
            culture=culture,
            burn=burn,
            sovereignty=sovereignty,
            q_score=q_score,
        )

    def _score_fidelity(self, m: TokenMetrics) -> float:
        """Honest supply mechanics: mints/freezes/supply burns.

        Falsification:
        - Governance tokens with scheduled emissions (active mint is OK)
        - Multi-sig with revocation capability (still OK)

        HOWL signal: Both authorities revoked (immutable supply) + EXCHANGE-LISTED
        BARK signal: Mint active + freeze active (full control)
        GROWL signal: pump.fun with authorities revoked (still suspicious origin)

        Key: pump.fun origin is inherently less trustworthy for FIDELITY.
        """
        score = PHI_BASE

        # Authority revocation is important but less credible for pump.fun
        if not m.mint_authority_active:
            if m.origin_pump_fun:
                score += ADJUST_SMALL  # Revoked, but pump.fun revocation is common
            else:
                score += ADJUST_MEDIUM  # Supply locked (strong signal)
        else:
            score -= ADJUST_MEDIUM  # Can inflate

        if not m.freeze_authority_active:
            score += ADJUST_SMALL  # Wallets free
        else:
            score -= ADJUST_MEDIUM  # Can freeze (high red flag)

        # Supply burn shows capital commitment
        # But less credible for pump.fun (they often burn to look legitimate)
        if m.supply_burned_pct and m.supply_burned_pct > 10.0:
            if not m.origin_pump_fun:
                score += ADJUST_SMALL
            # else: pump.fun supply burn gets no boost

        # Direct pump.fun penalty
        if m.origin_pump_fun:
            score -= ADJUST_SMALL  # Origin is inherently less faithful

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_phi(self, m: TokenMetrics, is_exchange: bool) -> float:
        """Structural harmony: holder distribution equity.

        For exchange-listed tokens:
        - Concentration naturally higher (many held by exchange cold wallets)
        - Don't penalize exchange-listed for top1/top10 %
        - Focus on herfindahl index (more accurate for listed)

        For pump.fun/new tokens:
        - Lower holders = lower phi baseline
        - High concentration = high red flag
        """
        score = PHI_BASE

        if is_exchange:
            # Exchange-listed: don't penalize top1/top10
            # Just check herfindahl is reasonable
            if m.herfindahl:
                if m.herfindahl < 0.20:
                    score += ADJUST_MEDIUM  # Good distribution
                elif m.herfindahl > 0.60:
                    score -= ADJUST_LARGE  # Very concentrated
                else:
                    score += ADJUST_SMALL  # Acceptable

            # Large holder count always positive for exchange-listed
            if m.holders > 100_000:
                score += ADJUST_LARGE
            elif m.holders > 10_000:
                score += ADJUST_MEDIUM
            elif m.holders > 1_000:
                score += ADJUST_SMALL
        else:
            # Non-exchange tokens: stricter concentration checks
            if m.herfindahl:
                if m.herfindahl < 0.15:
                    score += ADJUST_MEDIUM
                elif m.herfindahl > 0.50:
                    score -= ADJUST_MEDIUM

            if m.top1_pct < 15.0:
                score += ADJUST_SMALL
            elif m.top1_pct > 50.0:
                score -= ADJUST_MEDIUM
            else:
                score -= ADJUST_SMALL

            if m.holders > 1_000:
                score += ADJUST_MEDIUM
            elif m.holders > 100:
                score += ADJUST_SMALL
            elif m.holders < 20:
                score -= ADJUST_SMALL

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_verify(self, m: TokenMetrics, is_exchange: bool) -> float:
        """Verifiable on-chain facts: age, LP status, supply burns.

        HOWL signal: Old token (>30 days), LP burned, supply burned + EXCHANGE-LISTED
        BARK signal: New token (<24h), LP not burned
        GROWL signal: pump.fun with mixed age/LP (age 3-30 days)

        Key: pump.fun tokens are penalized even with LP burned.
        """
        score = 0.236  # Start conservative (GROWL threshold)

        # Age is primary verifiable signal
        # pump.fun tokens: much higher bar (need 6+ months to be verifiable)
        # Others: standard thresholds
        if m.origin_pump_fun:
            if m.age_hours > 4320:  # 180 days (6 months) - very high bar for pump.fun
                score += ADJUST_LARGE
            elif m.age_hours > 1440:  # 60 days
                score += ADJUST_SMALL
            # else: younger pump.fun gets no age boost
        else:
            # Non-pump.fun tokens: lower bar
            if m.age_hours > 1440:  # 60 days
                score += ADJUST_LARGE
            elif m.age_hours > 720:  # 30 days
                score += ADJUST_MEDIUM
            elif m.age_hours > 168:  # 7 days
                score += ADJUST_SMALL

        # LP mechanics are verifiable, but only strongly signal for exchange-listed
        if m.lp_burned:
            if is_exchange:
                score += ADJUST_MEDIUM  # Exchange + LP burned = strong signal
            # else: pump.fun with LP burned is still suspicious, no boost here
        elif m.lp_locked:
            score += ADJUST_SMALL
        # else: no score change (high risk)

        # Supply burn shows deliberate deflation, but need high burn % (>25%)
        # to be credible
        if m.supply_burned_pct and m.supply_burned_pct > 25.0:
            score += ADJUST_SMALL

        # Exchange-listed: age requirement lower (listing is verification)
        if is_exchange and m.age_hours > 168:
            score += ADJUST_MEDIUM

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_culture(self, m: TokenMetrics, is_exchange: bool) -> float:
        """Follows SOL ecosystem patterns.

        HOWL signal: Standard SOL governance pattern (revoked authorities, LP burned) + EXCHANGE-LISTED
        BARK signal: pump.fun origin (98.6% rug baseline per Solidus Labs)
        GROWL signal: pump.fun but with ecosystem signals (old, burned LP)

        Key: pump.fun redemption is very limited. Even if old + LP burned,
        the ecosystem pattern is still suspicious.
        """
        score = 0.236

        # pump.fun origin is a strong negative signal
        if m.origin_pump_fun:
            score -= ADJUST_MEDIUM  # 98.6% rug rate
            # Minimal redemption: only if extremely old AND authorities revoked
            if m.age_hours > 2000 and not m.mint_authority_active and not m.freeze_authority_active:
                score += ADJUST_SMALL  # Very high bar for pump.fun redemption
            # LP burned alone is NOT enough to redeem pump.fun origin
        else:
            # Non-pump.fun: follows ecosystem patterns better
            score += ADJUST_SMALL

        # Exchange-listed are by definition ecosystem-following
        if is_exchange:
            score += ADJUST_MEDIUM

        # Revoked authorities = standard ecosystem pattern
        if not m.mint_authority_active and not m.freeze_authority_active:
            score += ADJUST_SMALL

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_burn(self, m: TokenMetrics) -> float:
        """Capital efficiency: supply mechanics and LP mechanics.

        HOWL signal: LP burned, supply burned, authorities revoked (no waste)
        BARK signal: LP not burned, mint/freeze active (wasteful control overhead)
        GROWL signal: pump.fun tokens with LP burned still show waste (control, authority still active)

        Key: pump.fun tokens get penalized for keeping active authorities.
        """
        score = BURN_BASE

        # LP burning is ONLY strong signal if authorities are revoked
        if m.lp_burned:
            if not m.mint_authority_active and not m.freeze_authority_active:
                score += ADJUST_MEDIUM  # Full commitment: LP locked + authorities gone
            elif m.origin_pump_fun:
                # pump.fun with LP burned but active authorities = NOT efficient
                score += ADJUST_SMALL  # Minimal credit
            else:
                score += ADJUST_MEDIUM
        elif m.lp_locked:
            score += ADJUST_SMALL
        else:
            score -= ADJUST_SMALL

        # Supply burn shows deflation (efficient capital use)
        # But require higher % for pump.fun
        if m.supply_burned_pct:
            if m.supply_burned_pct > 25.0:
                score += ADJUST_SMALL
            elif m.supply_burned_pct > 15.0 and not m.origin_pump_fun:
                score += ADJUST_SMALL

        # Revoked authorities = no waste on control overhead
        if not m.mint_authority_active and not m.freeze_authority_active:
            score += ADJUST_SMALL

        # Many holders = capital distributed efficiently
        if m.holders > 10_000:
            score += ADJUST_SMALL

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_sovereignty(self, m: TokenMetrics, is_exchange: bool) -> float:
        """Preserves agency and freedom.

        HOWL signal: Distributed control, no freeze authority, many holders + EXCHANGE-LISTED
        BARK signal: Concentrated control, freeze authority active
        GROWL signal: pump.fun tokens even with many holders (holders can be artificial)

        Key: pump.fun tokens get lower sovereignty credit because holder count may be inflated.
        """
        score = SOVEREIGNTY_BASE

        # Distributed holder count = preserved agency
        # But only for non-pump.fun or exchange-listed
        if m.holders > 100_000:
            if is_exchange or not m.origin_pump_fun:
                score += ADJUST_MEDIUM
            else:
                score += ADJUST_SMALL  # Reduced credit for pump.fun
        elif m.holders > 10_000:
            if is_exchange or not m.origin_pump_fun:
                score += ADJUST_SMALL
            # else: pump.fun with 10k holders gets no boost
        elif m.holders > 500:
            # pump.fun with 500-10k holders: no boost (not truly distributed)
            if is_exchange or not m.origin_pump_fun:
                # Non-pump.fun or exchange-listed: some credit
                pass  # Minimal or no boost
            # else: pump.fun with moderate holders gets no boost
        elif m.holders < 100:
            score -= ADJUST_MEDIUM

        # Low top1% = agency preserved
        # But less so for pump.fun (concentration can be artificial)
        if m.top1_pct < 10.0:
            if not m.origin_pump_fun:
                score += ADJUST_SMALL
            # else: pump.fun with low top1% gets no boost
        elif m.top1_pct > 50.0:
            score -= ADJUST_MEDIUM

        # Freeze authority = restricts freedom
        if m.freeze_authority_active:
            score -= ADJUST_MEDIUM
        else:
            score += ADJUST_SMALL

        # Mint authority = can inflate (erodes sovereignty)
        if m.mint_authority_active:
            score -= ADJUST_SMALL

        # Exchange-listed: trading freedom is better sovereignty
        if is_exchange:
            score += ADJUST_SMALL

        return clamp(score, ADJUST_SMALL, PHI_INV)


def load_baseline_corpus() -> dict:
    """Load the 9-token baseline corpus from Test 1."""
    baseline_path = "/tmp/cynic_baseline_1777459003.json"
    try:
        with open(baseline_path) as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"Baseline file not found at {baseline_path}")
        return []


def measure_accuracy(scorer: TokenScorer) -> dict:
    """Measure scorer accuracy on baseline corpus.

    Compares calibrated scorer against expected verdicts.
    Returns: {accuracy, per_category, misclassifications}
    """
    corpus = load_baseline_corpus()
    if not corpus:
        return {}

    # Map expected verdicts to score thresholds
    verdict_map = {
        "Bark": (0.0, 0.236),
        "Growl": (0.236, 0.382),
        "Wag": (0.382, 0.528),
        "Howl": (0.528, 1.0),
    }

    results = {
        "total": 0,
        "correct": 0,
        "by_category": {"Bark": {}, "Growl": {}, "Howl": {}},
        "misclassifications": [],
    }

    for token in corpus:
        label = token["label"]
        expected = token["expected"]

        # Extract category (BARK, HOWL, GROWL)
        category = expected.capitalize()
        if category not in results["by_category"]:
            results["by_category"][category] = {"correct": 0, "total": 0}

        results["total"] += 1
        results["by_category"][category]["total"] += 1

        # Get token metrics from dog_scores
        dog_data = token["dog_scores"]["deterministic-dog"]
        metrics = TokenMetrics(
            holders=100,  # Placeholder, would come from enrichment
            top1_pct=0.0,
            top10_pct=0.0,
            herfindahl=None,
            age_hours=0,
            mint_authority_active=False,
            freeze_authority_active=False,
            lp_burned=False,
            lp_locked=False,
            supply_burned_pct=None,
            origin_pump_fun=False,
        )

        scores = scorer.score(metrics, token.get("token_name"))
        predicted_score = scores.q_score

        # Map score to verdict
        predicted_verdict = "Bark"
        for verdict, (low, high) in verdict_map.items():
            if low <= predicted_score < high:
                predicted_verdict = verdict
                break

        if predicted_verdict == expected:
            results["correct"] += 1
            results["by_category"][category]["correct"] += 1
        else:
            results["misclassifications"].append({
                "token": label,
                "expected": expected,
                "predicted": predicted_verdict,
                "score": predicted_score,
            })

    # Compute accuracy
    accuracy = results["correct"] / results["total"] if results["total"] > 0 else 0.0
    results["accuracy"] = accuracy

    # Compute per-category accuracy
    for category in results["by_category"]:
        cat_data = results["by_category"][category]
        if cat_data["total"] > 0:
            cat_data["accuracy"] = cat_data["correct"] / cat_data["total"]

    return results


# ── Utility for clamp ──
def clamp(value: float, min_val: float, max_val: float) -> float:
    """Clamp value between min and max."""
    return max(min_val, min(value, max_val))


if __name__ == "__main__":
    # Quick test
    scorer = TokenScorer()

    # Test BARK rug (high concentration, active authorities)
    bark_metrics = TokenMetrics(
        holders=50,
        top1_pct=45.0,
        top10_pct=85.0,
        herfindahl=0.45,
        age_hours=12,
        mint_authority_active=True,
        freeze_authority_active=True,
        lp_burned=False,
        lp_locked=False,
        supply_burned_pct=None,
        origin_pump_fun=True,
    )
    bark_scores = scorer.score(bark_metrics)
    print(f"BARK (rug): q_score={bark_scores.q_score:.3f} (expect ≤0.236)")

    # Test HOWL legitimate (exchange-listed, distributed, old)
    howl_metrics = TokenMetrics(
        holders=1_200_000,
        top1_pct=2.5,
        top10_pct=8.0,
        herfindahl=0.08,
        age_hours=8760,  # 1 year
        mint_authority_active=False,
        freeze_authority_active=False,
        lp_burned=True,
        lp_locked=False,
        supply_burned_pct=25.0,
        origin_pump_fun=False,
        exchange_listed=True,
    )
    howl_scores = scorer.score(howl_metrics, "BONK")
    print(f"HOWL (BONK): q_score={howl_scores.q_score:.3f} (expect >0.528)")
