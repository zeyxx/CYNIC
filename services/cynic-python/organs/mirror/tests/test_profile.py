"""Tests for BehavioralProfile data model and serialization."""
import json
from datetime import datetime, timezone

from organs.mirror.profile import (
    BehavioralProfile,
    Distribution,
    Feature,
    Tension,
    TimeWindow,
    feature_confidence,
)

PHI_INV = 0.618034


def test_empty_profile_creation() -> None:
    profile = BehavioralProfile.empty()
    assert profile.observation_count == 0
    assert profile.profile_version == "0.1.0"
    assert len(profile.activity_hours) == 0
    assert len(profile.narrative_affinity) == 0


def test_profile_roundtrip_json() -> None:
    profile = BehavioralProfile.empty()
    profile.observation_count = 42
    profile.activity_hours = {21: 0.35, 22: 0.28}
    profile.narrative_affinity = {"agent": 0.7, "defi": 0.3}

    json_str = profile.to_json()
    restored = BehavioralProfile.from_json(json_str)

    assert restored.observation_count == 42
    assert restored.activity_hours == {21: 0.35, 22: 0.28}
    assert restored.narrative_affinity == {"agent": 0.7, "defi": 0.3}


def test_feature_confidence_low_observations() -> None:
    c = feature_confidence(n_observations=5, stability=1.0)
    assert 0.08 < c < 0.10


def test_feature_confidence_high_observations() -> None:
    c = feature_confidence(n_observations=200, stability=0.9)
    assert 0.50 < c < PHI_INV


def test_feature_confidence_never_exceeds_phi_inv() -> None:
    c = feature_confidence(n_observations=1_000_000, stability=1.0)
    assert c <= PHI_INV


def test_feature_confidence_zero_stability() -> None:
    c = feature_confidence(n_observations=1000, stability=0.0)
    assert c == 0.0


def test_distribution_from_values() -> None:
    d = Distribution.from_values([1.0, 2.0, 3.0, 4.0, 5.0])
    assert d.n == 5
    assert d.mean == 3.0
    assert 1.4 < d.std < 1.5
    assert d.p50 == 3.0
