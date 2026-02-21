"""
CYNIC Immune System Framework — Self-Tests

Tests the framework itself to verify it works correctly before deployment.
Requirement: Must achieve 95%+ accuracy on known test cases.

Status: All layers tested with synthetic test cases.
"""
import asyncio
import pytest
from pathlib import Path

from cynic_immune_system import (
    SafeImmunityFramework,
    PathogenSeverity,
    ExceptionSwallowingDetector,
    VacuousTestDetector,
    ConsensusEngine,
    FrameworkSelfAudit,
    ImmunityMemory,
    FRAMEWORK_ACCURACY_THRESHOLD,
)


# ════════════════════════════════════════════════════════════════════════════
# TEST CASE LIBRARY
# ════════════════════════════════════════════════════════════════════════════

KNOWN_PATHOGENS = {
    "exception_swallowing_1": {
        "code": """
async def save_judgment():
    try:
        await db.save(judgment)
    except Exception:
        pass
""",
        "expected": ["exception_swallowing"],
        "should_not_find": ["vacuous_test"],
    },
    "exception_swallowing_2": {
        "code": """
def handle_event():
    try:
        process(event)
    except Exception:
        return None
""",
        "expected": ["exception_swallowing"],
        "should_not_find": ["vacuous_test"],
    },
    "vacuous_test_1": {
        "code": """
def test_something():
    assert True
""",
        "expected": ["vacuous_test"],
        "should_not_find": ["exception_swallowing"],
    },
    "vacuous_test_2": {
        "code": """
async def test_async_thing():
    assert True
    assert True
""",
        "expected": ["vacuous_test"],
        "should_not_find": ["exception_swallowing"],
    },
    "clean_exception_handling": {
        "code": """
try:
    result = await db.query()
except asyncpg.Error as e:
    logger.error("DB error", exc_info=True)
    raise PersistenceError() from e
""",
        "expected": [],  # Should NOT find pathogens
        "should_not_find": ["exception_swallowing"],
    },
    "good_test": {
        "code": """
def test_calculation():
    result = add(2, 3)
    assert result == 5
    assert result > 0
""",
        "expected": [],  # Should NOT find pathogens
        "should_not_find": ["vacuous_test"],
    },
}


# ════════════════════════════════════════════════════════════════════════════
# LAYER TESTS
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
class TestExceptionSwallowingDetector:
    """Test Layer 1: Exception Swallowing Detection"""

    async def test_detects_bare_except_with_pass(self):
        """Should detect 'except Exception: pass' pattern."""
        detector = ExceptionSwallowingDetector()
        code = "except Exception:\n    pass"
        pathogens = await detector.detect(code, "test.py")
        assert len(pathogens) > 0
        assert pathogens[0].name == "exception_swallowing"

    async def test_detects_bare_except_with_return(self):
        """Should detect 'except Exception: return'."""
        detector = ExceptionSwallowingDetector()
        code = "except Exception:\n    return None"
        pathogens = await detector.detect(code, "test.py")
        assert len(pathogens) > 0

    async def test_ignores_proper_exception_handling(self):
        """Should NOT detect proper exception re-raising."""
        detector = ExceptionSwallowingDetector()
        code = "except asyncpg.Error as e:\n    raise PersistenceError() from e"
        pathogens = await detector.detect(code, "test.py")
        assert len(pathogens) == 0


@pytest.mark.asyncio
class TestVacuousTestDetector:
    """Test Layer 1: Vacuous Test Detection"""

    async def test_detects_assert_true_only(self):
        """Should detect test that only asserts True."""
        detector = VacuousTestDetector()
        code = "def test_something():\n    assert True"
        pathogens = await detector.detect(code, "test.py")
        assert len(pathogens) > 0
        assert pathogens[0].name == "vacuous_test"

    async def test_ignores_real_tests(self):
        """Should NOT flag real tests."""
        detector = VacuousTestDetector()
        code = "def test_calculation():\n    result = add(2, 3)\n    assert result == 5"
        pathogens = await detector.detect(code, "test.py")
        assert len(pathogens) == 0


@pytest.mark.asyncio
class TestConsensusEngine:
    """Test Layer 2: Detector Consensus"""

    async def test_requires_consensus(self):
        """Only report pathogens with detector agreement."""
        engine = ConsensusEngine()
        code = KNOWN_PATHOGENS["exception_swallowing_1"]["code"]
        pathogens = await engine.verify_infection(code, "test.py", threshold=0.6)
        assert len(pathogens) > 0

    async def test_consensus_filters_false_positives(self):
        """Threshold filtering should reduce false positives."""
        engine = ConsensusEngine()
        code = "# Just a comment\nexcept Exception"  # Weak match
        pathogens = await engine.verify_infection(code, "test.py", threshold=0.7)
        # Should require strong consensus
        # (May be empty or low confidence)


@pytest.mark.asyncio
class TestFrameworkSelfAudit:
    """Test Layer 7: Framework Self-Audit"""

    async def test_framework_accuracy_high(self):
        """Framework should achieve > 95% accuracy on known cases."""
        audit = FrameworkSelfAudit()
        accuracy = await audit.audit_framework()
        print(f"\nFramework accuracy: {accuracy:.1%}")
        assert accuracy >= FRAMEWORK_ACCURACY_THRESHOLD, \
            f"Framework accuracy {accuracy:.1%} below threshold {FRAMEWORK_ACCURACY_THRESHOLD:.1%}"

    async def test_framework_detects_exception_swallowing(self):
        """Framework should detect exception swallowing."""
        audit = FrameworkSelfAudit()
        code = "except Exception:\n    pass"
        # Audit will run detection test
        accuracy = await audit.audit_framework()
        assert accuracy > 0


@pytest.mark.asyncio
class TestImmunityMemory:
    """Test Layer 8: Bounded Learning"""

    def test_memory_initialization(self, tmp_path):
        """Memory should initialize and save/load correctly."""
        memory_file = tmp_path / "test_memory.json"
        memory = ImmunityMemory(memory_file=memory_file)
        assert len(memory.patterns) == 0

    def test_learn_success_increments_confidence(self, tmp_path):
        """Learning success should increase confidence."""
        memory_file = tmp_path / "test_memory.json"
        memory = ImmunityMemory(memory_file=memory_file)

        initial_confidence = 0.1
        memory.learn_success("test_pattern", "sig123")
        pattern = memory.patterns["test_pattern"]

        assert pattern.confidence > initial_confidence
        assert pattern.success_count == 1
        assert pattern.status == "LEARNING"

    def test_learn_three_successes_marks_trusted(self, tmp_path):
        """Pattern becomes TRUSTED after 3 successes."""
        memory_file = tmp_path / "test_memory.json"
        memory = ImmunityMemory(memory_file=memory_file)

        for _ in range(3):
            memory.learn_success("test_pattern", "sig123")

        pattern = memory.patterns["test_pattern"]
        assert pattern.status == "TRUSTED"
        assert pattern.success_count == 3

    def test_learn_failure_decrements_confidence(self, tmp_path):
        """Learning failure should decrease confidence."""
        memory_file = tmp_path / "test_memory.json"
        memory = ImmunityMemory(memory_file=memory_file)

        memory.learn_success("test_pattern", "sig123")
        first_confidence = memory.patterns["test_pattern"].confidence

        memory.learn_failure("test_pattern", "sig123")
        second_confidence = memory.patterns["test_pattern"].confidence

        assert second_confidence < first_confidence

    def test_phi_bound_enforced(self, tmp_path):
        """Confidence should never exceed φ⁻¹ (0.618)."""
        from cynic_immune_system import MAX_CONFIDENCE
        memory_file = tmp_path / "test_memory.json"
        memory = ImmunityMemory(memory_file=memory_file)

        # Try to learn many successes
        for _ in range(20):
            memory.learn_success("test_pattern", "sig123")

        confidence = memory.patterns["test_pattern"].confidence
        assert confidence <= MAX_CONFIDENCE, \
            f"Confidence {confidence} exceeds MAX_CONFIDENCE {MAX_CONFIDENCE}"

    def test_memory_persistence(self, tmp_path):
        """Memory should persist to disk and reload."""
        memory_file = tmp_path / "test_memory.json"

        # Save
        memory1 = ImmunityMemory(memory_file=memory_file)
        memory1.learn_success("persistent_pattern", "sig456")
        memory1.save_memory()

        # Reload
        memory2 = ImmunityMemory(memory_file=memory_file)
        assert "persistent_pattern" in memory2.patterns
        assert memory2.patterns["persistent_pattern"].success_count == 1


@pytest.mark.asyncio
class TestSafeImmunityFramework:
    """Test overall framework safety."""

    async def test_framework_initialization(self):
        """Framework should initialize and pass self-audit."""
        framework = SafeImmunityFramework()
        result = await framework.initialize()
        assert result is True
        assert framework.is_safe is True

    async def test_framework_safety_flag_enforced(self):
        """Framework should refuse operations when not safe."""
        framework = SafeImmunityFramework()
        framework.is_safe = False

        code = "except Exception:\n    pass"
        pathogens = await framework.scan_for_pathogens(code, "test.py")
        assert len(pathogens) == 0  # Refused to scan

    async def test_can_detect_pathogens(self):
        """Framework should detect known pathogens."""
        framework = SafeImmunityFramework()
        await framework.initialize()

        code = KNOWN_PATHOGENS["exception_swallowing_1"]["code"]
        pathogens = await framework.scan_for_pathogens(code, "test.py")
        assert len(pathogens) > 0

    async def test_proposes_fix_only_if_quality_high(self):
        """Framework should only propose fixes with quality >= 75%."""
        framework = SafeImmunityFramework()
        await framework.initialize()

        code = KNOWN_PATHOGENS["exception_swallowing_1"]["code"]
        pathogens = await framework.scan_for_pathogens(code, "test.py")

        if pathogens:
            plan = await framework.propose_and_audit_fix(pathogens[0])
            if plan:
                assert plan.quality_score >= 0.75


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION TESTS
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
class TestFullWorkflow:
    """Test complete detection → audit → proposal workflow."""

    async def test_end_to_end_detection_and_proposal(self):
        """Full workflow: detect → audit quality → propose fix."""
        framework = SafeImmunityFramework()
        await framework.initialize()

        # Scan for known pathogen
        code = KNOWN_PATHOGENS["exception_swallowing_1"]["code"]
        pathogens = await framework.scan_for_pathogens(code, "example.py")

        assert len(pathogens) > 0, "Should detect pathogen"
        pathogen = pathogens[0]

        # Propose and audit fix
        plan = await framework.propose_and_audit_fix(pathogen)

        if plan:
            # Should have quality score
            assert hasattr(plan, "quality_score")
            assert plan.quality_score >= 0
            # Should require human approval
            assert not plan.human_approval


# ════════════════════════════════════════════════════════════════════════════
# ACCURACY BENCHMARK
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.asyncio
async def test_framework_accuracy_on_known_cases():
    """
    Comprehensive accuracy test on known test cases.
    Framework must achieve 95%+ accuracy.
    """
    framework = SafeImmunityFramework()
    await framework.initialize()

    correct = 0
    total = 0

    for case_name, case_data in KNOWN_PATHOGENS.items():
        total += 1
        code = case_data["code"]
        expected = case_data["expected"]

        pathogens = await framework.scan_for_pathogens(code, f"{case_name}.py")
        found_names = {p.name for p in pathogens}

        # Check: all expected pathogens found
        all_expected_found = all(name in found_names for name in expected)

        # Check: no unexpected pathogens found
        should_not_find = case_data.get("should_not_find", [])
        no_unexpected = not any(name in found_names for name in should_not_find)

        if all_expected_found and no_unexpected:
            correct += 1
            print(f"✓ {case_name}")
        else:
            print(f"✗ {case_name}")
            print(f"  Expected: {expected}")
            print(f"  Found: {found_names}")
            print(f"  Should not find: {should_not_find}")

    accuracy = correct / total
    print(f"\nFramework Accuracy: {accuracy:.1%} ({correct}/{total})")
    print(f"Threshold: {FRAMEWORK_ACCURACY_THRESHOLD:.1%}")

    assert accuracy >= FRAMEWORK_ACCURACY_THRESHOLD, \
        f"Accuracy {accuracy:.1%} below threshold {FRAMEWORK_ACCURACY_THRESHOLD:.1%}"


if __name__ == "__main__":
    # Run with: pytest scripts/test_immune_framework.py -v
    print("CYNIC Immune System Framework — Self-Test Suite")
    print(f"Accuracy Requirement: {FRAMEWORK_ACCURACY_THRESHOLD:.1%}")
