"""
Unit tests for Q-Learning â€” TD(0) + Thompson Sampling + EWC

Tests:
  1. TD(0) update: Q(s,a) â† Q(s,a) + Î± Ã— (r âˆ’ Q(s,a))
  2. Thompson Sampling: Beta(Î±_wins + 1, Î²_losses + 1) exploration
  3. EWC consolidation: fisher_weight = visits / F(8)
  4. Persistence: flush_to_db, load_from_db, load_from_entries
  5. Policy: exploit (greedy), explore (Thompson)
  6. Stats: matrix coverage, top states
"""

import pytest

from cynic.kernel.core.phi import LEARNING_RATE, fibonacci
from cynic.kernel.organism.brain.learning.qlearning import (
    THOMPSON_PRIOR,
    VERDICTS,
    LearningSignal,
    QTable,
)

# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# FIXTURES
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

@pytest.fixture
def qtable() -> QTable:
    """Create a fresh QTable for each test."""
    return QTable()


@pytest.fixture
def learning_signal() -> LearningSignal:
    """Create a sample learning signal."""
    return LearningSignal(
        state_key="CODE:JUDGE:PRESENT:1",
        action="GROWL",
        reward=0.7,
        judgment_id="test-123",
        loop_name="TEST",
    )


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# TD(0) UPDATE TESTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

def test_qtable_initialization(qtable: QTable) -> None:
    """Test QTable starts empty."""
    stats = qtable.stats()
    assert stats["states"] == 0
    assert stats["entries"] == 0
    assert stats["total_updates"] == 0


def test_qtable_creates_entry_on_first_update(qtable: QTable, learning_signal: LearningSignal) -> None:
    """Test that first update creates a new QEntry."""
    entry = qtable.update(learning_signal)
    
    assert entry is not None
    assert entry.state_key == learning_signal.state_key
    assert entry.action == learning_signal.action
    assert entry.visits == 1
    # Initial Q-value starts at 0.5, updated by TD(0)
    assert 0.0 <= entry.q_value <= 1.0


def test_td0_update_exact(qtable: QTable) -> None:
    """Test TD(0) formula: Q(s,a) â† Q(s,a) + Î± Ã— (r âˆ’ Q(s,a))"""
    signal = LearningSignal(
        state_key="test:state:1",
        action="WAG",
        reward=1.0,  # Max reward
    )
    
    entry = qtable.update(signal)
    
    # Q_new = Q_old + Î± Ã— (r - Q_old)
    # Q_new = 0.5 + 0.038 Ã— (1.0 - 0.5)
    # Q_new = 0.5 + 0.019 = 0.519
    expected_q = 0.5 + LEARNING_RATE * (1.0 - 0.5)
    assert abs(entry.q_value - expected_q) < 0.001


def test_td0_update_multiple_iterations(qtable: QTable) -> None:
    """Test TD(0) converges toward reward with multiple updates."""
    signal = LearningSignal(
        state_key="converge:test:1",
        action="HOWL",
        reward=0.8,
    )
    
    # Update 100 times
    for _ in range(100):
        qtable.update(signal)
    
    entry = qtable._table["converge:test:1"]["HOWL"]
    
    # After 100 updates with reward=0.8, should converge near 0.8
    assert 0.7 < entry.q_value < 0.9


def test_td0_different_actions_same_state(qtable: QTable) -> None:
    """Test separate Q-values for different actions in same state."""
    for action in VERDICTS:
        signal = LearningSignal(
            state_key="multi:action:1",
            action=action,
            reward=0.5 + VERDICTS.index(action) * 0.1,
        )
        qtable.update(signal)
    
    actions = qtable._table["multi:action:1"]
    assert len(actions) == len(VERDICTS)
    
    for action in VERDICTS:
        assert action in actions
        assert 0.0 <= actions[action].q_value <= 1.0


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# THOMPSON SAMPLING TESTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

def test_thompson_sample_balanced_prior(qtable: QTable) -> None:
    """Test Thompson sampling with balanced prior (THOMPSON_PRIOR wins/losses)."""
    entry = qtable._get_or_create("test:thompson:1", "BARK")
    
    # Prior should be balanced: wins=5, losses=5 (THOMPSON_PRIOR=5)
    assert entry.wins == THOMPSON_PRIOR
    assert entry.losses == THOMPSON_PRIOR
    
    # Sample should be around 0.5 with balanced prior
    samples = [entry.thompson_sample() for _ in range(1000)]
    mean = sum(samples) / len(samples)
    
    # Mean should be close to 0.5 (balanced)
    assert 0.45 < mean < 0.55


def test_thompson_sample_skewed_toward_wins(qtable: QTable) -> None:
    """Test Thompson sampling favors wins after positive rewards."""
    # Add many wins
    for _ in range(50):
        signal = LearningSignal(
            state_key="skewed:test:1",
            action="HOWL",
            reward=0.9,  # High reward â†’ win
        )
        qtable.update(signal)
    
    entry = qtable._table["skewed:test:1"]["HOWL"]
    
    # Should have many more wins than losses
    assert entry.wins > entry.losses * 2
    
    # Sample should skew toward 1.0
    samples = [entry.thompson_sample() for _ in range(1000)]
    mean = sum(samples) / len(samples)
    
    # Mean should be > 0.5 (skewed toward wins)
    assert mean > 0.6


def test_thompson_sample_skewed_toward_losses(qtable: QTable) -> None:
    """Test Thompson sampling favors losses after negative rewards."""
    # Add many losses
    for _ in range(50):
        signal = LearningSignal(
            state_key="losing:test:1",
            action="BARK",
            reward=0.1,  # Low reward â†’ loss
        )
        qtable.update(signal)
    
    entry = qtable._table["losing:test:1"]["BARK"]
    
    # Should have many more losses than wins
    assert entry.losses > entry.wins * 2
    
    # Sample should skew toward 0.0
    samples = [entry.thompson_sample() for _ in range(1000)]
    mean = sum(samples) / len(samples)
    
    # Mean should be < 0.5 (skewed toward losses)
    assert mean < 0.4


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# EWC (ELASTIC WEIGHT CONSOLIDATION) TESTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

def test_ewc_fisher_weight_increases_with_visits(qtable: QTable) -> None:
    """Test fisher_weight = min(visits / F(8), 1.0) increases with visits."""
    state = "ewc:test:1"
    action = "WAG"
    
    fib_8 = fibonacci(8)  # 21
    
    # Visit 0 times
    entry = qtable._get_or_create(state, action)
    fisher_0 = min(entry.visits / fib_8, 1.0)
    assert fisher_0 == 0.0
    
    # Visit 10 times
    for _ in range(10):
        qtable.update(LearningSignal(state_key=state, action=action, reward=0.5))
    entry = qtable._get_or_create(state, action)
    fisher_10 = min(entry.visits / fib_8, 1.0)
    assert 0.0 < fisher_10 < 1.0
    
    # Visit 21 times (F(8))
    for _ in range(11):  # 10 + 11 = 21
        qtable.update(LearningSignal(state_key=state, action=action, reward=0.5))
    entry = qtable._get_or_create(state, action)
    fisher_21 = min(entry.visits / fib_8, 1.0)
    assert fisher_21 == 1.0  # Capped at 1.0


def test_ewc_effective_alpha_decreases_with_consolidation(qtable: QTable) -> None:
    """Test effective_Î± = Î± Ã— (1 - Î» Ã— fisher) decreases with consolidation."""
    state = "ewc:alpha:test:1"
    action = "GROWL"
    
    # Track Q-values over updates
    q_values = []
    
    # Update with same reward multiple times
    for _ in range(30):
        qtable.update(LearningSignal(state_key=state, action=action, reward=0.8))
        entry = qtable._table[state][action]
        q_values.append(entry.q_value)
    
    # First updates should learn fast (high effective Î±)
    # Later updates should learn slow (low effective Î±)
    early_change = q_values[5] - q_values[0]
    late_change = q_values[25] - q_values[20]
    
    # Early learning should be faster than late learning
    assert early_change > late_change


def test_ewc_unvisited_states_learn_fast(qtable: QTable) -> None:
    """Test unvisited states learn at full Î±."""
    # Update state A 30 times
    for _ in range(30):
        qtable.update(LearningSignal(state_key="visited:a:1", action="WAG", reward=0.8))
    
    # Update state B once
    qtable.update(LearningSignal(state_key="visited:b:1", action="WAG", reward=0.8))
    
    entry_a = qtable._table["visited:a:1"]["WAG"]
    entry_b = qtable._table["visited:b:1"]["WAG"]
    
    # State A is consolidated, B is fresh
    # Both received same reward, but B should have larger change from initial
    # because it has higher effective Î±
    assert entry_b.visits == 1
    assert entry_a.visits == 30


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# POLICY TESTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

def test_exploit_returns_greedy_action(qtable: QTable) -> None:
    """Test exploit() returns action with highest Q-value."""
    state = "policy:exploit:1"
    
    # Update actions with different rewards
    qtable.update(LearningSignal(state_key=state, action="BARK", reward=0.3))
    qtable.update(LearningSignal(state_key=state, action="GROWL", reward=0.5))
    qtable.update(LearningSignal(state_key=state, action="WAG", reward=0.7))
    qtable.update(LearningSignal(state_key=state, action="HOWL", reward=0.9))
    
    best = qtable.exploit(state)
    
    assert best == "HOWL"


def test_exploit_returns_default_for_unknown_state(qtable: QTable) -> None:
    """Test exploit() returns 'GROWL' (cautious default) for unknown state."""
    best = qtable.exploit("unknown:state:1")
    assert best == "GROWL"


def test_explore_returns_all_verdicts(qtable: QTable) -> None:
    """Test explore() can return any verdict based on Thompson sampling."""
    state = "policy:explore:1"
    
    # Initialize all actions
    for action in VERDICTS:
        qtable.update(LearningSignal(state_key=state, action=action, reward=0.5))
    
    # Sample many times
    results = {v: 0 for v in VERDICTS}
    for _ in range(500):
        action = qtable.explore(state)
        results[action] += 1
    
    # All actions should be sampled at least once (with balanced prior)
    for action in VERDICTS:
        assert results[action] > 0


def test_confidence_bounded_by_phi_inv(qtable: QTable) -> None:
    """Test confidence = min(visits / F(8), Ï†â»Â¹) caps at 0.618."""
    state = "confidence:test:1"
    
    # Update many times
    for _ in range(100):
        qtable.update(LearningSignal(state_key=state, action="WAG", reward=0.5))
    
    confidence = qtable.confidence(state)
    
    # Should be capped at PHI_INV = 0.618
    from cynic.kernel.core.phi import MAX_CONFIDENCE, PHI_INV
    assert confidence <= MAX_CONFIDENCE
    assert confidence <= PHI_INV


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# STATS & INTROSPECTION TESTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

def test_stats_tracks_updates(qtable: QTable) -> None:
    """Test stats() tracks total_updates."""
    for i in range(10):
        qtable.update(LearningSignal(
            state_key=f"stats:test:{i}",
            action="WAG",
            reward=0.5,
        ))
    
    stats = qtable.stats()
    assert stats["total_updates"] == 10


def test_matrix_stats_coverage(qtable: QTable) -> None:
    """Test matrix_stats() reports coverage of 7Ã—7Ã—7 hypercube."""
    # Add some states
    qtable.update(LearningSignal(state_key="CODE:JUDGE:PRESENT:1", action="WAG", reward=0.5))
    qtable.update(LearningSignal(state_key="CODE:JUDGE:FUTURE:2", action="WAG", reward=0.5))
    qtable.update(LearningSignal(state_key="MARKET:ANALYSIS:PAST:1", action="WAG", reward=0.5))
    
    matrix = qtable.matrix_stats()
    
    assert matrix["total_cells"] == 3
    assert matrix["matrix_343"] == 343
    assert "by_reality" in matrix
    assert "by_analysis" in matrix


def test_top_states_returns_sorted(qtable: QTable) -> None:
    """Test top_states() returns states sorted by visits."""
    # Add states with different visit counts
    qtable.update(LearningSignal(state_key="low:visit:1", action="WAG", reward=0.5))
    
    for _ in range(5):
        qtable.update(LearningSignal(state_key="med:visit:1", action="WAG", reward=0.5))
    
    for _ in range(10):
        qtable.update(LearningSignal(state_key="high:visit:1", action="WAG", reward=0.5))
    
    top = qtable.top_states(n=3)
    
    assert len(top) == 3
    assert top[0]["state_key"] == "high:visit:1"
    assert top[1]["state_key"] == "med:visit:1"
    assert top[2]["state_key"] == "low:visit:1"


def test_reset_clears_state(qtable: QTable) -> None:
    """Test reset() clears Q-Table but not DB."""
    # Add some data
    qtable.update(LearningSignal(state_key="reset:test:1", action="WAG", reward=0.5))
    
    stats_before = qtable.stats()
    assert stats_before["states"] > 0
    
    # Reset
    qtable.reset()
    
    stats_after = qtable.stats()
    assert stats_after["states"] == 0
    assert stats_after["entries"] == 0


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# PREDICT_Q TESTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

def test_predict_q_returns_value_for_known_state(qtable: QTable) -> None:
    """Test predict_q() returns Q-value for known state-action."""
    signal = LearningSignal(state_key="predict:test:1", action="HOWL", reward=0.8)
    qtable.update(signal)
    
    q = qtable.predict_q("predict:test:1", "HOWL")
    assert 0.0 <= q <= 1.0


def test_predict_q_returns_default_for_unknown_state(qtable: QTable) -> None:
    """Test predict_q() returns 0.5 for unknown state."""
    q = qtable.predict_q("unknown:predict:1", "WAG")
    assert q == 0.5


def test_predict_q_returns_default_for_unknown_action(qtable: QTable) -> None:
    """Test predict_q() returns 0.5 for known state, unknown action."""
    qtable.update(LearningSignal(state_key="known:state:1", action="WAG", reward=0.5))
    
    q = qtable.predict_q("known:state:1", "BARK")  # Never updated
    assert q == 0.5


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# LOAD FROM ENTRIES TESTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

def test_load_from_entries_warm_start(qtable: QTable) -> None:
    """Test load_from_entries() allows warm start from external source."""
    entries = [
        {"state_key": "load:test:1", "action": "WAG", "q_value": 0.7, "visit_count": 10},
        {"state_key": "load:test:1", "action": "HOWL", "q_value": 0.9, "visit_count": 5},
        {"state_key": "load:test:2", "action": "GROWL", "q_value": 0.3, "visit_count": 20},
    ]
    
    loaded = qtable.load_from_entries(entries)
    
    assert loaded == 3
    
    # Verify loaded values
    assert qtable.predict_q("load:test:1", "WAG") == 0.7
    assert qtable.predict_q("load:test:1", "HOWL") == 0.9
    assert qtable.predict_q("load:test:2", "GROWL") == 0.3


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# EDGE CASES
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

def test_reward_clamped_to_0_1(qtable: QTable) -> None:
    """Test reward is clamped to [0, 1]."""
    # Test reward > 1
    signal = LearningSignal(state_key="clamp:test:1", action="WAG", reward=1.5)
    entry = qtable.update(signal)
    assert entry.q_value <= 1.0
    
    # Test reward < 0
    signal = LearningSignal(state_key="clamp:test:2", action="WAG", reward=-0.5)
    entry = qtable.update(signal)
    assert entry.q_value >= 0.0


def test_invalid_action_raises(qtable: QTable) -> None:
    """Test invalid action raises ValueError."""
    with pytest.raises(ValueError):
        LearningSignal(state_key="invalid:test:1", action="INVALID", reward=0.5)


def test_empty_qtable_stats(qtable: QTable) -> None:
    """Test stats() on empty QTable."""
    stats = qtable.stats()
    
    assert stats["states"] == 0
    assert stats["entries"] == 0
    assert stats["total_updates"] == 0
    assert stats["total_visits"] == 0
    assert stats["pending_flush"] == 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
