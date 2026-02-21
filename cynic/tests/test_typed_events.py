"""
CYNIC Typed Event Payload Tests — events_schema.py (Pydantic v2)

Verifies that all Pydantic payload models are correctly defined,
constructable, and cover the critical CoreEvent types.
"""
from __future__ import annotations

import pytest

from cynic.core.event_bus import CoreEvent, Event
from cynic.core.events_schema import (
    JudgmentCreatedPayload,
    JudgmentRequestedPayload,
    JudgmentFailedPayload,
    ConsensusReachedPayload,
    ConsensusFailedPayload,
    LearningEventPayload,
    DecisionMadePayload,
    ActRequestedPayload,
    ActCompletedPayload,
    ActionProposedPayload,
    EmergenceDetectedPayload,
    ResidualHighPayload,
    AxiomActivatedPayload,
    TranscendencePayload,
    SelfImprovementProposedPayload,
    PerceptionReceivedPayload,
    AnomalyDetectedPayload,
    MetaCyclePayload,
    DiskPressurePayload,
    MemoryPressurePayload,
    DiskClearedPayload,
    MemoryClearedPayload,
    BudgetWarningPayload,
    BudgetExhaustedPayload,
    SdkSessionStartedPayload,
    SdkToolJudgedPayload,
    SdkResultReceivedPayload,
    UserFeedbackPayload,
    UserCorrectionPayload,
    EwcCheckpointPayload,
    QTableUpdatedPayload,
    ConsciousnessChangedPayload,
)


# ── Inventory: every CoreEvent that has a typed payload ──────────────

PAYLOAD_MAP: dict[str, type] = {
    CoreEvent.JUDGMENT_REQUESTED:        JudgmentRequestedPayload,
    CoreEvent.JUDGMENT_CREATED:          JudgmentCreatedPayload,
    CoreEvent.JUDGMENT_FAILED:           JudgmentFailedPayload,
    CoreEvent.CONSENSUS_REACHED:         ConsensusReachedPayload,
    CoreEvent.CONSENSUS_FAILED:          ConsensusFailedPayload,
    CoreEvent.LEARNING_EVENT:            LearningEventPayload,
    CoreEvent.DECISION_MADE:             DecisionMadePayload,
    CoreEvent.ACT_REQUESTED:             ActRequestedPayload,
    CoreEvent.ACT_COMPLETED:             ActCompletedPayload,
    CoreEvent.ACTION_PROPOSED:           ActionProposedPayload,
    CoreEvent.EMERGENCE_DETECTED:        EmergenceDetectedPayload,
    CoreEvent.RESIDUAL_HIGH:             ResidualHighPayload,
    CoreEvent.AXIOM_ACTIVATED:           AxiomActivatedPayload,
    CoreEvent.TRANSCENDENCE:             TranscendencePayload,
    CoreEvent.SELF_IMPROVEMENT_PROPOSED: SelfImprovementProposedPayload,
    CoreEvent.PERCEPTION_RECEIVED:       PerceptionReceivedPayload,
    CoreEvent.ANOMALY_DETECTED:          AnomalyDetectedPayload,
    CoreEvent.META_CYCLE:                MetaCyclePayload,
    CoreEvent.DISK_PRESSURE:             DiskPressurePayload,
    CoreEvent.DISK_CLEARED:              DiskClearedPayload,
    CoreEvent.MEMORY_PRESSURE:           MemoryPressurePayload,
    CoreEvent.MEMORY_CLEARED:            MemoryClearedPayload,
    CoreEvent.BUDGET_WARNING:            BudgetWarningPayload,
    CoreEvent.BUDGET_EXHAUSTED:          BudgetExhaustedPayload,
    CoreEvent.SDK_SESSION_STARTED:       SdkSessionStartedPayload,
    CoreEvent.SDK_TOOL_JUDGED:           SdkToolJudgedPayload,
    CoreEvent.SDK_RESULT_RECEIVED:       SdkResultReceivedPayload,
    CoreEvent.USER_FEEDBACK:             UserFeedbackPayload,
    CoreEvent.USER_CORRECTION:           UserCorrectionPayload,
    CoreEvent.EWC_CHECKPOINT:            EwcCheckpointPayload,
    CoreEvent.Q_TABLE_UPDATED:           QTableUpdatedPayload,
    CoreEvent.CONSCIOUSNESS_CHANGED:     ConsciousnessChangedPayload,
}


class TestPayloadInventory:
    """Every important CoreEvent must have a typed payload class."""

    def test_top_8_events_covered(self):
        critical = {
            CoreEvent.JUDGMENT_CREATED,
            CoreEvent.DECISION_MADE,
            CoreEvent.ACT_REQUESTED,
            CoreEvent.LEARNING_EVENT,
            CoreEvent.EMERGENCE_DETECTED,
            CoreEvent.AXIOM_ACTIVATED,
            CoreEvent.SELF_IMPROVEMENT_PROPOSED,
            CoreEvent.PERCEPTION_RECEIVED,
        }
        for event in critical:
            assert event in PAYLOAD_MAP, f"Missing typed payload for {event}"

    def test_all_payload_classes_are_pydantic(self):
        from pydantic import BaseModel
        for event, cls in PAYLOAD_MAP.items():
            assert issubclass(cls, BaseModel), (
                f"{event} maps to {cls} which is not a Pydantic BaseModel"
            )

    def test_all_payload_classes_allow_extras(self):
        """extra='allow' is required for backward compatibility."""
        for event, cls in PAYLOAD_MAP.items():
            cfg = cls.model_config
            assert cfg.get("extra") == "allow", (
                f"{event} payload {cls.__name__} must have extra='allow'"
            )


class TestPayloadConstruction:
    """Each payload class must be constructable with defaults."""

    def test_judgment_created(self):
        p = JudgmentCreatedPayload(
            judgment_id="j-123",
            q_score=58.5,
            verdict="GROWL",
            confidence=0.42,
            reality="CODE",
        )
        assert p.judgment_id == "j-123"
        assert p.q_score == 58.5
        assert p.verdict == "GROWL"
        assert p.dog_votes == {}

    def test_decision_made(self):
        p = DecisionMadePayload(verdict="WAG", confidence=0.55, reality="CYNIC")
        assert p.verdict == "WAG"

    def test_act_requested(self):
        p = ActRequestedPayload(action_id="act-789", action_type="bash")
        assert p.action_type == "bash"

    def test_learning_event(self):
        p = LearningEventPayload(reward=0.75, action="WAG", state_key="CODE:JUDGE")
        assert p.reward == 0.75

    def test_emergence_detected(self):
        p = EmergenceDetectedPayload(pattern_type="SPIKE", severity=0.8)
        assert p.pattern_type == "SPIKE"

    def test_axiom_activated(self):
        p = AxiomActivatedPayload(axiom="AUTONOMY", maturity=0.75, trigger="decision")
        assert p.axiom == "AUTONOMY"

    def test_self_improvement_proposed(self):
        p = SelfImprovementProposedPayload(proposal_id="sp-001", analysis_type="QTABLE")
        assert p.analysis_type == "QTABLE"

    def test_perception_received(self):
        p = PerceptionReceivedPayload(reality="CODE", source="hook")
        assert p.reality == "CODE"

    def test_consensus_reached(self):
        p = ConsensusReachedPayload(q_score=75.0, votes=7, quorum=5)
        assert p.votes == 7

    def test_budget_warning(self):
        p = BudgetWarningPayload(remaining_usd=3.82, total_spent_usd=6.18)
        assert p.remaining_usd == 3.82

    def test_budget_exhausted(self):
        p = BudgetExhaustedPayload(overspend_usd=0.5)
        assert p.overspend_usd == 0.5

    def test_sdk_session_started(self):
        p = SdkSessionStartedPayload(session_id="s-abc")
        assert p.session_id == "s-abc"


class TestEventTypedIntegration:
    """Event.typed() must work with Pydantic payloads."""

    def test_event_typed_with_pydantic(self):
        p = JudgmentCreatedPayload(verdict="HOWL", q_score=90.0)
        ev = Event.typed(CoreEvent.JUDGMENT_CREATED, p, source="test")
        assert ev.payload["verdict"] == "HOWL"
        assert ev.payload["q_score"] == 90.0

    def test_event_typed_preserves_extra_fields(self):
        p = JudgmentCreatedPayload(verdict="WAG", custom_field="hello")
        ev = Event.typed(CoreEvent.JUDGMENT_CREATED, p, source="test")
        assert ev.payload["custom_field"] == "hello"

    def test_event_typed_roundtrip(self):
        """Emit typed → transport as dict → reconstruct typed."""
        original = DecisionMadePayload(
            verdict="GROWL", reality="SOLANA", confidence=0.38,
        )
        ev = Event.typed(CoreEvent.DECISION_MADE, original, source="test")
        restored = DecisionMadePayload.model_validate(ev.payload)
        assert restored.verdict == original.verdict
        assert restored.reality == original.reality
        assert restored.confidence == original.confidence

    def test_all_defaults_constructable(self):
        """Every payload must be constructable with zero arguments."""
        for event, cls in PAYLOAD_MAP.items():
            try:
                instance = cls()
                dumped = instance.model_dump()
                assert isinstance(dumped, dict), f"{cls.__name__}.model_dump() must return dict"
            except ValidationError as exc:
                pytest.fail(f"{cls.__name__} cannot be constructed with defaults: {exc}")


class TestAsTypedConsumer:
    """Event.as_typed() — consumer-side type reconstruction."""

    def test_as_typed_pydantic_roundtrip(self):
        """Emit typed → as_typed → fields match."""
        original = JudgmentCreatedPayload(
            verdict="HOWL", q_score=88.5, reality="CODE", judgment_id="j-rt",
        )
        ev = Event.typed(CoreEvent.JUDGMENT_CREATED, original, source="test")
        restored = ev.as_typed(JudgmentCreatedPayload)
        assert restored.verdict == "HOWL"
        assert restored.q_score == 88.5
        assert restored.judgment_id == "j-rt"

    def test_as_typed_preserves_extras(self):
        """Extra fields survive the roundtrip."""
        ev = Event(
            type=CoreEvent.JUDGMENT_CREATED,
            payload={"verdict": "WAG", "custom": "data"},
        )
        p = ev.as_typed(JudgmentCreatedPayload)
        assert p.verdict == "WAG"
        assert p.custom == "data"  # extra='allow'

    def test_as_typed_with_empty_payload(self):
        """Empty payload → all defaults."""
        ev = Event(type=CoreEvent.DECISION_MADE, payload={})
        p = ev.as_typed(DecisionMadePayload)
        assert p.verdict == "WAG"   # default
        assert p.reality == "CODE"  # default

    def test_as_typed_rejects_non_model(self):
        """as_typed() with a plain class should raise."""
        ev = Event(type=CoreEvent.JUDGMENT_CREATED, payload={})
        with pytest.raises(TypeError):
            ev.as_typed(dict)

    def test_as_typed_all_payload_classes(self):
        """Every payload class works with as_typed() from empty event."""
        for event, cls in PAYLOAD_MAP.items():
            ev = Event(type=event, payload={})
            p = ev.as_typed(cls)
            assert isinstance(p, cls), f"as_typed({cls.__name__}) failed"
