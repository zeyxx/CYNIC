"""Wallet signal analysis for token judgment.

Scores holder behavior patterns on 6 axioms:
- FIDELITY: real wallets (low bot %, real humans)
- PHI: distributed holders (no concentration, many whales)
- VERIFY: verifiable trading (daily active traders, on exchanges)
- CULTURE: follows SOL norms (held on retail wallets, not concentrated)
- BURN: efficient capital (high retail %, low exchange %)
- SOVEREIGNTY: decentralized control (no whales, many holders)
"""

from dataclasses import dataclass
from dataset_builder import WalletSignals

PHI_BASE = 0.30
ADJUST_SMALL = 0.05
ADJUST_MEDIUM = 0.10
ADJUST_LARGE = 0.15
PHI_INV = 0.618034


@dataclass
class AxiomScores:
    fidelity: float
    phi: float
    verify: float
    culture: float
    burn: float
    sovereignty: float
    q_score: float


def clamp(value: float, min_val: float, max_val: float) -> float:
    return max(min_val, min(value, max_val))


class WalletScorer:
    """Score wallet holder behavior on all 6 axioms."""

    def score(self, signals: WalletSignals) -> AxiomScores:
        """Score wallet signals.

        Key insight: Wallets reveal intent that on-chain metrics miss:
        - Real users vs bots (bot_score, NFT overlap)
        - Whale risk (whale_count, top_10_hold_pct)
        - Organic interest (daily active traders, accumulation trend)
        - Institutional vs retail (exchange_held_pct)
        """

        fidelity = self._score_fidelity(signals)
        phi = self._score_phi(signals)
        verify = self._score_verify(signals)
        culture = self._score_culture(signals)
        burn = self._score_burn(signals)
        sovereignty = self._score_sovereignty(signals)

        # Trimmed mean
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

    def _score_fidelity(self, w: WalletSignals) -> float:
        """Real wallets vs bots.

        Falsification:
        - New tokens naturally have higher bot % (easier to bot new tokens)
        - SOL ecosystem bots are sophisticated (NFT overlap can be faked)

        HOWL signal: <10% bot score, high NFT overlap (real users)
        BARK signal: >50% bot score, no NFT overlap (pure bots)
        """
        score = PHI_BASE

        # Bot score is the primary fidelity signal
        if w.bot_score < 0.10:
            score += ADJUST_LARGE  # Very real wallets
        elif w.bot_score < 0.30:
            score += ADJUST_MEDIUM
        elif w.bot_score < 0.50:
            score -= ADJUST_SMALL
        else:
            score -= ADJUST_LARGE  # Mostly bots

        # NFT holder overlap shows real SOL ecosystem users
        if w.nft_holder_overlap and w.nft_holder_overlap > 0.20:
            score += ADJUST_MEDIUM  # Real ecosystem users
        elif w.nft_holder_overlap and w.nft_holder_overlap > 0.05:
            score += ADJUST_SMALL
        elif w.nft_holder_overlap == 0.0:
            score -= ADJUST_SMALL  # Suspicious

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_phi(self, w: WalletSignals) -> float:
        """Distributed holders: no concentration, many whales.

        HOWL signal: 0 whales, <10% top 10, many holders
        BARK signal: 3+ whales, >90% top 10, few holders
        """
        score = PHI_BASE

        # Whale count (0-1 is ideal)
        if w.whale_count == 0:
            score += ADJUST_LARGE
        elif w.whale_count == 1:
            score += ADJUST_MEDIUM
        elif w.whale_count <= 2:
            score -= ADJUST_SMALL
        else:
            score -= ADJUST_LARGE

        # Top 10 concentration
        if w.top_10_hold_pct < 10.0:
            score += ADJUST_LARGE
        elif w.top_10_hold_pct < 30.0:
            score += ADJUST_MEDIUM
        elif w.top_10_hold_pct < 50.0:
            score -= ADJUST_SMALL
        else:
            score -= ADJUST_LARGE

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_verify(self, w: WalletSignals) -> float:
        """Verifiable trading activity: real traders on exchanges.

        HOWL signal: >500 daily traders, long hold duration
        BARK signal: <10 daily traders, <1 day hold duration
        """
        score = 0.236

        # Daily active traders (the smoking gun for real interest)
        if w.daily_active_traders > 500:
            score += ADJUST_LARGE  # Very active
        elif w.daily_active_traders > 100:
            score += ADJUST_MEDIUM
        elif w.daily_active_traders > 20:
            score += ADJUST_SMALL
        elif w.daily_active_traders < 10:
            score -= ADJUST_SMALL  # Very low activity

        # Average hold duration (conviction signal)
        if w.avg_hold_duration_days > 30:
            score += ADJUST_MEDIUM  # Long-term holders
        elif w.avg_hold_duration_days > 7:
            score += ADJUST_SMALL
        elif w.avg_hold_duration_days < 1:
            score -= ADJUST_MEDIUM  # Pump & dump

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_culture(self, w: WalletSignals) -> float:
        """Follows SOL ecosystem norms: retail-held, not exchange-hoarded.

        HOWL signal: >70% retail, <15% exchange
        BARK signal: <20% retail, >60% exchange (likely insider dump)
        """
        score = PHI_BASE

        # Retail holding is the culture standard for SOL
        if w.retail_held_pct > 70.0:
            score += ADJUST_LARGE
        elif w.retail_held_pct > 50.0:
            score += ADJUST_MEDIUM
        elif w.retail_held_pct > 30.0:
            score -= ADJUST_SMALL
        else:
            score -= ADJUST_LARGE

        # Exchange holding (high % = likely insiders dumping)
        if w.exchange_held_pct < 15.0:
            score += ADJUST_SMALL
        elif w.exchange_held_pct < 30.0:
            pass  # Neutral
        else:
            score -= ADJUST_MEDIUM  # Too much on exchange

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_burn(self, w: WalletSignals) -> float:
        """Efficient capital: high retail %, low waste on exchange wallets.

        BURN = efficient use of capital
        """
        score = 0.35  # BURN_BASE

        # Retail holding is efficient (capital in hands)
        if w.retail_held_pct > 75.0:
            score += ADJUST_LARGE
        elif w.retail_held_pct > 50.0:
            score += ADJUST_MEDIUM
        elif w.retail_held_pct < 30.0:
            score -= ADJUST_MEDIUM

        # Exchange holding is inefficient (centralized risk)
        if w.exchange_held_pct > 50.0:
            score -= ADJUST_LARGE  # Very inefficient
        elif w.exchange_held_pct > 30.0:
            score -= ADJUST_SMALL

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_sovereignty(self, w: WalletSignals) -> float:
        """Decentralized control: no whales, many holders, active traders.

        HOWL signal: 0 whales, many daily traders, high accumulation
        BARK signal: 3+ whales, few traders, 95% dumping
        """
        score = 0.40  # SOVEREIGNTY_BASE

        # Whale count (whales restrict freedom)
        if w.whale_count == 0:
            score += ADJUST_LARGE
        elif w.whale_count == 1:
            score += ADJUST_MEDIUM
        elif w.whale_count > 3:
            score -= ADJUST_LARGE

        # Many traders = many stakeholders with power
        if w.daily_active_traders > 500:
            score += ADJUST_LARGE
        elif w.daily_active_traders > 100:
            score += ADJUST_MEDIUM
        elif w.daily_active_traders < 20:
            score -= ADJUST_MEDIUM

        # Accumulation trend (positive = holders staying)
        if w.accumulation_trend > 0.60:
            score += ADJUST_MEDIUM  # Holders confident
        elif w.accumulation_trend < 0.30:
            score -= ADJUST_MEDIUM  # Holders dumping

        return clamp(score, ADJUST_SMALL, PHI_INV)


if __name__ == "__main__":
    scorer = WalletScorer()

    # Test HOWL (healthy wallets)
    howl_signals = WalletSignals(
        whale_count=0,
        top_10_hold_pct=8.0,
        bot_score=0.05,
        daily_active_traders=1000,
        avg_hold_duration_days=120,
        accumulation_trend=0.65,
        exchange_held_pct=15.0,
        retail_held_pct=80.0,
        nft_holder_overlap=0.30,
    )
    howl_scores = scorer.score(howl_signals)
    print(f"HOWL (healthy wallets): q_score={howl_scores.q_score:.3f} (expect >0.528)")

    # Test BARK (bot wallets)
    bark_signals = WalletSignals(
        whale_count=3,
        top_10_hold_pct=98.0,
        bot_score=0.80,
        daily_active_traders=5,
        avg_hold_duration_days=2,
        accumulation_trend=0.05,
        exchange_held_pct=50.0,
        retail_held_pct=5.0,
        nft_holder_overlap=0.0,
    )
    bark_scores = scorer.score(bark_signals)
    print(f"BARK (bot wallets): q_score={bark_scores.q_score:.3f} (expect ≤0.236)")
