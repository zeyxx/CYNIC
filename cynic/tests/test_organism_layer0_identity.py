"""Layer 0: Identity Tests — Validate Axiom Constraints as DNA

Tests that:
1. Identity is immutable (axioms never change)
2. Constraints are executable (can validate judgments)
3. Invalid judgments are rejected (immune check works)
4. Serialization preserves DNA
"""

import pytest
from cynic.organism.layers.identity import OrganismIdentity, AxiomConstraint, MAX_CONFIDENCE, MAX_Q_SCORE


class MockJudgment:
    """Mock judgment object for testing constraints."""

    def __init__(
        self,
        confidence: float = 0.5,
        q_score: float = 75.0,
        justification: str = "test",
        precedent: str = "past_decision",
        cost_usd: float = 0.01,
    ):
        self.confidence = confidence
        self.q_score = q_score
        self.justification = justification
        self.precedent = precedent
        self.cost_usd = cost_usd


def test_identity_creation():
    """Identity can be instantiated."""
    identity = OrganismIdentity(name="CYNIC-ALPHA")
    assert identity.name == "CYNIC-ALPHA"
    assert identity.species == "Cynical Dog"
    assert identity.axioms is not None


def test_identity_has_5_axioms():
    """Exactly 5 axioms present (DNA structure)."""
    identity = OrganismIdentity()
    assert len(identity.axioms) == 5
    expected_axioms = {"FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"}
    assert set(identity.axioms.keys()) == expected_axioms


def test_axiom_immutability():
    """Axioms never change after instantiation."""
    identity1 = OrganismIdentity()
    axioms1 = set(identity1.axioms.keys())

    identity2 = OrganismIdentity()
    axioms2 = set(identity2.axioms.keys())

    # Both instances should have same axioms
    assert axioms1 == axioms2 == {"FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"}


def test_valid_judgment_passes_all_constraints():
    """Valid judgment violates no axioms."""
    identity = OrganismIdentity()

    # Valid: all constraints satisfied
    judgment = MockJudgment(
        confidence=0.5,  # < φ⁻¹ (0.618)
        q_score=75,  # [0, 100]
        justification="test reason",
        precedent="past decision",
        cost_usd=0.01,
    )

    violations = identity.validate_judgment(judgment)
    assert len(violations) == 0, f"Valid judgment should have no violations, but got: {violations}"


def test_fidelity_constraint_rejects_high_confidence():
    """FIDELITY constraint: confidence ≤ φ⁻¹ (0.618)."""
    identity = OrganismIdentity()

    # Invalid: confidence > 0.618
    judgment = MockJudgment(
        confidence=0.9,  # > MAX_CONFIDENCE
        q_score=75,
        justification="test",
        precedent="past",
        cost_usd=0.01,
    )

    violations = identity.validate_judgment(judgment)
    assert "FIDELITY" in violations, "Should reject high confidence"


def test_phi_constraint_rejects_q_score_out_of_range():
    """PHI constraint: Q-Score ∈ [0, 100]."""
    identity = OrganismIdentity()

    # Invalid: Q-Score > 100
    judgment = MockJudgment(
        confidence=0.5,
        q_score=150,  # > MAX_Q_SCORE
        justification="test",
        precedent="past",
        cost_usd=0.01,
    )

    violations = identity.validate_judgment(judgment)
    assert "PHI" in violations, "Should reject Q-Score > 100"

    # Also test negative Q-Score
    judgment.q_score = -10
    violations = identity.validate_judgment(judgment)
    assert "PHI" in violations, "Should reject Q-Score < 0"


def test_verify_constraint_requires_justification():
    """VERIFY constraint: judgment must have justification."""
    identity = OrganismIdentity()

    # Invalid: no justification
    judgment = MockJudgment(
        confidence=0.5,
        q_score=75,
        justification=None,  # Missing
        precedent="past",
        cost_usd=0.01,
    )

    violations = identity.validate_judgment(judgment)
    assert "VERIFY" in violations, "Should require justification"


def test_culture_constraint_requires_precedent():
    """CULTURE constraint: judgment must have precedent."""
    identity = OrganismIdentity()

    # Invalid: no precedent
    judgment = MockJudgment(
        confidence=0.5,
        q_score=75,
        justification="test",
        precedent=None,  # Missing
        cost_usd=0.01,
    )

    violations = identity.validate_judgment(judgment)
    assert "CULTURE" in violations, "Should require precedent"


def test_burn_constraint_requires_cost_accounting():
    """BURN constraint: judgment must account for cost."""
    identity = OrganismIdentity()

    # Invalid: no cost_usd
    judgment = MockJudgment(
        confidence=0.5,
        q_score=75,
        justification="test",
        precedent="past",
        cost_usd=None,  # Missing
    )

    violations = identity.validate_judgment(judgment)
    assert "BURN" in violations, "Should require cost accounting"


def test_multiple_violations_detected():
    """Multiple axiom violations are all detected."""
    identity = OrganismIdentity()

    # Invalid: multiple violations
    judgment = MockJudgment(
        confidence=0.95,  # FIDELITY violation
        q_score=200,  # PHI violation
        justification=None,  # VERIFY violation
        precedent=None,  # CULTURE violation
        cost_usd=None,  # BURN violation
    )

    violations = identity.validate_judgment(judgment)
    # Should catch all 5 violations
    expected_violations = {"FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"}
    assert set(violations) == expected_violations, f"Expected all 5 violations, got: {violations}"


def test_can_execute_judgment_convenience_method():
    """can_execute_judgment() is boolean equivalent of no violations."""
    identity = OrganismIdentity()

    # Valid judgment
    valid = MockJudgment(confidence=0.5, q_score=75, justification="x", precedent="y", cost_usd=0.01)
    assert identity.can_execute_judgment(valid) is True

    # Invalid judgment
    invalid = MockJudgment(confidence=0.95, q_score=75, justification="x", precedent="y", cost_usd=0.01)
    assert identity.can_execute_judgment(invalid) is False


def test_organism_string_representation():
    """Readable string representation includes axioms."""
    identity = OrganismIdentity(name="CYNIC-V1")
    str_repr = str(identity)
    assert "CYNIC-V1" in str_repr
    assert "Cynical Dog" in str_repr
    assert "DNA:" in str_repr
    # Should mention at least one axiom
    assert any(ax in str_repr for ax in ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"])


def test_serialization_to_dict():
    """Identity serializes to dict preserving axiom names."""
    identity = OrganismIdentity(name="CYNIC-TEST")
    d = identity.to_dict()

    assert d["name"] == "CYNIC-TEST"
    assert d["species"] == "Cynical Dog"
    assert "axioms" in d
    assert len(d["axioms"]) == 5


def test_serialization_to_json():
    """Identity serializes to JSON."""
    identity = OrganismIdentity()
    json_str = identity.to_json()

    assert isinstance(json_str, str)
    assert "FIDELITY" in json_str
    assert "PHI" in json_str
    assert "VERIFY" in json_str
    assert "CULTURE" in json_str
    assert "BURN" in json_str


def test_missing_attribute_raises_error_constraint():
    """Missing attribute on judgment triggers error constraint."""
    identity = OrganismIdentity()

    # Judgment missing required attributes
    class PartialJudgment:
        pass

    judgment = PartialJudgment()
    violations = identity.validate_judgment(judgment)

    # Should catch errors on all constraints
    # (not all 5 axioms might fire, but at least some should)
    assert len(violations) > 0, "Should detect missing attributes"
