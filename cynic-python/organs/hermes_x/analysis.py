"""
Layer 4: ANALYSE & COMPRÉHENSION — Extract patterns via Gemini reasoning.

Inputs: CleanedData
Outputs: PatternAnalysis (typed contract)

This layer dispatches work to Gemini CLI to:
- Identify patterns in cleaned data
- Detect anomalies
- Extract opportunities
- Formulate recommendations
"""

import json
import subprocess
from pathlib import Path
from datetime import datetime
from collections import Counter
from .schema import CleanedData, PatternAnalysis, DomainMetric


class DataAnalyzer:
    """Analyze cleaned data using local heuristics"""

    @staticmethod
    def analyze_verdicts(verdicts: list, cycle: int) -> dict:
        """
        Analyze verdict patterns.

        Metrics:
        - Count by domain
        - Average q_score by domain
        - Count by verdict type
        - Confidence (φ⁻¹ = 0.618)
        """
        domain_counts = Counter()
        domain_scores = {}
        verdict_types = Counter()

        for verdict in verdicts:
            domain = verdict.domain
            domain_counts[domain] += 1
            verdict_types[verdict.verdict_type] += 1

            if domain not in domain_scores:
                domain_scores[domain] = []
            domain_scores[domain].append(verdict.q_score)

        # Compute per-domain metrics
        domain_metrics = {}
        for domain in domain_counts:
            scores = domain_scores.get(domain, [])
            avg_score = sum(scores) / len(scores) if scores else 0.0

            # Count by verdict type
            howls = sum(1 for v in verdicts if v.domain == domain and v.verdict_type == "HOWL")
            barks = sum(1 for v in verdicts if v.domain == domain and v.verdict_type == "BARK")
            growls = sum(1 for v in verdicts if v.domain == domain and v.verdict_type == "GROWL")
            wags = sum(1 for v in verdicts if v.domain == domain and v.verdict_type == "WAG")

            # Confidence: based on sample size
            sample_size = domain_counts[domain]
            if sample_size >= 20:
                confidence = min(0.95, 0.618 + (sample_size - 20) * 0.01)
            elif sample_size >= 5:
                confidence = 0.618
            else:
                confidence = 0.3 + (sample_size * 0.05)

            domain_metrics[domain] = DomainMetric(
                domain=domain,
                verdict_count=domain_counts[domain],
                avg_q_score=avg_score,
                howl_count=howls,
                bark_count=barks,
                growl_count=growls,
                wag_count=wags,
                confidence=confidence,
            )

        # Find dominant verdict type
        dominant = verdict_types.most_common(1)[0][0] if verdict_types else "unknown"

        return {
            "verdict_count": len(verdicts),
            "domain_metrics": domain_metrics,
            "dominant_type": dominant,
        }

    @staticmethod
    def analyze_tweets(tweets: list) -> dict:
        """
        Analyze tweet patterns.

        Metrics:
        - Count of high-signal tweets (score > 0)
        - Average signal score
        """
        high_signal = [t for t in tweets if t.signal_score is not None and t.signal_score > 0]
        scores = [t.signal_score for t in tweets if t.signal_score is not None]
        avg_score = sum(scores) / len(scores) if scores else 0.0

        return {
            "tweet_count": len(tweets),
            "high_signal_count": len(high_signal),
            "avg_signal_score": avg_score,
        }

    @staticmethod
    def analyze_sessions(sessions: list) -> dict:
        """
        Analyze Hermes agent behavior patterns.

        Metrics:
        - Agent decision count
        - Dominant domain focus
        """
        intents = Counter(s.intent for s in sessions)
        dominant = intents.most_common(1)[0][0] if intents else "mixed"

        return {
            "session_count": len(sessions),  # Now agent decisions, not sessions
            "dominant_intent": dominant,  # Now dominant domain (token/wallet/social/security)
        }

    def analyze(self, cleaned_data: CleanedData, cycle: int) -> PatternAnalysis:
        """
        Analyze all cleaned data and extract patterns.

        Returns:
            PatternAnalysis with metrics and anomalies
        """
        tweet_analysis = self.analyze_tweets(cleaned_data.tweets_valid)
        verdict_analysis = self.analyze_verdicts(cleaned_data.verdicts_valid, cycle)
        session_analysis = self.analyze_sessions(cleaned_data.sessions_valid)

        # Detect anomalies
        anomalies = []
        if cleaned_data.tweets_dropped > len(cleaned_data.tweets_valid):
            anomalies.append("high_tweet_dropout")
        if cleaned_data.verdicts_dropped > len(cleaned_data.verdicts_valid):
            anomalies.append("high_verdict_dropout")
        if tweet_analysis["avg_signal_score"] < -1.0:
            anomalies.append("negative_signal_trend")

        # Opportunities
        opportunities = []
        if tweet_analysis["high_signal_count"] > 0:
            pct = 100 * tweet_analysis["high_signal_count"] / len(cleaned_data.tweets_valid)
            opportunities.append(f"{pct:.1f}% of tweets are high-signal")
        if len(verdict_analysis["domain_metrics"]) > 1:
            opportunities.append(f"{len(verdict_analysis['domain_metrics'])} domains active")

        # Recommendation
        recommendation = "Monitor signal trends and verdict confidence. Focus on high-signal tweets."

        return PatternAnalysis(
            timestamp=cleaned_data.timestamp,
            cycle=cycle,
            tweets_analyzed=tweet_analysis["tweet_count"],
            high_signal_tweets=tweet_analysis["high_signal_count"],
            avg_signal_score=tweet_analysis["avg_signal_score"],
            verdicts_analyzed=verdict_analysis["verdict_count"],
            domain_metrics=verdict_analysis["domain_metrics"],
            dominant_verdict_type=verdict_analysis["dominant_type"],
            sessions_analyzed=session_analysis["session_count"],
            dominant_intent=session_analysis["dominant_intent"],
            anomalies=anomalies,
            opportunities=opportunities,
            recommendation=recommendation,
            gemini_reasoning=None,  # Will be populated if Gemini is used
        )


class GeminiAnalyzer:
    """Dispatch analysis to Gemini CLI for reasoning (future enhancement)"""

    @staticmethod
    def dispatch_to_gemini(cleaned_data: CleanedData, cycle: int) -> str:
        """
        Dispatch cleaned data to Gemini CLI for pattern analysis.

        Returns the Gemini reasoning as a string.
        """
        # Build a prompt with the data
        prompt = f"""
You are analyzing data from CYNIC, a system that judges tokens and social signals.

Cycle: {cycle}
Timestamp: {cleaned_data.timestamp}

DATA SUMMARY:
- Tweets analyzed: {len(cleaned_data.tweets_valid)} (dropped: {cleaned_data.tweets_dropped})
- Verdicts analyzed: {len(cleaned_data.verdicts_valid)} (dropped: {cleaned_data.verdicts_dropped})
- Sessions analyzed: {len(cleaned_data.sessions_valid)} (dropped: {cleaned_data.sessions_dropped})

TWEETS SAMPLE (first 5):
{json.dumps([{'text': t.text[:100], 'signal': t.signal_score} for t in cleaned_data.tweets_valid[:5]], indent=2)}

VERDICTS SAMPLE (first 5):
{json.dumps([{'domain': v.domain, 'q_score': v.q_score, 'type': v.verdict_type} for v in cleaned_data.verdicts_valid[:5]], indent=2)}

TASK:
1. Identify key patterns in the data
2. Detect anomalies or unusual trends
3. Suggest opportunities for improvement
4. Formulate a recommendation for the next cycle

Keep your response concise and actionable.
"""

        try:
            # Call Gemini CLI
            result = subprocess.run(
                ["gemini", "chat", "-q", prompt],
                capture_output=True,
                text=True,
                timeout=30,
            )

            if result.returncode == 0:
                return result.stdout.strip()
            else:
                return f"Gemini error: {result.stderr}"

        except FileNotFoundError:
            return "Gemini CLI not available"
        except subprocess.TimeoutExpired:
            return "Gemini timeout"
        except Exception as e:
            return f"Gemini exception: {str(e)}"
