#!/usr/bin/env python3
"""
Domain Verdict Builder — Layer 2 (May 3–5, 2026)

For each of the 28 tokens from conviction demo:
1. Scrape social mentions (Twitter, pump.fun, Telegram if available)
2. Classify sentiment: SAFE / HYPE / RUG_WARNING / UNKNOWN
3. Map sentiment to domain verdict: Howl / Growl / Bark
4. Compare conviction verdict vs domain verdict
5. Identify mismatches for fusion layer

Usage:
    python3 domain_verdict_builder.py
    python3 domain_verdict_builder.py --output domain_verdicts.json
"""

import json
import sys
from pathlib import Path
from typing import Dict, List, Optional
from dataclasses import dataclass, asdict
from enum import Enum


class SentimentTier(Enum):
    """Sentiment classification from social signals."""
    SAFE = "safe"  # Positive mentions, community trust
    HYPE = "hype"  # Mixed, speculative, trendy
    RUG_WARNED = "rug_warned"  # Rug warnings, exit scams, honeypot signals
    UNKNOWN = "unknown"  # No signal, insufficient data


class DomainVerdict(Enum):
    """Domain-derived verdict from sentiment."""
    Howl = "Howl"  # Safe signal: community trust, transparency
    Growl = "Growl"  # Mixed signal: hype + some warnings
    Bark = "Bark"  # Rug signal: warnings > trust


@dataclass
class DomainSignal:
    """Social signal from a single source."""
    source: str  # "twitter", "pump.fun", "telegram", etc.
    sentiment: SentimentTier
    mentions_count: int
    confidence: float  # 0.0-1.0, how confident is this signal?
    sample_text: Optional[str] = None


@dataclass
class TokenDomainVerdictRecord:
    """Complete domain verdict for one token."""
    mint: str
    symbol: str
    conviction_verdict: str  # From conviction layer (Howl/Growl/Bark)
    domain_signals: List[DomainSignal]  # Multiple sources
    domain_sentiment: SentimentTier  # Aggregated sentiment
    domain_verdict: str  # DomainVerdict (Howl/Growl/Bark)
    alignment: bool  # conviction_verdict == domain_verdict
    alignment_confidence: float  # How confident is the alignment?


def load_conviction_tokens(file_path: str = "cynic-python/video_demo_tokens.json") -> List[Dict]:
    """Load the 28 conviction-scored tokens."""
    path = Path(file_path)
    if not path.exists():
        print(f"❌ File not found: {file_path}")
        return []

    with open(path) as f:
        return json.load(f)


def sentiment_to_verdict(sentiment: SentimentTier) -> str:
    """Map sentiment tier to domain verdict."""
    mapping = {
        SentimentTier.SAFE: DomainVerdict.Howl.value,
        SentimentTier.HYPE: DomainVerdict.Growl.value,
        SentimentTier.RUG_WARNED: DomainVerdict.Bark.value,
        SentimentTier.UNKNOWN: DomainVerdict.Growl.value,  # Default to uncertain
    }
    return mapping[sentiment]


def aggregate_sentiment(signals: List[DomainSignal]) -> SentimentTier:
    """Aggregate multiple signals into a single sentiment tier.

    Weighted by confidence. If any signal is RUG_WARNED with high confidence, that wins.
    Otherwise, majority vote with confidence weighting.
    """
    if not signals:
        return SentimentTier.UNKNOWN

    # Check for high-confidence rug warnings (veto)
    for signal in signals:
        if signal.sentiment == SentimentTier.RUG_WARNED and signal.confidence >= 0.7:
            return SentimentTier.RUG_WARNED

    # Weighted voting
    weights = {
        SentimentTier.SAFE: sum(s.confidence for s in signals if s.sentiment == SentimentTier.SAFE),
        SentimentTier.HYPE: sum(s.confidence for s in signals if s.sentiment == SentimentTier.HYPE),
        SentimentTier.RUG_WARNED: sum(s.confidence for s in signals if s.sentiment == SentimentTier.RUG_WARNED),
    }

    # Find highest-weighted sentiment
    if not any(weights.values()):
        return SentimentTier.UNKNOWN

    max_sentiment = max(weights, key=weights.get)
    return max_sentiment


def check_alignment(conviction_verdict: str, domain_verdict: str) -> bool:
    """Check if conviction and domain verdicts agree."""
    return conviction_verdict == domain_verdict


def narrative_to_sentiment(narrative: str) -> SentimentTier:
    """Map tweet narrative tags to sentiment tiers."""
    narrative_lower = narrative.lower()

    # Rug warning indicators (highest priority)
    if any(x in narrative_lower for x in ["rug", "scam", "warning", "honeypot", "exit_scam"]):
        return SentimentTier.RUG_WARNED

    # Hype indicators
    if any(x in narrative_lower for x in ["hype", "pump", "trend", "moon"]):
        return SentimentTier.HYPE

    # Safe/positive indicators (community, analysis, ecosystem)
    if any(x in narrative_lower for x in ["analysis", "ecosystem", "community", "onchain"]):
        return SentimentTier.SAFE

    return SentimentTier.UNKNOWN


def load_organ_x_twitter_data(tokens: List[Dict]) -> Dict[str, List[Dict]]:
    """Load Organ X Twitter analysis data and map mentions to tokens.

    Returns dict: {symbol: [list of relevant tweets with narratives]}
    """
    organ_x_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "analysis_findings.json"

    if not organ_x_path.exists():
        print(f"⚠️  Organ X data not found at {organ_x_path}")
        return {}

    with open(organ_x_path) as f:
        findings = json.load(f)

    # Build symbol lookup
    token_symbols = {t["symbol"].strip(): t for t in tokens}
    token_mentions = {symbol: [] for symbol in token_symbols}

    # Search through high-signal tweets for token mentions
    for tweet in findings.get("high_signal_tweets", []):
        text = tweet["text"].lower()
        for symbol in token_symbols:
            symbol_lower = symbol.lower()
            # Search for $SYMBOL pattern
            if f"${symbol_lower}" in text:
                token_mentions[symbol].append(tweet)

    return token_mentions


def build_domain_verdicts_from_organ_x(tokens: List[Dict]) -> List[Dict]:
    """Build domain verdicts using Organ X Twitter data.

    Processes real tweet mentions from Organ X with narrative classifications.
    """
    # Load Twitter mention data for each token
    token_mentions = load_organ_x_twitter_data(tokens)

    records = []

    for token in tokens:
        symbol = token["symbol"].strip()
        conviction_verdict = token["verdict"]

        # Get tweets mentioning this token
        mentions = token_mentions.get(symbol, [])

        # Build signals from tweet narratives
        signals: List[DomainSignal] = []
        sentiment_counts = {tier: 0.0 for tier in SentimentTier}

        if mentions:
            for tweet in mentions:
                narratives = tweet.get("narratives", [])

                if narratives:
                    # Map narratives to sentiments
                    for narrative in narratives:
                        sentiment = narrative_to_sentiment(narrative)
                        sentiment_counts[sentiment] += 1
                else:
                    # No narratives — treat as neutral/hype (observed mentions)
                    sentiment_counts[SentimentTier.HYPE] += 0.5

            # Create signals from aggregated counts
            total = sum(sentiment_counts.values())
            for sentiment, count in sentiment_counts.items():
                if count > 0:
                    confidence = min(count / total, 1.0)  # Normalize confidence
                    signals.append(
                        DomainSignal(
                            source="organ_x_twitter",
                            sentiment=sentiment,
                            mentions_count=len(mentions),
                            confidence=confidence,
                            sample_text=f"[{len(mentions)} tweets mentioning ${symbol}]",
                        )
                    )
        else:
            # No mentions — unknown
            signals.append(
                DomainSignal(
                    source="organ_x_twitter",
                    sentiment=SentimentTier.UNKNOWN,
                    mentions_count=0,
                    confidence=0.0,
                    sample_text=f"[No mentions of ${symbol} in Organ X]",
                )
            )

        # Aggregate sentiment
        aggregated_sentiment = aggregate_sentiment(signals)
        domain_verdict = sentiment_to_verdict(aggregated_sentiment)

        # Check alignment
        alignment = check_alignment(conviction_verdict, domain_verdict)
        alignment_confidence = 0.7 if len(mentions) >= 3 else 0.4

        record = TokenDomainVerdictRecord(
            mint=token["mint"],
            symbol=symbol,
            conviction_verdict=conviction_verdict,
            domain_signals=signals,
            domain_sentiment=aggregated_sentiment,
            domain_verdict=domain_verdict,
            alignment=alignment,
            alignment_confidence=alignment_confidence,
        )
        records.append(record)

    return records


def print_summary(records: List[TokenDomainVerdictRecord]) -> None:
    """Print summary of domain verdict analysis."""
    aligned = sum(1 for r in records if r.alignment)
    misaligned = len(records) - aligned

    print("=" * 80)
    print("DOMAIN VERDICT ANALYSIS (Organ X Twitter Integration)")
    print("=" * 80)
    print()

    print("📊 ALIGNMENT STATS")
    print("-" * 80)
    print(f"Total tokens: {len(records)}")
    print(f"Aligned (conviction ↔ domain): {aligned} ({aligned/len(records)*100:.1f}%)")
    print(f"Misaligned: {misaligned} ({misaligned/len(records)*100:.1f}%)")
    print()

    if misaligned > 0:
        print("⚠️  MISALIGNED TOKENS (Conviction ≠ Domain)")
        print("-" * 80)
        for r in records:
            if not r.alignment:
                print(f"  ${r.symbol:10s} | Conviction: {r.conviction_verdict:6s} | Domain: {r.domain_verdict:6s} | Reason: {r.domain_sentiment.value}")
        print()

    print("📋 VERDICT DISTRIBUTION (Domain Layer)")
    print("-" * 80)
    for verdict in ["Howl", "Growl", "Bark"]:
        count = sum(1 for r in records if r.domain_verdict == verdict)
        pct = count / len(records) * 100 if records else 0
        print(f"  {verdict:6s}: {count:3d} tokens ({pct:5.1f}%)")
    print()

    print("=" * 80)
    print("📊 NEXT STEP: Compound with on-chain signals")
    print("   - Layer Helius wallet authenticity scoring (getTokenLargestAccounts)")
    print("   - Add pump.fun holder concentration metrics")
    print("   - Fuse conviction + domain + on-chain into final verdict")
    print("=" * 80)


def main():
    print(f"📊 Loading conviction tokens...")
    tokens = load_conviction_tokens()
    if not tokens:
        return 1

    print(f"✓ Loaded {len(tokens)} tokens\n")

    print(f"🔍 Building domain verdicts (Organ X Twitter layer)...")
    records = build_domain_verdicts_from_organ_x(tokens)
    print(f"✓ Built {len(records)} domain verdict records\n")

    print_summary(records)

    # Save results
    output_file = Path("cynic-python/domain_verdicts_stub.json")
    with open(output_file, "w") as f:
        # Serialize records with enum values
        serializable = [
            {
                **asdict(r),
                "domain_sentiment": r.domain_sentiment.value,
                "domain_signals": [
                    {
                        **asdict(sig),
                        "sentiment": sig.sentiment.value,
                    }
                    for sig in r.domain_signals
                ],
            }
            for r in records
        ]
        json.dump(serializable, f, indent=2)

    print(f"✓ Domain verdicts saved to: {output_file}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
