"""Layer 4: ANALYSE — Extract patterns."""
from collections import Counter
from .schema import CleanedData, PatternAnalysis, DomainMetric

class DataAnalyzer:
    def analyze(self, cleaned: CleanedData, cycle: int) -> PatternAnalysis:
        # Tweet analysis
        high_signal = [t for t in cleaned.tweets_valid if t.signal_score and t.signal_score > 0]
        scores = [t.signal_score for t in cleaned.tweets_valid if t.signal_score]
        avg_signal = sum(scores) / len(scores) if scores else 0

        # Verdict analysis
        by_domain = {}
        for v in cleaned.verdicts_valid:
            if v.domain not in by_domain:
                by_domain[v.domain] = []
            by_domain[v.domain].append(v)

        domain_metrics = {}
        for domain, vs in by_domain.items():
            avg_score = sum(v.q_score for v in vs) / len(vs)
            howls = sum(1 for v in vs if v.verdict_type == "HOWL")
            barks = sum(1 for v in vs if v.verdict_type == "BARK")
            growls = sum(1 for v in vs if v.verdict_type == "GROWL")
            wags = sum(1 for v in vs if v.verdict_type == "WAG")
            confidence = min(0.95, 0.618 + max(0, len(vs) - 5) * 0.01)
            domain_metrics[domain] = DomainMetric(
                domain=domain,
                verdict_count=len(vs),
                avg_q_score=avg_score,
                howl_count=howls,
                bark_count=barks,
                growl_count=growls,
                wag_count=wags,
                confidence=confidence,
            )

        # Session analysis
        intents = Counter(s.intent for s in cleaned.sessions_valid)
        dominant_intent = intents.most_common(1)[0][0] if intents else "mixed"
        dominant_type = Counter(v.verdict_type for v in cleaned.verdicts_valid).most_common(1)[0][0] if cleaned.verdicts_valid else "unknown"

        # Anomalies
        anomalies = []
        if cleaned.tweets_dropped > len(cleaned.tweets_valid):
            anomalies.append("high_tweet_dropout")
        if cleaned.verdicts_dropped > len(cleaned.verdicts_valid):
            anomalies.append("high_verdict_dropout")

        # Opportunities
        opportunities = []
        if high_signal:
            pct = 100 * len(high_signal) / len(cleaned.tweets_valid)
            opportunities.append(f"{pct:.1f}% of tweets are high-signal")

        return PatternAnalysis(
            timestamp=cleaned.timestamp,
            cycle=cycle,
            tweets_analyzed=len(cleaned.tweets_valid),
            high_signal_tweets=len(high_signal),
            avg_signal_score=avg_signal,
            verdicts_analyzed=len(cleaned.verdicts_valid),
            domain_metrics=domain_metrics,
            dominant_verdict_type=dominant_type,
            sessions_analyzed=len(cleaned.sessions_valid),
            dominant_intent=dominant_intent,
            anomalies=anomalies,
            opportunities=opportunities,
            recommendation="Monitor signal trends. Focus on high-signal tweets.",
        )
