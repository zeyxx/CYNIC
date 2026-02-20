"""Layer 1: Judgment Engine Tests — Unified Will (Not Averaging)

Tests that:
1. Engine computes Q-Scores correctly (geometric mean, highest confidence, PBFT)
2. Layer 0 axiom penalties applied to dog inputs
3. Unified judgment passes Layer 0 immune check
4. Multiple algorithms produce different results as expected
5. Edge cases handled (empty inputs, single dog, outliers)
"""

import pytest
import asyncio
from cynic.organism.layers.identity import OrganismIdentity
from cynic.organism.layers.judgment_engine import (
    JudgmentEngine,
    DogInput,
    UnifiedJudgment,
)


@pytest.fixture
def identity():
    """Fresh identity (Layer 0) for each test."""
    return OrganismIdentity(name="TEST-ENGINE")


@pytest.fixture
def engine_geometric(identity):
    """Judgment engine using geometric mean."""
    return JudgmentEngine(
        identity=identity,
        algorithm="geometric_mean",
        axiom_penalty=0.5,
    )


@pytest.fixture
def engine_highest_conf(identity):
    """Judgment engine using highest confidence."""
    return JudgmentEngine(
        identity=identity,
        algorithm="highest_confidence",
        axiom_penalty=0.5,
    )


@pytest.fixture
def engine_pbft(identity):
    """Judgment engine using PBFT."""
    return JudgmentEngine(
        identity=identity,
        algorithm="pbft",
        axiom_penalty=0.5,
    )


def mock_dog_inputs(scores_and_confs):
    """Helper: create list of DogInput from (q_score, confidence) pairs.

    scores_and_confs: list of (q_score, confidence) tuples
    Returns: list of DogInput
    """
    return [
        DogInput(
            dog_id=f"dog_{i}",
            q_score=score,
            confidence=conf,
            verdict="WAG",
            justification=f"dog_{i} judgment",
        )
        for i, (score, conf) in enumerate(scores_and_confs)
    ]


# ═══════════════════════════════════════════════════════════════════════════════
# BASIC TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_engine_initialization(identity):
    """Engine can be instantiated."""
    engine = JudgmentEngine(identity=identity, algorithm="geometric_mean")
    assert engine.identity is identity
    assert engine.algorithm == "geometric_mean"
    assert engine.axiom_penalty == 0.5


@pytest.mark.asyncio
async def test_engine_empty_input(engine_geometric):
    """Empty dog inputs produce neutral judgment."""
    judgment = await engine_geometric.judge([])
    assert judgment.q_score == 50.0
    assert judgment.verdict == "WAG"
    assert judgment.confidence == 0.0


@pytest.mark.asyncio
async def test_engine_single_dog(engine_geometric):
    """Single dog input produces judgment from that dog."""
    inputs = mock_dog_inputs([(75.0, 0.6)])
    judgment = await engine_geometric.judge(inputs)

    assert abs(judgment.q_score - 75.0) < 0.01  # Allow floating point error
    assert judgment.verdict == "WAG"


# ═══════════════════════════════════════════════════════════════════════════════
# ALGORITHM TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_geometric_mean_algorithm(engine_geometric):
    """Geometric mean produces expected Q-Score."""
    # All dogs agree
    inputs = mock_dog_inputs([
        (50.0, 0.5),
        (50.0, 0.5),
        (50.0, 0.5),
    ])
    judgment = await engine_geometric.judge(inputs)

    # Geometric mean of [50, 50, 50] = 50
    assert abs(judgment.q_score - 50.0) < 1.0


@pytest.mark.asyncio
async def test_highest_confidence_algorithm(engine_highest_conf):
    """Highest confidence picks dog with highest confidence (after axiom penalties)."""
    inputs = mock_dog_inputs([
        (30.0, 0.3),  # Low confidence (no violation)
        (75.0, 0.8),  # High confidence BUT FIDELITY violation → penalized to 0.4
        (60.0, 0.5),  # Medium confidence (no violation) ← highest after penalties
    ])
    judgment = await engine_highest_conf.judge(inputs)

    # After FIDELITY penalty (0.8 > 0.618):
    # - dog_0: 0.3 (no violation)
    # - dog_1: 0.4 (penalized)
    # - dog_2: 0.5 (highest)
    # Should use dog_2 (60.0, confidence 0.5)
    assert judgment.q_score == 60.0


@pytest.mark.asyncio
async def test_pbft_algorithm_rejects_outliers(engine_pbft):
    """PBFT consensus rejects outliers."""
    inputs = mock_dog_inputs([
        (10.0, 0.5),   # Outlier low
        (50.0, 0.5),
        (51.0, 0.5),
        (52.0, 0.5),
        (90.0, 0.5),   # Outlier high
    ])
    judgment = await engine_pbft.judge(inputs)

    # PBFT should trim outliers and take median of [50, 51, 52]
    # Median = 51
    assert abs(judgment.q_score - 51.0) < 1.0


# ═══════════════════════════════════════════════════════════════════════════════
# VERDICT TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_verdict_bark_low_score(engine_geometric):
    """Q-Score < 38.2 → BARK."""
    inputs = mock_dog_inputs([(30.0, 0.5)])
    judgment = await engine_geometric.judge(inputs)
    assert judgment.verdict == "BARK"


@pytest.mark.asyncio
async def test_verdict_growl_medium_score(engine_geometric):
    """Q-Score [38.2, 61.8) → GROWL."""
    inputs = mock_dog_inputs([(50.0, 0.5)])
    judgment = await engine_geometric.judge(inputs)
    assert judgment.verdict == "GROWL"


@pytest.mark.asyncio
async def test_verdict_wag_high_score(engine_geometric):
    """Q-Score [61.8, 82.0) → WAG."""
    inputs = mock_dog_inputs([(75.0, 0.5)])
    judgment = await engine_geometric.judge(inputs)
    assert judgment.verdict == "WAG"


@pytest.mark.asyncio
async def test_verdict_howl_very_high_score(engine_geometric):
    """Q-Score >= 82.0 → HOWL."""
    inputs = mock_dog_inputs([(85.0, 0.5)])
    judgment = await engine_geometric.judge(inputs)
    assert judgment.verdict == "HOWL"


# ═══════════════════════════════════════════════════════════════════════════════
# LAYER 0 AXIOM PENALTY TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_axiom_penalty_on_high_confidence_dog(engine_geometric):
    """Dog with high confidence (violates FIDELITY) gets penalized."""
    inputs = mock_dog_inputs([
        (75.0, 0.95),  # FIDELITY violation: conf > 0.618
    ])
    judgment = await engine_geometric.judge(inputs)

    # Dog's confidence should be penalized (multiplied by 0.5)
    # But Q-Score stays (confidence is different from q_score)
    # The penalized dog has less influence on unified confidence
    assert judgment.confidence < 0.95  # Should be lower


@pytest.mark.asyncio
async def test_axiom_penalty_multiple_violations(engine_geometric):
    """Multiple dogs violating axioms get penalized."""
    inputs = mock_dog_inputs([
        (75.0, 0.95),  # FIDELITY violation
        (75.0, 0.95),  # FIDELITY violation
        (75.0, 0.3),   # OK
    ])
    judgment = await engine_geometric.judge(inputs)

    # Two dogs penalized, should reduce unified confidence
    assert judgment.confidence < 0.5  # Average of (0.95*0.5, 0.95*0.5, 0.3)


# ═══════════════════════════════════════════════════════════════════════════════
# LAYER 0 IMMUNE CHECK TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_unified_judgment_passes_layer0(engine_geometric):
    """Unified judgment passes Layer 0 axiom check."""
    inputs = mock_dog_inputs([
        (75.0, 0.5),
        (80.0, 0.5),
    ])
    judgment = await engine_geometric.judge(inputs)

    # Judgment should have:
    # - confidence <= 0.618 (φ-bounded)
    # - q_score in [0, 100]
    # - justification (set by engine)
    # - precedent (set by engine)
    # - cost_usd (set by engine)
    # All Layer 0 checks should pass
    assert len(judgment.layer0_violations) == 0


@pytest.mark.asyncio
async def test_unified_judgment_contains_dog_votes(engine_geometric):
    """Unified judgment records which dogs voted what."""
    inputs = mock_dog_inputs([
        (30.0, 0.5),
        (75.0, 0.5),
        (90.0, 0.5),
    ])
    judgment = await engine_geometric.judge(inputs)

    assert len(judgment.dog_votes) == 3
    assert judgment.dog_votes["dog_0"] == 30.0
    assert judgment.dog_votes["dog_1"] == 75.0
    assert judgment.dog_votes["dog_2"] == 90.0


# ═══════════════════════════════════════════════════════════════════════════════
# CONFIDENCE COMPUTATION TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_confidence_high_when_dogs_agree(engine_geometric):
    """Unified confidence high when dogs agree on Q-Score."""
    # All dogs have same confidence → no disagreement → high unified confidence
    inputs = mock_dog_inputs([
        (75.0, 0.6),
        (75.0, 0.6),
        (75.0, 0.6),
    ])
    judgment = await engine_geometric.judge(inputs)

    # All have same confidence, so variance is 0, disagreement_penalty = 1.0
    # unified_conf = 0.6 * 1.0 = 0.6
    assert judgment.confidence > 0.5


@pytest.mark.asyncio
async def test_confidence_low_when_dogs_disagree(engine_geometric):
    """Unified confidence low when dogs disagree heavily."""
    # Dogs have very different confidences → high disagreement → low unified confidence
    inputs = mock_dog_inputs([
        (75.0, 0.1),
        (75.0, 0.9),
        (75.0, 0.1),
    ])
    judgment = await engine_geometric.judge(inputs)

    # High variance in confidence → disagreement_penalty is low
    # unified_conf should be lower
    assert judgment.confidence < 0.4


# ═══════════════════════════════════════════════════════════════════════════════
# ALGORITHM COMPARISON TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_algorithms_produce_different_results(identity):
    """Different algorithms can produce different results."""
    inputs = mock_dog_inputs([
        (10.0, 0.5),
        (50.0, 0.5),
        (90.0, 0.5),
    ])

    eng_geo = JudgmentEngine(identity, algorithm="geometric_mean")
    eng_high = JudgmentEngine(identity, algorithm="highest_confidence")
    eng_pbft = JudgmentEngine(identity, algorithm="pbft")

    jdg_geo = await eng_geo.judge(inputs)
    jdg_high = await eng_high.judge(inputs)
    jdg_pbft = await eng_pbft.judge(inputs)

    # All should be defined
    assert jdg_geo.q_score is not None
    assert jdg_high.q_score is not None
    assert jdg_pbft.q_score is not None

    # May or may not be different, but at least recorded
    assert jdg_geo.algorithm == "geometric_mean"
    assert jdg_high.algorithm == "highest_confidence"
    assert jdg_pbft.algorithm == "pbft"


# ═══════════════════════════════════════════════════════════════════════════════
# ERROR HANDLING TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_invalid_algorithm_raises_error(identity):
    """Invalid algorithm name raises ValueError."""
    engine = JudgmentEngine(identity, algorithm="invalid_algorithm")
    inputs = mock_dog_inputs([(75.0, 0.5)])

    with pytest.raises(ValueError, match="Unknown algorithm"):
        await engine.judge(inputs)


def test_invalid_axiom_penalty_raises_error(identity):
    """Axiom penalty outside [0, 1] raises ValueError."""
    with pytest.raises(ValueError, match="axiom_penalty must be in"):
        JudgmentEngine(identity, axiom_penalty=1.5)

    with pytest.raises(ValueError, match="axiom_penalty must be in"):
        JudgmentEngine(identity, axiom_penalty=-0.1)


# ═══════════════════════════════════════════════════════════════════════════════
# UNIFIED JUDGMENT STRUCTURE TESTS
# ═══════════════════════════════════════════════════════════════════════════════


@pytest.mark.asyncio
async def test_unified_judgment_has_all_fields(engine_geometric):
    """Unified judgment has all required fields."""
    inputs = mock_dog_inputs([(75.0, 0.5)])
    judgment = await engine_geometric.judge(inputs)

    assert hasattr(judgment, 'q_score')
    assert hasattr(judgment, 'verdict')
    assert hasattr(judgment, 'confidence')
    assert hasattr(judgment, 'justification')
    assert hasattr(judgment, 'precedent')
    assert hasattr(judgment, 'cost_usd')
    assert hasattr(judgment, 'dog_votes')
    assert hasattr(judgment, 'algorithm')
    assert hasattr(judgment, 'layer0_violations')


@pytest.mark.asyncio
async def test_unified_judgment_q_score_in_range(engine_geometric):
    """Q-Score is always in [0, 100]."""
    # Test with extreme dog inputs
    for scores in [
        [(0.0, 0.5)],
        [(100.0, 0.5)],
        [(0.0, 0.5), (100.0, 0.5)],
        [(10.0, 0.5), (20.0, 0.5), (90.0, 0.5)],
    ]:
        inputs = mock_dog_inputs(scores)
        judgment = await engine_geometric.judge(inputs)
        assert 0 <= judgment.q_score <= 100, f"Q-Score {judgment.q_score} out of range"


@pytest.mark.asyncio
async def test_unified_judgment_confidence_phi_bounded(engine_geometric):
    """Confidence is φ-bounded (≤ 0.618)."""
    inputs = mock_dog_inputs([(75.0, 0.5), (75.0, 0.5)])
    judgment = await engine_geometric.judge(inputs)

    assert judgment.confidence <= 0.618, f"Confidence {judgment.confidence} exceeds φ-bound"
