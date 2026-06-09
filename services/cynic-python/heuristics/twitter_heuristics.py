"""Twitter signal analysis for token judgment.

Scores community sentiment, engagement, and red flags on 6 axioms:
- FIDELITY: honest community (low rug allegations, low bot %)
- PHI: structured engagement (consistent, healthy velocity)
- VERIFY: verifiable engagement (real tweet/follower ratios, not inflated)
- CULTURE: follows ecosystem norms (active trading discussion, no spam)
- BURN: efficient marketing (good engagement rate, not wasteful spam)
- SOVEREIGNTY: decentralized community (not creator-controlled, diverse voices)
"""

from dataclasses import dataclass
from dataset_builder import TwitterSignals
import math

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


class TwitterScorer:
    """Score twitter community signals on all 6 axioms."""

    def score(self, signals: TwitterSignals) -> AxiomScores:
        """Score twitter signals.

        Key insight: Twitter reveals what on-chain metrics hide:
        - Community sentiment (positive % tells truth about quality)
        - Organizational health (tweet velocity, active discussion)
        - Red flags (rug allegations, creator criticism, spam)
        - Decentralization (diverse voices vs creator-controlled narratives)
        """

        # ── FIDELITY: Honest community (no rug allegations, no spam) ──
        fidelity = self._score_fidelity(signals)

        # ── PHI: Structured engagement (consistent activity, healthy patterns) ──
        phi = self._score_phi(signals)

        # ── VERIFY: Verifiable metrics (real engagement, not inflated followers) ──
        verify = self._score_verify(signals)

        # ── CULTURE: Follows ecosystem norms (LP discussion, no spam) ──
        culture = self._score_culture(signals)

        # ── BURN: Efficient marketing (good engagement rate) ──
        burn = self._score_burn(signals)

        # ── SOVEREIGNTY: Decentralized community (diverse voices) ──
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

    def _score_fidelity(self, s: TwitterSignals) -> float:
        """Honest community: no rug allegations, no creator criticism.

        Falsification:
        - New projects legitimately get criticized (growing pains)
        - Some FUD is inevitable for any token

        HOWL signal: <10% negative, no rug allegations, low spam
        BARK signal: >50% negative, rug allegations, heavy spam
        """
        score = PHI_BASE

        # Rug allegations are damning
        if s.has_rug_allegations:
            score -= ADJUST_LARGE
        else:
            score += ADJUST_SMALL

        # Creator criticism (second sign of dishonesty)
        if s.has_creator_criticism:
            score -= ADJUST_MEDIUM
        else:
            score += ADJUST_SMALL

        # Buy pressure spam (scam signal)
        if s.has_buy_pressure_spam:
            score -= ADJUST_MEDIUM
        else:
            score += ADJUST_SMALL

        # Negative sentiment ratio
        if s.negative_pct > 0.50:
            score -= ADJUST_MEDIUM  # Mostly negative
        elif s.negative_pct > 0.25:
            score -= ADJUST_SMALL  # Some negativity
        elif s.negative_pct < 0.10:
            score += ADJUST_SMALL  # Very positive

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_phi(self, s: TwitterSignals) -> float:
        """Structured engagement: consistent, healthy velocity patterns.

        HOWL signal: 2-10 tweets/day, consistent active discussion
        BARK signal: Either 0 (dead) or >20/day (bot-like spam)
        GROWL signal: 0.5-2 tweets/day (new or niche community)
        """
        score = PHI_BASE

        # Tweet velocity (the smoking gun for bots)
        if 2.0 <= s.tweet_velocity <= 10.0:
            score += ADJUST_LARGE  # Perfect healthy range
        elif 0.5 <= s.tweet_velocity < 2.0:
            score += ADJUST_MEDIUM  # Slower but consistent
        elif 0.0 < s.tweet_velocity < 0.5:
            score -= ADJUST_SMALL  # Very slow (zombie or niche)
        elif s.tweet_velocity > 20.0:
            score -= ADJUST_LARGE  # Bot-like spam
        elif s.tweet_velocity == 0.0:
            score -= ADJUST_LARGE  # Dead account

        # Recent activity (alive vs dormant)
        if s.tweets_last_7d > 10:
            score += ADJUST_MEDIUM  # Very active
        elif s.tweets_last_7d > 3:
            score += ADJUST_SMALL  # Somewhat active
        elif s.tweets_last_7d == 0:
            score -= ADJUST_MEDIUM  # Dormant

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_verify(self, s: TwitterSignals) -> float:
        """Verifiable engagement metrics.

        Can fake followers, but harder to fake real engagement.
        Tweet/follower ratio reveals if followers are real.
        """
        score = 0.236  # Start conservative

        # Engagement rate is the hardest metric to fake
        # Real tokens: 5-15% engagement
        # Bots: <1% or >50%
        if 0.05 <= s.engagement_rate <= 0.15:
            score += ADJUST_LARGE  # Very healthy
        elif 0.02 <= s.engagement_rate < 0.05:
            score += ADJUST_MEDIUM  # Decent
        elif 0.01 <= s.engagement_rate < 0.02:
            score += ADJUST_SMALL  # Low but real
        elif s.engagement_rate < 0.01 or s.engagement_rate > 0.50:
            score -= ADJUST_MEDIUM  # Suspicious (bot or fake)

        # Tweet/follower ratio
        # Real: ~50-100 tweets per 1000 followers
        # Bots: >500 tweets per 1000 followers or <10
        if s.follower_count > 0:
            tweet_per_1k = (s.tweet_count * 1000) / s.follower_count
            if 50 <= tweet_per_1k <= 100:
                score += ADJUST_SMALL  # Healthy ratio
            elif 10 < tweet_per_1k < 50:
                score -= ADJUST_SMALL  # Low ratio (old account?)
            elif tweet_per_1k > 500:
                score -= ADJUST_MEDIUM  # Bot-like (too many tweets)

        # Liquidity discussion is a strong positive verification signal
        if s.liquidity_discussion_active:
            score += ADJUST_SMALL

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_culture(self, s: TwitterSignals) -> float:
        """Follows ecosystem norms: LP discussion, no spam.

        HOWL signal: Active trading/LP discussion, no spam
        BARK signal: Heavy spam, no substantive discussion
        """
        score = PHI_BASE

        # Liquidity/trading discussion shows ecosystem awareness
        if s.liquidity_discussion_active:
            score += ADJUST_MEDIUM
        else:
            score -= ADJUST_SMALL

        # Spam is a red flag for ecosystem norm-breaking
        if s.has_buy_pressure_spam:
            score -= ADJUST_MEDIUM

        # Active community discussion (tweets in last 7 days)
        if s.tweets_last_7d > 5:
            score += ADJUST_SMALL

        # Positive sentiment shows healthy culture
        if s.positive_pct > 0.60:
            score += ADJUST_SMALL

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_burn(self, s: TwitterSignals) -> float:
        """Efficient marketing: good engagement, not wasteful spam.

        HOWL signal: High engagement rate, no spam spending
        BARK signal: Low engagement, heavy spam waste
        """
        score = 0.35  # BURN_BASE

        # Engagement rate = marketing efficiency
        if s.engagement_rate > 0.08:
            score += ADJUST_MEDIUM  # Efficient marketing
        elif s.engagement_rate > 0.03:
            score += ADJUST_SMALL
        elif s.engagement_rate < 0.01:
            score -= ADJUST_SMALL  # Inefficient

        # Spam is wasteful marketing spend
        if s.has_buy_pressure_spam:
            score -= ADJUST_LARGE  # Very wasteful
        else:
            score += ADJUST_SMALL

        # High tweet velocity without engagement = spam waste
        if s.tweet_velocity > 15.0 and s.engagement_rate < 0.05:
            score -= ADJUST_MEDIUM  # Spammy and inefficient

        return clamp(score, ADJUST_SMALL, PHI_INV)

    def _score_sovereignty(self, s: TwitterSignals) -> float:
        """Decentralized community: diverse voices, not creator-controlled.

        HOWL signal: Active discussion, low creator criticism (implies diverse views)
        BARK signal: Creator is primary voice or heavily criticized
        """
        score = 0.40  # SOVEREIGNTY_BASE

        # Creator criticism paradoxically signals freedom of speech
        # (community willing to criticize creator = not controlled)
        if s.has_creator_criticism:
            score += ADJUST_SMALL  # Shows diverse voices
        else:
            # No criticism could mean unified support OR controlled community
            score -= ADJUST_SMALL

        # Large follower base suggests diverse stakeholders
        if s.follower_count > 10_000:
            score += ADJUST_MEDIUM
        elif s.follower_count > 1_000:
            score += ADJUST_SMALL
        elif s.follower_count < 100:
            score -= ADJUST_SMALL

        # High tweet count (with healthy velocity) = many voices
        if s.tweet_count > 500 and 1.0 <= s.tweet_velocity <= 10.0:
            score += ADJUST_MEDIUM

        # Discussion of LP/trading = community making own decisions
        if s.liquidity_discussion_active:
            score += ADJUST_SMALL

        return clamp(score, ADJUST_SMALL, PHI_INV)


if __name__ == "__main__":
    scorer = TwitterScorer()

    # Test HOWL (healthy community)
    howl_signals = TwitterSignals(
        follower_count=50_000,
        tweet_count=2000,
        engagement_rate=0.08,
        positive_pct=0.75,
        negative_pct=0.05,
        neutral_pct=0.20,
        has_rug_allegations=False,
        has_creator_criticism=False,
        has_buy_pressure_spam=False,
        liquidity_discussion_active=True,
        tweets_last_7d=50,
        tweet_velocity=7.1,
    )
    howl_scores = scorer.score(howl_signals)
    print(f"HOWL (healthy): q_score={howl_scores.q_score:.3f} (expect >0.528)")

    # Test BARK (rug scam)
    bark_signals = TwitterSignals(
        follower_count=100,
        tweet_count=20,
        engagement_rate=0.02,
        positive_pct=0.20,
        negative_pct=0.70,
        neutral_pct=0.10,
        has_rug_allegations=True,
        has_creator_criticism=True,
        has_buy_pressure_spam=True,
        liquidity_discussion_active=False,
        tweets_last_7d=0,
        tweet_velocity=0.0,
    )
    bark_scores = scorer.score(bark_signals)
    print(f"BARK (rug): q_score={bark_scores.q_score:.3f} (expect ≤0.236)")
