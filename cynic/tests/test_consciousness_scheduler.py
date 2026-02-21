"""Tests for ConsciousnessScheduler — blended escalation policy (Task #8)."""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch
import asyncio

from cynic.cognition.cortex.orchestrator import ConsciousnessScheduler
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.phi import PHI_INV


@pytest.fixture
def mock_axiom_monitor():
    """Create a mock AxiomMonitor."""
    monitor = MagicMock()
    monitor.active_count = MagicMock(return_value=0)
    monitor.active_axioms = MagicMock(return_value=[])
    monitor.get_maturity = MagicMock(return_value=0.0)
    return monitor


@pytest.fixture
def mock_escore_tracker():
    """Create a mock EScoreTracker."""
    tracker = MagicMock()
    tracker.get_score = MagicMock(return_value={"q": 0.0})
    return tracker


@pytest.fixture
def mock_oracle_dog():
    """Create a mock Oracle Dog."""
    dog = MagicMock()
    # Mock the analyze method to be async
    async def mock_analyze(cell, budget_usd=None):
        judgment = MagicMock()
        judgment.confidence = 0.0
        return judgment
    dog.analyze = mock_analyze
    return dog


@pytest.fixture
def scheduler(mock_axiom_monitor, mock_escore_tracker, mock_oracle_dog):
    """Create a ConsciousnessScheduler instance."""
    return ConsciousnessScheduler(mock_axiom_monitor, mock_escore_tracker, mock_oracle_dog)


@pytest.fixture
def sample_cell():
    """Create a sample Cell for testing."""
    cell = MagicMock(spec=Cell)
    cell.cell_id = "test_cell_001"
    cell.reality = "CODE"
    cell.content = "test content"
    cell.budget_usd = 1.0
    return cell


class TestConsciousnessSchedulerBasics:
    """Test basic ConsciousnessScheduler structure."""

    def test_scheduler_initialization(self, mock_axiom_monitor, mock_escore_tracker, mock_oracle_dog):
        """ConsciousnessScheduler initializes with dependencies."""
        scheduler = ConsciousnessScheduler(mock_axiom_monitor, mock_escore_tracker, mock_oracle_dog)
        assert scheduler.axiom_monitor == mock_axiom_monitor
        assert scheduler.escore_tracker == mock_escore_tracker
        assert scheduler.oracle_dog == mock_oracle_dog

    def test_scheduler_has_select_level_method(self, scheduler):
        """ConsciousnessScheduler has select_level method."""
        assert hasattr(scheduler, "select_level")
        assert callable(scheduler.select_level)

    def test_scheduler_has_get_signals_method(self, scheduler):
        """ConsciousnessScheduler has get_signals method."""
        assert hasattr(scheduler, "get_signals")
        assert callable(scheduler.get_signals)


class TestConsciousnessLevelSelection:
    """Test consciousness level selection logic."""

    @pytest.mark.asyncio
    async def test_select_level_reflex_when_all_zero(self, scheduler, sample_cell):
        """Select L3 REFLEX when all signals are zero (axiom=0, e_score=0, oracle=0)."""
        level = await scheduler.select_level(sample_cell)
        assert level == ConsciousnessLevel.REFLEX

    @pytest.mark.asyncio
    async def test_select_level_micro_when_low_signals(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """Select L2 MICRO when blended score < 0.618 (38.2%)."""
        # Set signals to produce blended score around 0.45
        # axiom_maturity=30, e_score=30, oracle_confidence=0.4
        # blended = (30/100)*0.4 + (30/100)*0.3 + 0.4*0.3 = 0.12 + 0.09 + 0.12 = 0.33
        # That's < 0.382, so it should be REFLEX. Let me adjust.
        # To get 0.45: need (a/100)*0.4 + (e/100)*0.3 + o*0.3 = 0.45
        # If all equal: x*0.4 + x*0.3 + x*0.3 = 0.45 → x = 0.45 (but x must be ≤0.382 for o)
        # So axiom=60, e_score=60, oracle=0.45
        # blended = (60/100)*0.4 + (60/100)*0.3 + 0.45*0.3 = 0.24 + 0.18 + 0.135 = 0.555 ≈ 0.5 < 0.618 ✓
        mock_axiom_monitor.active_count.return_value = 1
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY"]
        mock_axiom_monitor.get_maturity.return_value = 60.0
        mock_escore_tracker.get_score.return_value = {"q": 60.0}
        scheduler.oracle_dog.analyze = AsyncMock()
        oracle_judgment = MagicMock()
        oracle_judgment.confidence = 0.45
        scheduler.oracle_dog.analyze.return_value = oracle_judgment

        level = await scheduler.select_level(sample_cell)
        assert level == ConsciousnessLevel.MICRO

    @pytest.mark.asyncio
    async def test_select_level_macro_when_medium_signals(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """Select L1 MACRO when 0.618 ≤ blended score < 0.82."""
        # axiom=75, e_score=75, oracle=0.5
        # blended = (75/100)*0.4 + (75/100)*0.3 + 0.5*0.3 = 0.30 + 0.225 + 0.15 = 0.675 ✓
        mock_axiom_monitor.active_count.return_value = 1
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY"]
        mock_axiom_monitor.get_maturity.return_value = 75.0
        mock_escore_tracker.get_score.return_value = {"q": 75.0}
        scheduler.oracle_dog.analyze = AsyncMock()
        oracle_judgment = MagicMock()
        oracle_judgment.confidence = 0.5
        scheduler.oracle_dog.analyze.return_value = oracle_judgment

        level = await scheduler.select_level(sample_cell)
        assert level == ConsciousnessLevel.MACRO

    @pytest.mark.asyncio
    async def test_select_level_meta_when_high_signals_and_axioms_active(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """Select L4 META when blended score ≥ 0.82 AND all 4 axioms active."""
        # axiom=100, e_score=100, oracle=0.618
        # blended = (100/100)*0.4 + (100/100)*0.3 + 0.618*0.3 = 0.4 + 0.3 + 0.1854 = 0.8854 ✓
        mock_axiom_monitor.active_count.return_value = 4
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY"]
        mock_axiom_monitor.get_maturity.return_value = 100.0
        mock_escore_tracker.get_score.return_value = {"q": 100.0}
        scheduler.oracle_dog.analyze = AsyncMock()
        oracle_judgment = MagicMock()
        oracle_judgment.confidence = PHI_INV  # 0.618
        scheduler.oracle_dog.analyze.return_value = oracle_judgment

        level = await scheduler.select_level(sample_cell)
        assert level == ConsciousnessLevel.META

    @pytest.mark.asyncio
    async def test_select_level_macro_when_high_signals_but_not_all_axioms(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """Select L1 MACRO when blended score ≥ 0.82 but only 2 axioms active."""
        # Same signals as META test, but only 2 axioms active
        mock_axiom_monitor.active_count.return_value = 2
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY", "SYMBIOSIS"]
        mock_axiom_monitor.get_maturity.return_value = 100.0
        mock_escore_tracker.get_score.return_value = {"q": 100.0}
        scheduler.oracle_dog.analyze = AsyncMock()
        oracle_judgment = MagicMock()
        oracle_judgment.confidence = PHI_INV
        scheduler.oracle_dog.analyze.return_value = oracle_judgment

        level = await scheduler.select_level(sample_cell)
        # Should be MACRO, not META (not all 4 axioms active)
        assert level == ConsciousnessLevel.MACRO


class TestBlendedSignalCalculation:
    """Test the blended signal calculation."""

    def test_get_signals_with_zero_values(self, scheduler, mock_axiom_monitor, mock_escore_tracker):
        """get_signals returns correct structure with zero values."""
        signals = scheduler.get_signals()
        assert isinstance(signals, dict)
        assert "axiom_maturity" in signals
        assert "e_score" in signals
        assert "oracle_confidence" in signals
        assert "blended" in signals
        assert "active_axioms" in signals
        assert signals["axiom_maturity"] == 0.0
        assert signals["e_score"] == 0.0
        assert signals["oracle_confidence"] == 0.0
        assert signals["blended"] == 0.0

    def test_get_signals_with_nonzero_axiom(self, scheduler, mock_axiom_monitor, mock_escore_tracker):
        """get_signals calculates blended with axiom contribution."""
        # axiom=60, e_score=0, oracle=0
        # blended = (60/100)*0.4 + 0 + 0 = 0.24
        mock_axiom_monitor.active_count.return_value = 1
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY"]
        mock_axiom_monitor.get_maturity.return_value = 60.0

        signals = scheduler.get_signals()
        assert signals["axiom_maturity"] == 60.0
        assert signals["blended"] == pytest.approx(0.24, abs=0.01)

    def test_get_signals_with_nonzero_escore(self, scheduler, mock_axiom_monitor, mock_escore_tracker):
        """get_signals calculates blended with e_score contribution."""
        # axiom=0, e_score=60, oracle=0
        # blended = 0 + (60/100)*0.3 + 0 = 0.18
        mock_axiom_monitor.active_count.return_value = 0
        mock_escore_tracker.get_score.return_value = {"q": 60.0}

        signals = scheduler.get_signals()
        assert signals["e_score"] == 60.0
        assert signals["blended"] == pytest.approx(0.18, abs=0.01)

    def test_get_signals_weighs_contributions_correctly(self, scheduler, mock_axiom_monitor, mock_escore_tracker):
        """get_signals applies correct weights to each signal."""
        # axiom=50 (40% weight), e_score=50 (30% weight)
        # blended = (50/100)*0.4 + (50/100)*0.3 = 0.2 + 0.15 = 0.35
        mock_axiom_monitor.active_count.return_value = 1
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY"]
        mock_axiom_monitor.get_maturity.return_value = 50.0
        mock_escore_tracker.get_score.return_value = {"q": 50.0}

        signals = scheduler.get_signals()
        assert signals["blended"] == pytest.approx(0.35, abs=0.01)


class TestOracleFallback:
    """Test oracle dog failure handling."""

    @pytest.mark.asyncio
    async def test_select_level_handles_oracle_exception(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """select_level gracefully handles oracle dog exception."""
        scheduler.oracle_dog.analyze = AsyncMock(side_effect=RuntimeError("Oracle failed"))

        # Should not raise, should handle gracefully
        level = await scheduler.select_level(sample_cell)
        assert isinstance(level, ConsciousnessLevel)

    @pytest.mark.asyncio
    async def test_select_level_handles_none_oracle(self, scheduler, sample_cell):
        """select_level works when oracle_dog is None."""
        scheduler.oracle_dog = None
        level = await scheduler.select_level(sample_cell)
        # Should still select a level (based on axiom + escore alone)
        assert isinstance(level, ConsciousnessLevel)


class TestEScoreTrackerIntegration:
    """Test e_score_tracker integration."""

    @pytest.mark.asyncio
    async def test_handles_escore_dict_format(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """select_level handles e_score as dict with 'q' key."""
        mock_axiom_monitor.active_count.return_value = 0
        mock_escore_tracker.get_score.return_value = {"q": 50.0}

        level = await scheduler.select_level(sample_cell)
        assert isinstance(level, ConsciousnessLevel)

    @pytest.mark.asyncio
    async def test_handles_escore_object_format(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """select_level handles e_score as object with .q attribute."""
        mock_axiom_monitor.active_count.return_value = 0
        escore_obj = MagicMock()
        escore_obj.q = 50.0
        mock_escore_tracker.get_score.return_value = escore_obj

        level = await scheduler.select_level(sample_cell)
        assert isinstance(level, ConsciousnessLevel)

    @pytest.mark.asyncio
    async def test_handles_missing_escore(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """select_level handles escore_tracker exception."""
        mock_axiom_monitor.active_count.return_value = 0
        mock_escore_tracker.get_score.side_effect = Exception("Tracker error")

        level = await scheduler.select_level(sample_cell)
        assert isinstance(level, ConsciousnessLevel)


class TestThresholdBoundaries:
    """Test consciousness level thresholds."""

    @pytest.mark.asyncio
    async def test_threshold_382_percent_reflex(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """Blended score just below 0.382 (38.2%) selects REFLEX."""
        # To get blended just below 0.382:
        # axiom=95, e_score=0, oracle=0
        # blended = (95/100)*0.4 = 0.38 < 0.382 ✓
        mock_axiom_monitor.active_count.return_value = 1
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY"]
        mock_axiom_monitor.get_maturity.return_value = 95.0
        mock_escore_tracker.get_score.return_value = {"q": 0.0}
        scheduler.oracle_dog.analyze = AsyncMock()
        oracle_judgment = MagicMock()
        oracle_judgment.confidence = 0.0
        scheduler.oracle_dog.analyze.return_value = oracle_judgment

        level = await scheduler.select_level(sample_cell)
        assert level == ConsciousnessLevel.REFLEX

    @pytest.mark.asyncio
    async def test_threshold_618_percent_micro(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """Blended score in MICRO range (0.382 to 0.618) selects MICRO."""
        # Need: 0.382 ≤ blended < 0.618
        # axiom=60, e_score=60, oracle=0.0
        # blended = (60/100)*0.4 + (60/100)*0.3 + 0.0*0.3 = 0.24 + 0.18 = 0.42 ✓
        mock_axiom_monitor.active_count.return_value = 1
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY"]
        mock_axiom_monitor.get_maturity.return_value = 60.0
        mock_escore_tracker.get_score.return_value = {"q": 60.0}
        scheduler.oracle_dog.analyze = AsyncMock()
        oracle_judgment = MagicMock()
        oracle_judgment.confidence = 0.0
        scheduler.oracle_dog.analyze.return_value = oracle_judgment

        level = await scheduler.select_level(sample_cell)
        assert level == ConsciousnessLevel.MICRO

    @pytest.mark.asyncio
    async def test_threshold_82_percent_macro(
        self, scheduler, mock_axiom_monitor, mock_escore_tracker, sample_cell
    ):
        """Blended score exactly at 0.82 (82%) selects MACRO (no 4 axioms)."""
        # All signals = 82
        mock_axiom_monitor.active_count.return_value = 2  # Not 4
        mock_axiom_monitor.active_axioms.return_value = ["AUTONOMY", "SYMBIOSIS"]
        mock_axiom_monitor.get_maturity.return_value = 82.0
        mock_escore_tracker.get_score.return_value = {"q": 82.0}
        scheduler.oracle_dog.analyze = AsyncMock()
        oracle_judgment = MagicMock()
        oracle_judgment.confidence = 0.82
        scheduler.oracle_dog.analyze.return_value = oracle_judgment

        level = await scheduler.select_level(sample_cell)
        assert level == ConsciousnessLevel.MACRO


class TestNoneHandling:
    """Test handling of None dependencies."""

    @pytest.mark.asyncio
    async def test_scheduler_with_none_axiom_monitor(self, mock_escore_tracker, mock_oracle_dog, sample_cell):
        """select_level works with None axiom_monitor."""
        scheduler = ConsciousnessScheduler(None, mock_escore_tracker, mock_oracle_dog)
        level = await scheduler.select_level(sample_cell)
        assert isinstance(level, ConsciousnessLevel)

    @pytest.mark.asyncio
    async def test_scheduler_with_none_escore_tracker(self, mock_axiom_monitor, mock_oracle_dog, sample_cell):
        """select_level works with None escore_tracker."""
        scheduler = ConsciousnessScheduler(mock_axiom_monitor, None, mock_oracle_dog)
        level = await scheduler.select_level(sample_cell)
        assert isinstance(level, ConsciousnessLevel)

    @pytest.mark.asyncio
    async def test_scheduler_with_none_oracle_dog(self, mock_axiom_monitor, mock_escore_tracker, sample_cell):
        """select_level works with None oracle_dog."""
        scheduler = ConsciousnessScheduler(mock_axiom_monitor, mock_escore_tracker, None)
        level = await scheduler.select_level(sample_cell)
        assert isinstance(level, ConsciousnessLevel)
