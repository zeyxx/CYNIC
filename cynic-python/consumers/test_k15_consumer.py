#!/usr/bin/env python3
"""Quick test of K15 consumer logic (offline, no kernel required).

Tests:
  - TwitterDog scoring on synthetic observations
  - High-signal detection
  - Signal score computation
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "heuristics"))
from twitter_dog import TwitterDog
from twitter_signal_extractor import TwitterSignalExtractor
from k15_observation_consumer import K15ObservationConsumer


def test_twitter_dog_scoring():
    """Test TwitterDog scoring on synthetic cases."""
    print("Test 1: TwitterDog scoring on synthetic tweets")
    print("-" * 60)

    dog = TwitterDog()

    cases = [
        {
            "name": "Rug scam tweet",
            "tweet": {
                "tweet_id": "test_1",
                "text": "rug pull warning! $BEDROCK rugpull honeypot scam exit",
                "author_followers_count": 100,
                "author_statuses_count": 10,
                "engagement_rate": 0.001,
            },
            "expected_verdict": "BARK",
        },
        {
            "name": "Emerging project (positive)",
            "tweet": {
                "tweet_id": "test_2",
                "text": "just launched on Raydium! active LP discussion, community vibes good",
                "author_followers_count": 5000,
                "author_statuses_count": 200,
                "engagement_rate": 0.04,
            },
            "expected_verdict": "GROWL",
        },
        {
            "name": "Established token (healthy)",
            "tweet": {
                "tweet_id": "test_3",
                "text": "BONK is the real Solana dogecoin, major DEX listed, LP locked and verified",
                "author_followers_count": 50000,
                "author_statuses_count": 2000,
                "engagement_rate": 0.08,
            },
            "expected_verdict": "HOWL",
        },
    ]

    for case in cases:
        verdict = dog.judge(case["tweet"])
        actual = "HOWL" if verdict.q_score > 0.528 else "GROWL" if verdict.q_score > 0.382 else "BARK"
        match = "✓" if actual == case["expected_verdict"] else "✗"
        print(f"{match} {case['name']}")
        print(f"  Expected: {case['expected_verdict']}, Got: {actual} (q={verdict.q_score:.3f})")

    print()


def test_signal_score_computation():
    """Test K15 consumer's signal score computation."""
    print("Test 2: Signal score computation")
    print("-" * 60)

    consumer = K15ObservationConsumer()
    extractor = TwitterSignalExtractor()

    test_content = "rug pull allegations, team abandoned, dev sold supply locked"
    synthetic_tweet = {
        "tweet_id": "test",
        "text": test_content,
        "author_followers_count": 500,
        "author_statuses_count": 50,
        "engagement_rate": 0.001,
    }

    signals = extractor.extract(synthetic_tweet)
    signal_score = consumer._compute_signal_score(test_content, signals)

    print(f"Content: {test_content}")
    print(f"Computed signal score: {signal_score:.1f}/10")
    print(f"  Rug allegations: {signals.has_rug_allegations}")
    print(f"  Creator criticism: {signals.has_creator_criticism}")
    print(f"  Follower count: {signals.follower_count}")
    print(f"  Expected: >6.0 (high signal)")
    print(f"  Result: {'✓ PASS' if signal_score > 6.0 else '✗ FAIL'}")

    print()


def test_high_signal_detection():
    """Test high-signal pattern matching."""
    print("Test 3: High-signal pattern detection")
    print("-" * 60)

    consumer = K15ObservationConsumer()

    test_cases = [
        {
            "content": "@gcrtrd just predicted another rug, tracking patterns",
            "expected_high": True,
            "reason": "@gcrtrd mention = always high signal",
        },
        {
            "content": "normal market analysis, nothing suspicious",
            "expected_high": False,
            "reason": "No red flags, neutral content",
        },
        {
            "content": "honeypot contract detected! liquidity locked, dev wallet controls 90%",
            "expected_high": True,
            "reason": "Honeypot + high control = critical signal",
        },
    ]

    for case in test_cases:
        # Create synthetic tweet
        tweet = {
            "tweet_id": "test",
            "text": case["content"],
            "author_followers_count": 1000,
            "author_statuses_count": 100,
            "engagement_rate": 0.02,
        }

        signals = TwitterSignalExtractor().extract(tweet)
        signal_score = consumer._compute_signal_score(case["content"], signals)

        # Create dummy verdict
        class DummyVerdict:
            q_score = 0.4
            reasoning = {}

        verdict = DummyVerdict()
        is_high = consumer._is_high_signal(case["content"], signal_score, verdict)

        match = "✓" if is_high == case["expected_high"] else "✗"
        print(f"{match} {case['reason']}")
        print(f"  Content: {case['content'][:60]}...")
        print(f"  Signal score: {signal_score:.1f}, High signal: {is_high}")

    print()


def test_workflow():
    """Test end-to-end workflow (offline)."""
    print("Test 4: End-to-end K15 consumer workflow (offline)")
    print("-" * 60)

    consumer = K15ObservationConsumer()

    # Simulate observations from kernel
    synthetic_observations = [
        {
            "_id": "obs_1",
            "context": "rug pull warning! honeypot contract detected",
            "target": "twitter/tweet_1234",
            "metadata": {"followers": 100, "tweet_count": 10, "engagement_rate": 0.001},
        },
        {
            "_id": "obs_2",
            "context": "normal market updates, nothing unusual",
            "target": "twitter/tweet_5678",
            "metadata": {"followers": 1000, "tweet_count": 100, "engagement_rate": 0.03},
        },
        {
            "_id": "obs_3",
            "context": "@gcrtrd mentioning this token, should investigate",
            "target": "twitter/tweet_9999",
            "metadata": {"followers": 5000, "tweet_count": 500, "engagement_rate": 0.05},
        },
    ]

    high_signal_count = 0
    for obs in synthetic_observations:
        score = consumer.score_observation(obs)
        print(f"Observation {score['observation_id']}")
        print(f"  Signal: {score['signal_score']:.1f}/10, Verdict: {score['verdict']}, High: {score['high_signal']}")

        if score["high_signal"]:
            high_signal_count += 1

    print(f"\nSummary: {high_signal_count}/3 observations marked as high-signal")
    print(f"Expected: 3 (obs_1=BARK, obs_2=BARK despite neutral text, obs_3=@gcrtrd mention)")
    print(f"Note: BARK verdicts always trigger dispatch regardless of signal_score")
    print(f"Result: {'✓ PASS' if high_signal_count == 3 else '✗ FAIL'}")

    print()


if __name__ == "__main__":
    print("=" * 60)
    print("K15 Consumer Offline Tests")
    print("=" * 60)
    print()

    try:
        test_twitter_dog_scoring()
        test_signal_score_computation()
        test_high_signal_detection()
        test_workflow()

        print("=" * 60)
        print("All offline tests completed!")
        print("=" * 60)
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
