"""
Test suite for ConsciousState memory management.

Verifies that the judgment buffer is bounded and properly pruned.
This addresses the MEMORY_MANAGEMENT blue screen issue.

DEPRECATED: ConsciousState refactored to UnifiedConsciousState in V5.
"""

import pytest

pytestmark = pytest.mark.skip(reason="ConsciousState refactored to UnifiedConsciousState in V5")

try:
    from cynic.kernel.core.phi import fibonacci
    from cynic.kernel.core.unified_state import UnifiedConsciousState as ConsciousState
except ImportError:
    ConsciousState = None
    fibonacci = None


@pytest.fixture
async def conscious_state():
    """Create fresh ConsciousState for testing."""
    # Reset singleton
    ConsciousState._instance = None
    state = ConsciousState()
    # Don't initialize with buses for unit testing
    return state


@pytest.mark.asyncio
async def test_judgment_buffer_bounded_at_fibonacci_11(conscious_state):
    """Buffer should be capped at F(11)=89, not 1000."""
    # Add more judgments than the limit
    for i in range(150):
        event_payload = {
            "judgment_id": f"test-{i}",
            "q_score": float(i % 100),
            "verdict": "WAG",
            "confidence": 0.5,
            "dog_votes": {"DOG_A": 0.5, "DOG_B": 0.7},
            "source": "test",
        }
        await conscious_state._on_judgment_created(
            type("FakeEvent", (), {"payload": event_payload})()
        )

    # Should NOT exceed F(11)=89
    max_size = fibonacci(11)  # 89
    assert len(conscious_state._recent_judgments) <= max_size, (
        f"Buffer size {len(conscious_state._recent_judgments)} exceeds "
        f"Fibonacci(11)={max_size}"
    )


@pytest.mark.asyncio
async def test_burn_ordered_pruning(conscious_state):
    """When pruned, lowest Q-Score judgments should be deleted first (BURN axiom)."""
    # Add judgments with varying Q-Scores
    q_scores = [50.0, 90.0, 20.0, 85.0, 10.0, 95.0]
    for i, q_score in enumerate(q_scores):
        event_payload = {
            "judgment_id": f"test-{i}",
            "q_score": q_score,
            "verdict": "WAG",
            "confidence": 0.5,
            "dog_votes": {},
            "source": "test",
        }
        await conscious_state._on_judgment_created(
            type("FakeEvent", (), {"payload": event_payload})()
        )

    # Fill to max
    max_size = fibonacci(11)  # 89
    for i in range(len(q_scores), max_size + 10):
        event_payload = {
            "judgment_id": f"test-{i}",
            "q_score": 50.0,  # Middle value
            "verdict": "WAG",
            "confidence": 0.5,
            "dog_votes": {},
            "source": "test",
        }
        await conscious_state._on_judgment_created(
            type("FakeEvent", (), {"payload": event_payload})()
        )

    # Verify lowest Q-Score was pruned
    judgments = conscious_state._recent_judgments
    q_scores_in_buffer = [j.q_score for j in judgments]

    # The lowest original Q-Score (10.0) should be pruned
    assert 10.0 not in q_scores_in_buffer, (
        "Lowest Q-Score (10.0) should have been pruned under BURN axiom"
    )

    # Higher Q-Scores should remain
    assert 95.0 in q_scores_in_buffer or 90.0 in q_scores_in_buffer, (
        "Higher Q-Scores should be preserved"
    )


@pytest.mark.asyncio
async def test_pending_and_failed_judgments_also_bounded(conscious_state):
    """Pending and failed judgment placeholders should also be bounded."""
    max_size = fibonacci(11)  # 89

    # Add pending judgments
    for i in range(max_size + 20):
        await conscious_state.record_pending_judgment(f"pending-{i}")

    assert len(conscious_state._recent_judgments) <= max_size

    # Add failed judgments
    for i in range(max_size + 20):
        await conscious_state.record_judgment_failed(f"failed-{i}", "test error")

    assert len(conscious_state._recent_judgments) <= max_size


@pytest.mark.asyncio
async def test_memory_footprint_reasonable(conscious_state):
    """
    Full buffer should use reasonable memory.

    F(11)=89 judgments Ã— ~1KB each â‰ˆ 89KB (acceptable)
    vs 1000 judgments Ã— ~1KB each â‰ˆ 1MB (problematic under load)
    """
    max_size = fibonacci(11)  # 89

    # Fill buffer completely with judgments containing dog_votes
    for i in range(max_size):
        event_payload = {
            "judgment_id": f"test-{i}",
            "q_score": 50.0,
            "verdict": "WAG",
            "confidence": 0.5,
            "dog_votes": {
                f"DOG_{j}": float(j) / 100.0
                for j in range(11)  # 11 dogs
            },
            "source": "test",
        }
        await conscious_state._on_judgment_created(
            type("FakeEvent", (), {"payload": event_payload})()
        )

    # Estimate memory usage
    import sys
    total_size = sum(
        sys.getsizeof(j) + sys.getsizeof(j.dog_votes)
        for j in conscious_state._recent_judgments
    )

    # Should be under 200KB (reasonable)
    assert total_size < 200_000, (
        f"Buffer memory footprint {total_size} bytes exceeds reasonable threshold"
    )


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
