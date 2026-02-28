"""
Tests for Q-Learning Module

Tests the QTable and LearningLoop classes for:
- TD(0) updates
- Thompson Sampling exploration
- EWC (Elastic Weight Consolidation)
- Policy selection (exploit/explore)
- DB persistence
- Learning signal handling
"""
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.brain.learning.qlearning import (
    QTable, QEntry, LearningSignal, LearningLoop,
    VERDICTS, THOMPSON_PRIOR,
)
from cynic.kernel.core.phi import (
    LEARNING_RATE, EWC_PENALTY, fibonacci, PHI_INV, PHI_INV_2,
)


class TestQEntry:
    """Test suite for QEntry dataclass."""

    def test_default_values(self):
        """Should initialize with neutral values."""
        entry = QEntry(state_key="test", action="BARK")
        
        assert entry.state_key == "test"
        assert entry.action == "BARK"
        assert entry.q_value == 0.5  # Neutral start
        assert entry.visits == 0
        assert entry.wins == THOMPSON_PRIOR  # 5
        assert entry.losses == THOMPSON_PRIOR  # 5

    def test_thompson_sample_range(self):
        """Thompson sample should be in [0, 1]."""
        entry = QEntry(state_key="test", action="BARK")
        
        samples = [entry.thompson_sample() for _ in range(100)]
        
        assert all(0 <= s <= 1 for s in samples)

    def test_to_dict(self):
        """Should serialize to dict correctly."""
        entry = QEntry(
            state_key="test:state",
            action="WAG",
            q_value=0.75,
            visits=10,
            wins=8,
            losses=7,
            last_updated=1234567890.0
        )
        
        d = entry.to_dict()
        
        assert d["state_key"] == "test:state"
        assert d["action"] == "WAG"
        assert d["q_value"] == 0.75
        assert d["visits"] == 10
        assert d["wins"] == 8
        assert d["losses"] == 7


class TestLearningSignal:
    """Test suite for LearningSignal dataclass."""

    def test_valid_signal(self):
        """Should accept valid signal."""
        signal = LearningSignal(
            state_key="CODE:JUDGE:PRESENT:1",
            action="GROWL",
            reward=0.6
        )
        
        assert signal.state_key == "CODE:JUDGE:PRESENT:1"
        assert signal.action == "GROWL"
        assert signal.reward == 0.6

    def test_reward_clamping(self):
        """Should clamp reward to [0, 1]."""
        signal = LearningSignal(
            state_key="test",
            action="WAG",
            reward=1.5  # Above 1
        )
        assert signal.reward == 1.0
        
        signal2 = LearningSignal(
            state_key="test",
            action="WAG",
            reward=-0.5  # Below 0
        )
        assert signal2.reward == 0.0

    def test_invalid_action(self):
        """Should raise on invalid action."""
        with pytest.raises(ValueError):
            LearningSignal(
                state_key="test",
                action="INVALID",
                reward=0.5
            )


class TestQTable:
    """Test suite for QTable."""

    @pytest.fixture
    def qtable(self):
        """Create a fresh QTable for testing."""
        return QTable()

    def test_initialization(self, qtable):
        """Should initialize with default values."""
        assert qtable._alpha == LEARNING_RATE
        assert qtable._gamma == PHI_INV_2
        assert len(qtable._table) == 0

    def test_update_creates_entry(self, qtable):
        """Should create new entry on first update."""
        signal = LearningSignal(
            state_key="test:state",
            action="WAG",
            reward=0.8
        )
        
        entry = qtable.update(signal)
        
        assert entry.q_value != 0.5  # Updated from neutral
        assert entry.visits == 1

    def test_update_td0_formula(self, qtable):
        """TD(0) update: Q(s,a) ← Q(s,a) + α × (r − Q(s,a))."""
        signal = LearningSignal(
            state_key="test:state",
            action="HOWL",
            reward=1.0  # Maximum reward
        )
        
        entry = qtable.update(signal)
        
        # New Q should be higher than old (0.5)
        assert entry.q_value > 0.5

    def test_update_increments_visits(self, qtable):
        """Should increment visits on each update."""
        for i in range(5):
            signal = LearningSignal(
                state_key="test",
                action="BARK",
                reward=0.5
            )
            qtable.update(signal)
        
        entry = qtable._table["test"]["BARK"]
        assert entry.visits == 5

    def test_update_records_wins_losses(self, qtable):
        """Should track wins (>0.5) and losses (<=0.5)."""
        # High reward = win
        signal1 = LearningSignal(state_key="test", action="WAG", reward=0.8)
        qtable.update(signal1)
        
        # Low reward = loss
        signal2 = LearningSignal(state_key="test", action="WAG", reward=0.3)
        qtable.update(signal2)
        
        entry = qtable._table["test"]["WAG"]
        assert entry.wins == THOMPSON_PRIOR + 1
        assert entry.losses == THOMPSON_PRIOR + 1

    def test_exploit_returns_best_action(self, qtable):
        """Exploit should return action with highest Q-value."""
        # Make WAG the best
        signal = LearningSignal(state_key="test", action="WAG", reward=1.0)
        qtable.update(signal)
        
        best = qtable.exploit("test")
        
        assert best == "WAG"

    def test_exploit_unknown_state(self):
        """Unknown state should return default GROWL."""
        qtable = QTable()
        
        best = qtable.exploit("unknown:state")
        
        assert best == "GROWL"

    def test_explore_returns_sampled_action(self, qtable):
        """Explore should return Thompson-sampled action."""
        # Add some data
        for _ in range(10):
            signal = LearningSignal(state_key="test", action="BARK", reward=0.7)
            qtable.update(signal)
        
        # Explore should return something (not empty)
        action = qtable.explore("test")
        
        assert action in VERDICTS

    def test_predict_q_unknown(self, qtable):
        """Unknown (state, action) should return 0.5."""
        q = qtable.predict_q("unknown", "HOWL")
        
        assert q == 0.5

    def test_predict_q_known(self, qtable):
        """Known (state, action) should return stored Q-value."""
        signal = LearningSignal(state_key="test", action="BARK", reward=0.9)
        entry = qtable.update(signal)
        
        q = qtable.predict_q("test", "BARK")
        
        assert q == entry.q_value

    def test_confidence_unknown_state(self, qtable):
        """Unknown state should have 0 confidence."""
        conf = qtable.confidence("unknown")
        
        assert conf == 0.0

    def test_confidence_known_state(self, qtable):
        """Confidence should increase with visits."""
        # Add many updates
        for _ in range(21):  # F(8) = 21
            signal = LearningSignal(state_key="test", action="WAG", reward=0.5)
            qtable.update(signal)
        
        conf = qtable.confidence("test")

        assert conf > 0  # Has visits
        assert conf <= 0.618033988749895  # Capped at PHI_INV (φ⁻¹)

    def test_stats(self, qtable):
        """Stats should return correct metrics."""
        # Add some data
        signal = LearningSignal(state_key="s1", action="BARK", reward=0.6)
        qtable.update(signal)
        
        stats = qtable.stats()
        
        assert "states" in stats
        assert "entries" in stats
        assert "total_updates" in stats
        assert stats["total_updates"] == 1

    def test_top_states(self, qtable):
        """Top states should return most visited states."""
        # Create uneven distribution
        for _ in range(5):
            qtable.update(LearningSignal(state_key="popular", action="WAG", reward=0.5))
        for _ in range(2):
            qtable.update(LearningSignal(state_key="less_popular", action="WAG", reward=0.5))
        
        top = qtable.top_states(n=2)
        
        assert len(top) == 2
        assert top[0]["state_key"] == "popular"

    def test_matrix_stats(self, qtable):
        """Matrix stats should count states by dimension."""
        qtable.update(LearningSignal(state_key="CODE:JUDGE:PAST:1", action="BARK", reward=0.5))
        qtable.update(LearningSignal(state_key="CODE:JUDGE:PRESENT:2", action="WAG", reward=0.5))
        qtable.update(LearningSignal(state_key="MARKET:ANALYSIS:FUTURE:3", action="HOWL", reward=0.5))
        
        stats = qtable.matrix_stats()
        
        assert stats["total_cells"] == 3
        assert "CODE" in stats["by_reality"]
        assert "MARKET" in stats["by_reality"]

    def test_reset(self, qtable):
        """Reset should clear all entries."""
        qtable.update(LearningSignal(state_key="test", action="BARK", reward=0.5))
        
        qtable.reset()
        
        assert len(qtable._table) == 0
        assert qtable._total_updates == 0


class TestQTableEWC:
    """Test suite for EWC (Elastic Weight Consolidation)."""

    def test_ewc_penalizes_consolidated_states(self):
        """EWC should reduce learning rate for heavily-visited states."""
        qtable = QTable()
        
        # First visit - full learning rate
        signal1 = LearningSignal(state_key="test", action="BARK", reward=0.9)
        entry1 = qtable.update(signal1)
        first_q = entry1.q_value
        
        # Many more visits
        for _ in range(20):  # Total 21 = F(8)
            signal = LearningSignal(state_key="test", action="BARK", reward=0.9)
            qtable.update(signal)
        
        # At F(8)=21 visits, should have EWC penalty
        final_entry = qtable._table["test"]["BARK"]
        
        # EWC consolidated entries should have visited >= F(8)
        stats = qtable.stats()
        assert stats["ewc_consolidated"] >= 1


class TestLearningLoop:
    """Test suite for LearningLoop."""

    @pytest.fixture
    def mock_event_bus(self):
        """Create a mock event bus."""
        bus = MagicMock()
        bus.on = MagicMock()
        return bus

    @pytest.fixture
    def qtable(self):
        """Create a QTable for testing."""
        return QTable()

    def test_initialization(self, qtable, mock_event_bus):
        """Should initialize correctly."""
        loop = LearningLoop(qtable)
        
        assert loop.qtable is qtable
        assert loop._pool is None
        assert loop._active is False
        assert loop._updates_since_flush == 0

    def test_start_registers_listener(self, qtable, mock_event_bus):
        """Start should register LEARNING_EVENT listener."""
        loop = LearningLoop(qtable)
        
        loop.start(mock_event_bus)
        
        mock_event_bus.on.assert_called_once()
        assert loop._active is True

    def test_stop_unregisters(self, qtable, mock_event_bus):
        """Stop should deactivate."""
        loop = LearningLoop(qtable)
        loop.start(mock_event_bus)
        
        loop.stop()
        
        assert loop._active is False

    @pytest.mark.asyncio
    async def test_on_learning_event(self, qtable, mock_event_bus):
        """Should process learning events."""
        loop = LearningLoop(qtable)
        loop.start(mock_event_bus)
        
        # Create mock event
        mock_event = MagicMock()
        mock_event.dict_payload = {
            "state_key": "test:state",
            "action": "WAG",
            "reward": 0.7,
            "judgment_id": "test-123",
            "loop_name": "TEST"
        }
        
        await loop._on_learning_event(mock_event)
        
        # Check Q-table was updated
        assert qtable.predict_q("test:state", "WAG") != 0.5

    @pytest.mark.asyncio
    async def test_on_learning_event_invalid_payload(self, qtable, mock_event_bus):
        """Should handle invalid payload gracefully."""
        loop = LearningLoop(qtable)
        loop.start(mock_event_bus)
        
        # Invalid payload
        mock_event = MagicMock()
        mock_event.dict_payload = {"invalid": "data"}
        
        # Should not raise
        await loop._on_learning_event(mock_event)

    def test_flush_interval(self):
        """Flush interval should be F(8) = 21."""
        assert LearningLoop.FLUSH_INTERVAL == fibonacci(8)


class TestQTableDBPersistence:
    """Test suite for QTable DB persistence."""

    @pytest.fixture
    def qtable(self):
        """Create a QTable for testing."""
        return QTable()

    @pytest.mark.asyncio
    async def test_flush_to_db_empty(self, qtable):
        """Empty table should return 0."""
        mock_pool = AsyncMock()
        
        flushed = await qtable.flush_to_db(mock_pool)
        
        assert flushed == 0

    @pytest.mark.asyncio
    async def test_load_from_db(self, qtable):
        """Should load entries from DB."""
        mock_pool = MagicMock()  # Regular mock, not AsyncMock
        mock_conn = AsyncMock()
        mock_pool.acquire.return_value.__aenter__ = AsyncMock(return_value=mock_conn)
        mock_pool.acquire.return_value.__aexit__ = AsyncMock(return_value=None)
        
        mock_conn.fetch = AsyncMock(return_value=[
            {"state_key": "test", "action": "BARK", "q_value": 0.8, "visit_count": 10}
        ])
        
        count = await qtable.load_from_db(mock_pool)
        
        assert count == 1
        assert qtable.predict_q("test", "BARK") == 0.8

    def test_load_from_entries(self, qtable):
        """Should load from entry list."""
        entries = [
            {"state_key": "s1", "action": "WAG", "q_value": 0.7, "visit_count": 5},
            {"state_key": "s2", "action": "BARK", "q_value": 0.3, "visit_count": 3},
        ]
        
        count = qtable.load_from_entries(entries)
        
        assert count == 2
        assert qtable.predict_q("s1", "WAG") == 0.7
        assert qtable.predict_q("s2", "BARK") == 0.3


class TestVerdicts:
    """Test suite for verdict constants."""

    def test_all_verdicts_defined(self):
        """All 4 verdicts should be defined."""
        assert "BARK" in VERDICTS
        assert "GROWL" in VERDICTS
        assert "WAG" in VERDICTS
        assert "HOWL" in VERDICTS
        assert len(VERDICTS) == 4

    def test_thompson_prior_fibonacci(self):
        """Thompson prior should be F(5) = 5."""
        assert THOMPSON_PRIOR == fibonacci(5)
