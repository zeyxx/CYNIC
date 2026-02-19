"""
Tests for events_schema.py — typed event payload contracts (T36).

Coverage:
  1.  All schemas instantiate with zero arguments (all defaults valid)
  2.  model_validate() with empty dict → same as no-arg construction
  3.  model_validate() with realistic payload round-trips cleanly
  4.  Extra fields are silently allowed (never reject unknown fields)
  5.  MetaCyclePayload computed properties (.pass_rate, .regression)
  6.  SdkToolJudgedPayload fields match handler's mapping constants
  7.  ActionProposedPayload priority field defaults to 3 (normal)
  8.  JudgmentCreatedPayload verdict defaults to "WAG" (safe neutral)
  9.  ResidualHighPayload residual_variance defaults to 0.0
  10. UserFeedbackPayload rating defaults to 3.0 (neutral)
  11. DiskPressurePayload pressure defaults to "WARN"
  12. TranscendencePayload active_axioms is an empty list by default
  13. All 24 schema classes are importable from a single import
  14. AxiomActivatedPayload trigger field is optional (default "")
  15. ConsciousnessChangedPayload direction is optional (default "")
  16. SdkResultReceivedPayload is_error defaults to False
  17. LearningEventPayload reward defaults to 0.0
  18. DecisionMadePayload mcts defaults to False
  19. EmergenceDetectedPayload evidence is an empty dict by default
  20. MetaCyclePayload.regression is False when evolve is empty
"""
from __future__ import annotations

import pytest
from cynic.core.events_schema import (
    ActionProposedPayload,
    ActCompletedPayload,
    ActRequestedPayload,
    AnomalyDetectedPayload,
    AxiomActivatedPayload,
    BudgetWarningPayload,
    ConsciousnessChangedPayload,
    ConsensusFailedPayload,
    ConsensusReachedPayload,
    DecisionMadePayload,
    DiskPressurePayload,
    EmergenceDetectedPayload,
    EwcCheckpointPayload,
    JudgmentCreatedPayload,
    LearningEventPayload,
    MemoryPressurePayload,
    MetaCyclePayload,
    PerceptionReceivedPayload,
    QTableUpdatedPayload,
    ResidualHighPayload,
    SdkResultReceivedPayload,
    SdkSessionStartedPayload,
    SdkToolJudgedPayload,
    SelfImprovementProposedPayload,
    TranscendencePayload,
    UserCorrectionPayload,
    UserFeedbackPayload,
)
from cynic.core.phi import HOWL_MIN, WAG_MIN, GROWL_MIN


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

ALL_SCHEMAS = [
    ActionProposedPayload,
    ActCompletedPayload,
    ActRequestedPayload,
    AnomalyDetectedPayload,
    AxiomActivatedPayload,
    BudgetWarningPayload,
    ConsciousnessChangedPayload,
    ConsensusFailedPayload,
    ConsensusReachedPayload,
    DecisionMadePayload,
    DiskPressurePayload,
    EmergenceDetectedPayload,
    EwcCheckpointPayload,
    JudgmentCreatedPayload,
    LearningEventPayload,
    MemoryPressurePayload,
    MetaCyclePayload,
    PerceptionReceivedPayload,
    QTableUpdatedPayload,
    ResidualHighPayload,
    SdkResultReceivedPayload,
    SdkSessionStartedPayload,
    SdkToolJudgedPayload,
    SelfImprovementProposedPayload,
    TranscendencePayload,
    UserCorrectionPayload,
    UserFeedbackPayload,
]


# ════════════════════════════════════════════════════════════════════════════
# UNIVERSAL TESTS (all 27 schemas)
# ════════════════════════════════════════════════════════════════════════════

class TestDefaultConstruction:
    """Every schema must be constructable with zero arguments."""

    @pytest.mark.parametrize("schema_cls", ALL_SCHEMAS)
    def test_no_arg_construction(self, schema_cls):
        """All schemas have valid defaults — no required fields."""
        instance = schema_cls()
        assert instance is not None

    @pytest.mark.parametrize("schema_cls", ALL_SCHEMAS)
    def test_model_validate_empty_dict(self, schema_cls):
        """model_validate({}) == no-arg construction — empty payload is safe."""
        from_empty = schema_cls.model_validate({})
        from_default = schema_cls()
        # Compare via dict to avoid identity checks
        assert from_empty.model_dump() == from_default.model_dump()

    @pytest.mark.parametrize("schema_cls", ALL_SCHEMAS)
    def test_extra_fields_allowed(self, schema_cls):
        """Unknown fields must NOT raise — emitters often include extra fields."""
        instance = schema_cls.model_validate({
            "unknown_field_xyz": "should not raise",
            "another_unknown": 42,
        })
        assert instance is not None


# ════════════════════════════════════════════════════════════════════════════
# JUDGMENT_CREATED
# ════════════════════════════════════════════════════════════════════════════

class TestJudgmentCreatedPayload:
    def test_verdict_defaults_to_wag(self):
        """WAG is the safe neutral default — never assume HOWL."""
        p = JudgmentCreatedPayload()
        assert p.verdict == "WAG"

    def test_reality_defaults_to_code(self):
        """Most judgments are CODE reality — good default."""
        p = JudgmentCreatedPayload()
        assert p.reality == "CODE"

    def test_q_score_defaults_to_zero(self):
        """q_score=0.0 when missing — conservative default (BARK territory)."""
        p = JudgmentCreatedPayload()
        assert p.q_score == pytest.approx(0.0)

    def test_realistic_payload_round_trips(self):
        """Full orchestrator payload validates cleanly."""
        payload = {
            "state_key": "human.perceive.abc123",
            "verdict": "HOWL",
            "q_score": 85.5,
            "confidence": 0.55,
            "reality": "HUMAN",
            "dog_votes": {"guardian": 90.0, "analyst": 80.0},
            "judgment_id": "j-001",
            "level_used": "MACRO",
            "content_preview": "user asked about deployment",
            "context": "CI/CD pipeline",
        }
        p = JudgmentCreatedPayload.model_validate(payload)
        assert p.verdict == "HOWL"
        assert p.q_score == pytest.approx(85.5)
        assert p.reality == "HUMAN"
        assert p.dog_votes["guardian"] == pytest.approx(90.0)


# ════════════════════════════════════════════════════════════════════════════
# META_CYCLE
# ════════════════════════════════════════════════════════════════════════════

class TestMetaCyclePayload:
    def test_pass_rate_property_from_evolve(self):
        """MetaCyclePayload.pass_rate extracts from nested evolve dict."""
        p = MetaCyclePayload(evolve={"pass_rate": 0.92, "regression": False})
        assert p.pass_rate == pytest.approx(0.92)

    def test_regression_property_from_evolve(self):
        """MetaCyclePayload.regression extracts from nested evolve dict."""
        p = MetaCyclePayload(evolve={"pass_rate": 0.60, "regression": True})
        assert p.regression is True

    def test_regression_false_when_evolve_empty(self):
        """Empty evolve → regression=False (safe default, no false alarms)."""
        p = MetaCyclePayload()
        assert p.regression is False
        assert p.pass_rate == pytest.approx(0.0)

    def test_realistic_meta_cycle_payload(self):
        """Full META_CYCLE payload from orchestrator.evolve() round-trips."""
        payload = {
            "evolve": {
                "pass_count": 45,
                "total": 50,
                "pass_rate": 0.90,
                "regression": False,
                "results": [{"test": "test_phi", "ok": True}],
            }
        }
        p = MetaCyclePayload.model_validate(payload)
        assert p.pass_rate == pytest.approx(0.90)
        assert p.regression is False


# ════════════════════════════════════════════════════════════════════════════
# SDK_TOOL_JUDGED
# ════════════════════════════════════════════════════════════════════════════

class TestSdkToolJudgedPayload:
    def test_verdict_default_is_empty_string(self):
        """Empty verdict → unknown state — handler maps unknown → WAG_MIN."""
        p = SdkToolJudgedPayload()
        assert p.verdict == ""

    def test_tool_default_is_empty_string(self):
        p = SdkToolJudgedPayload()
        assert p.tool == ""

    def test_realistic_tool_judged_payload(self):
        payload = {"session_id": "s-001", "tool": "bash", "verdict": "HOWL"}
        p = SdkToolJudgedPayload.model_validate(payload)
        assert p.tool == "bash"
        assert p.verdict == "HOWL"


# ════════════════════════════════════════════════════════════════════════════
# ACTION_PROPOSED
# ════════════════════════════════════════════════════════════════════════════

class TestActionProposedPayload:
    def test_priority_defaults_to_3(self):
        """Priority 3 = normal — most proposed actions are non-critical."""
        p = ActionProposedPayload()
        assert p.priority == 3

    def test_verdict_defaults_to_wag(self):
        """WAG = safe default, same as JudgmentCreatedPayload."""
        p = ActionProposedPayload()
        assert p.verdict == "WAG"


# ════════════════════════════════════════════════════════════════════════════
# RESIDUAL_HIGH
# ════════════════════════════════════════════════════════════════════════════

class TestResidualHighPayload:
    def test_residual_variance_defaults_to_zero(self):
        p = ResidualHighPayload()
        assert p.residual_variance == pytest.approx(0.0)

    def test_realistic_residual_payload(self):
        payload = {
            "cell_id": "cell-abc",
            "residual_variance": 0.75,
            "judgment_id": "j-002",
        }
        p = ResidualHighPayload.model_validate(payload)
        assert p.residual_variance == pytest.approx(0.75)
        assert p.cell_id == "cell-abc"


# ════════════════════════════════════════════════════════════════════════════
# USER_FEEDBACK
# ════════════════════════════════════════════════════════════════════════════

class TestUserFeedbackPayload:
    def test_rating_defaults_to_3(self):
        """Neutral rating (3.0/5) when not provided — avoids false BARK/HOWL."""
        p = UserFeedbackPayload()
        assert p.rating == pytest.approx(3.0)

    def test_realistic_feedback(self):
        p = UserFeedbackPayload.model_validate({"rating": 5.0, "judgment_id": "j-003"})
        assert p.rating == pytest.approx(5.0)
        assert p.judgment_id == "j-003"


# ════════════════════════════════════════════════════════════════════════════
# STORAGE / HEALTH
# ════════════════════════════════════════════════════════════════════════════

class TestHealthPayloads:
    def test_disk_pressure_defaults_to_warn(self):
        p = DiskPressurePayload()
        assert p.pressure == "WARN"

    def test_memory_pressure_defaults_to_warn(self):
        p = MemoryPressurePayload()
        assert p.pressure == "WARN"

    def test_disk_pressure_with_pct(self):
        p = DiskPressurePayload.model_validate(
            {"pressure": "CRITICAL", "used_pct": 0.85, "disk_pct": 0.85}
        )
        assert p.pressure == "CRITICAL"
        assert p.disk_pct == pytest.approx(0.85)


# ════════════════════════════════════════════════════════════════════════════
# EMERGENCE
# ════════════════════════════════════════════════════════════════════════════

class TestEmergencePayloads:
    def test_transcendence_active_axioms_empty_list(self):
        """No axioms active by default — TRANSCENDENCE is rare."""
        p = TranscendencePayload()
        assert p.active_axioms == []

    def test_transcendence_with_all_four(self):
        p = TranscendencePayload.model_validate({
            "active_axioms": ["AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY"],
            "maturity": 95.0,
        })
        assert len(p.active_axioms) == 4
        assert p.maturity == pytest.approx(95.0)

    def test_emergence_evidence_is_empty_dict(self):
        p = EmergenceDetectedPayload()
        assert p.evidence == {}

    def test_axiom_activated_trigger_optional(self):
        p = AxiomActivatedPayload(axiom="SYMBIOSIS", maturity=65.0)
        assert p.trigger == ""    # optional field has empty default

    def test_axiom_activated_realistic(self):
        p = AxiomActivatedPayload.model_validate({
            "axiom": "ANTIFRAGILITY",
            "maturity": 72.5,
            "trigger": "META_CYCLE_REGRESSION",
        })
        assert p.axiom == "ANTIFRAGILITY"
        assert p.trigger == "META_CYCLE_REGRESSION"


# ════════════════════════════════════════════════════════════════════════════
# DECISION / ACT
# ════════════════════════════════════════════════════════════════════════════

class TestDecisionActPayloads:
    def test_decision_mcts_defaults_false(self):
        p = DecisionMadePayload()
        assert p.mcts is False

    def test_act_completed_success_defaults_false(self):
        """Failure is safer default than success — explicit success required."""
        p = ActCompletedPayload()
        assert p.success is False

    def test_sdk_result_is_error_defaults_false(self):
        p = SdkResultReceivedPayload()
        assert p.is_error is False

    def test_learning_event_reward_defaults_zero(self):
        p = LearningEventPayload()
        assert p.reward == pytest.approx(0.0)

    def test_consciousness_changed_direction_optional(self):
        p = ConsciousnessChangedPayload()
        assert p.direction == ""
