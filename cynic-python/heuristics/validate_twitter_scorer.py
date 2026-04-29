"""Validate TwitterScorer against known signal patterns.

Ground truth:
- HOWL: established projects with healthy communities
- BARK: known rug/scams, bot activity, spam
- GROWL: emerging projects or controversial signals
"""

from twitter_heuristics import TwitterScorer
from dataset_builder import TwitterSignals


def test_healthy_established_token():
    """HOWL case: established token with healthy metrics.

    Example: BONK (Solana's dogecoin, 1.2M holders, major DEX)
    """
    signals = TwitterSignals(
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

    scorer = TwitterScorer()
    scores = scorer.score(signals)

    print("=" * 70)
    print("TEST 1: HOWL (Healthy Established Token)")
    print("-" * 70)
    print(f"  Scenario: BONK-like (50K followers, active LP discussion)")
    print(f"  Q-Score: {scores.q_score:.3f} (expect >0.528 for HOWL)")
    print(f"  Breakdown:")
    print(f"    FIDELITY:   {scores.fidelity:.3f} (honest, no rug allegations)")
    print(f"    PHI:        {scores.phi:.3f} (healthy velocity: {signals.tweet_velocity:.1f} tweets/day)")
    print(f"    VERIFY:     {scores.verify:.3f} (high engagement: {signals.engagement_rate:.1%})")
    print(f"    CULTURE:    {scores.culture:.3f} (LP discussion active)")
    print(f"    BURN:       {scores.burn:.3f} (efficient marketing)")
    print(f"    SOVEREIGNTY:{scores.sovereignty:.3f} (large diverse community)")

    verdict = "HOWL ✓" if scores.q_score > 0.528 else "UNEXPECTED"
    print(f"  Verdict: {verdict}")
    return scores.q_score > 0.528


def test_rug_scam():
    """BARK case: known rug scam patterns.

    Example: Dead account with rug allegations, low engagement, bot-like velocity
    """
    signals = TwitterSignals(
        follower_count=1000,  # Low followers
        tweet_count=50,  # Few tweets
        engagement_rate=0.01,  # Low engagement (bot-like)
        positive_pct=0.10,
        negative_pct=0.80,
        neutral_pct=0.10,
        has_rug_allegations=True,  # CRITICAL: rug mentions
        has_creator_criticism=True,
        has_buy_pressure_spam=True,
        liquidity_discussion_active=False,
        tweets_last_7d=0,  # Dead (no recent activity)
        tweet_velocity=0.0,
    )

    scorer = TwitterScorer()
    scores = scorer.score(signals)

    print("\n" + "=" * 70)
    print("TEST 2: BARK (Rug Scam)")
    print("-" * 70)
    print(f"  Scenario: Rug scam ($BEDROCK-like)")
    print(f"  Q-Score: {scores.q_score:.3f} (expect ≤0.236 for BARK)")
    print(f"  Breakdown:")
    print(f"    FIDELITY:   {scores.fidelity:.3f} (rug allegations present!)")
    print(f"    PHI:        {scores.phi:.3f} (dead account: {signals.tweet_velocity:.1f} tweets/day)")
    print(f"    VERIFY:     {scores.verify:.3f} (low engagement: {signals.engagement_rate:.1%})")
    print(f"    CULTURE:    {scores.culture:.3f} (spam, no LP discussion)")
    print(f"    BURN:       {scores.burn:.3f} (wasteful spam spend)")
    print(f"    SOVEREIGNTY:{scores.sovereignty:.3f} (creator attacked, no trust)")

    verdict = "BARK ✓" if scores.q_score <= 0.236 else "UNEXPECTED"
    print(f"  Verdict: {verdict}")
    return scores.q_score <= 0.236


def test_emerging_project():
    """GROWL case: emerging project with mixed signals.

    Example: New token, active community discussion, some skepticism
    """
    signals = TwitterSignals(
        follower_count=5_000,  # Medium followers (growing)
        tweet_count=200,
        engagement_rate=0.04,  # Moderate engagement
        positive_pct=0.50,
        negative_pct=0.20,
        neutral_pct=0.30,
        has_rug_allegations=False,
        has_creator_criticism=False,  # No major criticism yet
        has_buy_pressure_spam=False,
        liquidity_discussion_active=True,  # Active LP discussion
        tweets_last_7d=10,  # Active
        tweet_velocity=1.5,  # Moderate velocity
    )

    scorer = TwitterScorer()
    scores = scorer.score(signals)

    print("\n" + "=" * 70)
    print("TEST 3: GROWL (Emerging Project)")
    print("-" * 70)
    print(f"  Scenario: New project, active but unproven")
    print(f"  Q-Score: {scores.q_score:.3f} (expect 0.236 < score < 0.528)")
    print(f"  Breakdown:")
    print(f"    FIDELITY:   {scores.fidelity:.3f} (no red flags yet)")
    print(f"    PHI:        {scores.phi:.3f} (moderate velocity: {signals.tweet_velocity:.1f} tweets/day)")
    print(f"    VERIFY:     {scores.verify:.3f} (moderate engagement: {signals.engagement_rate:.1%})")
    print(f"    CULTURE:    {scores.culture:.3f} (LP discussion + some skepticism)")
    print(f"    BURN:       {scores.burn:.3f} (reasonable marketing spend)")
    print(f"    SOVEREIGNTY:{scores.sovereignty:.3f} (emerging community)")

    verdict = "GROWL ✓" if 0.236 < scores.q_score < 0.528 else "UNEXPECTED"
    print(f"  Verdict: {verdict}")
    return 0.236 < scores.q_score < 0.528


def test_recovery_scam():
    """BARK variant: recovery scam (tweets about "recovery services").

    Falsifies: high-signal tweets discussing scam patterns (should score low)
    """
    signals = TwitterSignals(
        follower_count=100,  # Very low
        tweet_count=10,
        engagement_rate=0.001,  # Minimal
        positive_pct=0.05,
        negative_pct=0.90,  # Mostly negative (discussing scam)
        neutral_pct=0.05,
        has_rug_allegations=True,  # Mentions of "recovery" scams
        has_creator_criticism=True,
        has_buy_pressure_spam=False,
        liquidity_discussion_active=False,
        tweets_last_7d=0,
        tweet_velocity=0.1,
    )

    scorer = TwitterScorer()
    scores = scorer.score(signals)

    print("\n" + "=" * 70)
    print("TEST 4: BARK variant (Recovery Scam Pattern)")
    print("-" * 70)
    print(f"  Scenario: Tweet discussing recovery scam (@gcrtrd signal pattern)")
    print(f"  Q-Score: {scores.q_score:.3f} (expect ≤0.236)")
    print(f"  Note: High-signal content (discussing scams) should score LOW")
    print(f"        because it indicates fraud/victim discussion, not endorsement")

    verdict = "BARK ✓" if scores.q_score <= 0.236 else "UNEXPECTED"
    print(f"  Verdict: {verdict}")
    return scores.q_score <= 0.236


if __name__ == "__main__":
    results = [
        test_healthy_established_token(),
        test_rug_scam(),
        test_emerging_project(),
        test_recovery_scam(),
    ]

    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    passed = sum(results)
    total = len(results)
    print(f"TwitterScorer validation: {passed}/{total} tests passed")

    if passed == total:
        print("✓ TwitterScorer correctly discriminates HOWL/GROWL/BARK patterns")
        print("✓ Ready to deploy twitter-domain judgment to Dogs")
    else:
        print("✗ TwitterScorer needs tuning before production")
