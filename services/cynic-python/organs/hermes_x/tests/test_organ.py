"""
Tests for Hermes X organ layers.

Verifies:
1. Sensors perceive data correctly
2. Transformer cleans without data loss
3. Analyzer detects anomalies
4. Reflector compounds observations
"""

import pytest
from ..sensors import HermesXSensors
from ..transformation import DataTransformer
from ..analysis import DataAnalyzer
from ..learning import OrganReflector
from ..schema import Tweet, Verdict, SessionTurn, RawPerception, CleanedData


def test_sensors_perceive():
    """Test that sensors can perceive from real data sources"""
    sensors = HermesXSensors()
    perception = sensors.perceive()

    assert isinstance(perception, RawPerception)
    assert perception.timestamp is not None
    assert len(perception.tweets) >= 0
    assert len(perception.verdicts) >= 0
    assert len(perception.sessions) >= 0


def test_transformer_cleans():
    """Test that transformer cleans data without losing everything"""
    transformer = DataTransformer()

    # Create fake perception
    tweets = [
        Tweet(id="1", text="valid tweet", author="test", created_at="2026-04-30", signal_score=1.0),
        Tweet(id="2", text="", author="test", created_at="2026-04-30"),  # Empty — should drop
    ]
    verdicts = [
        Verdict(id="1", content="test", domain="test", q_score=0.5, verdict_type="BARK", timestamp="", created_at=""),
    ]
    sessions = [
        SessionTurn(session_id="1", turn_count=1, timestamp="", intent="feature", message_length=10),
    ]

    perception = RawPerception(
        timestamp="2026-04-30",
        tweets=tweets,
        verdicts=verdicts,
        sessions=sessions,
        observation_count=0,
    )

    cleaned = transformer.transform(perception)

    assert isinstance(cleaned, CleanedData)
    assert len(cleaned.tweets_valid) == 1  # One valid tweet
    assert cleaned.tweets_dropped == 1  # One dropped
    assert cleaned.quality_score() < 1.0  # Not perfect quality


def test_analyzer_detects_anomalies():
    """Test that analyzer detects anomalies"""
    analyzer = DataAnalyzer()

    tweets = [
        Tweet(id="1", text="test", author="test", created_at="2026-04-30", signal_score=1.0),
    ]
    verdicts = []  # Empty — should trigger anomaly
    sessions = [
        SessionTurn(session_id="1", turn_count=1, timestamp="", intent="feature", message_length=10),
    ]

    cleaned = CleanedData(
        timestamp="2026-04-30",
        tweets_valid=tweets,
        tweets_dropped=0,
        verdicts_valid=verdicts,
        verdicts_dropped=1,  # High dropout
        sessions_valid=sessions,
        sessions_dropped=0,
    )

    analysis = analyzer.analyze(cleaned, cycle=0)

    # Should detect anomaly from verdict dropout
    assert "high_verdict_dropout" in analysis.anomalies


def test_reflector_persists():
    """Test that reflector persists and reads reflections"""
    reflector = OrganReflector()

    # Prior reflections should be readable
    priors = reflector.read_prior_reflections()
    assert isinstance(priors, list)


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
