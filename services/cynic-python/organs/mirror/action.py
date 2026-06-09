"""ActionGate — confidence-gated decision between observing and acting.

Tier 2 INFRASTRUCTURE: Action gate for mirror organ dispatch.

K15 Consumer: cynic_dispatch_agent_task (dispatch when ACT)
Stability: new
"""
from __future__ import annotations

from enum import Enum

from organs.mirror.profile import BehavioralProfile, PHI_INV_SQ, feature_confidence


class ActionDecision(Enum):
    """Whether to observe passively or dispatch an agent task."""

    OBSERVE = "observe"
    ACT = "act"


# Features relevant per domain — domain → list of profile attribute names
_DOMAIN_FEATURES: dict[str, list[str]] = {
    "x_curation": ["narrative_affinity", "author_affinity"],
}


class ActionGate:
    """Gate that decides ACT vs OBSERVE based on behavioral confidence.

    Input contract: profile is a valid BehavioralProfile; domain is a known key.
    Output guarantee: returns ActionDecision (never raises on empty profile).
    Failure modes: unknown domain → treats as no relevant features → OBSERVE.
    Valid domains: x_curation.
    """

    def evaluate(self, profile: BehavioralProfile, domain: str) -> ActionDecision:
        """Compute average feature confidence and gate on PHI_INV_SQ (0.382).

        Stability is derived from profile.pattern_stability — averaged across
        all recorded axes, defaulting to 0.5 when the dict is empty.
        """
        relevant_features = _DOMAIN_FEATURES.get(domain, [])
        if not relevant_features:
            return ActionDecision.OBSERVE

        stability_values = list(profile.pattern_stability.values())
        avg_stability = (
            sum(stability_values) / len(stability_values)
            if stability_values
            else 0.5
        )

        confidences: list[float] = []
        for feature_name in relevant_features:
            feature_data = getattr(profile, feature_name, {})
            # A feature is "active" when its dict is non-empty
            if feature_data:
                conf = feature_confidence(
                    profile.observation_count, avg_stability
                )
                confidences.append(conf)
            else:
                confidences.append(0.0)

        avg_confidence = sum(confidences) / len(confidences)
        return ActionDecision.ACT if avg_confidence >= PHI_INV_SQ else ActionDecision.OBSERVE

    def build_task_content(self, profile: BehavioralProfile) -> str:
        """Build a human-readable Hermes task prompt from the profile.

        Includes the top 3 narratives (by affinity score), top 3 authors
        (by affinity score), and the profile's overall confidence.

        Input contract: profile is a valid BehavioralProfile (may be empty).
        Output guarantee: always returns a non-empty string.
        """
        top_narratives = _top_keys(profile.narrative_affinity, n=3)
        top_authors = _top_keys(profile.author_affinity, n=3)

        stability_values = list(profile.pattern_stability.values())
        avg_stability = (
            sum(stability_values) / len(stability_values)
            if stability_values
            else 0.5
        )
        conf = feature_confidence(profile.observation_count, avg_stability)

        return (
            f"Browse X looking for content matching these preferences: "
            f"Narratives: {top_narratives}. "
            f"Priority authors: {top_authors}. "
            f"Pattern confidence: {conf:.2f}"
        )


def _top_keys(mapping: dict[str, float], n: int) -> list[str]:
    """Return the top-n keys sorted by descending value."""
    return [k for k, _ in sorted(mapping.items(), key=lambda kv: kv[1], reverse=True)[:n]]
