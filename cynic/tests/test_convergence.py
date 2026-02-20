"""
Test ConvergenceValidator — Announcement vs Reality tracking.

Phase 3: Observability system for validating announced vs actual behavior.
"""
import pytest
from cynic.core.convergence import (
    Announcement,
    Outcome,
    Convergence,
    ConvergenceValidator,
)


class TestAnnouncement:
    """Test Announcement dataclass."""

    def test_announcement_creation(self):
        ann = Announcement(
            announcement_id="ann_000001",
            timestamp=1234567890.0,
            announced_verdict="HOWL",
            announced_q_score=85.5,
            cell_id="code:judge:1",
            confidence=0.618,
        )
        assert ann.announced_verdict == "HOWL"
        assert ann.announced_q_score == 85.5
        assert "HOWL Q=85.5" in repr(ann)

    def test_announcement_without_cell_id(self):
        ann = Announcement(
            announcement_id="ann_000002",
            timestamp=1234567890.0,
            announced_verdict="GROWL",
            announced_q_score=45.0,
        )
        assert ann.cell_id is None
        assert ann.announced_verdict == "GROWL"


class TestOutcome:
    """Test Outcome dataclass."""

    def test_outcome_creation(self):
        out = Outcome(
            outcome_id="out_000001",
            timestamp=1234567895.0,
            actual_verdict="HOWL",
            actual_q_score=83.2,
            actual_action_executed=True,
        )
        assert out.actual_verdict == "HOWL"
        assert out.actual_action_executed is True
        assert "HOWL Q=83.2" in repr(out)

    def test_outcome_with_error(self):
        out = Outcome(
            outcome_id="out_000002",
            timestamp=1234567895.0,
            actual_verdict="BARK",
            actual_q_score=15.0,
            error="execution failed",
        )
        assert out.error == "execution failed"
        assert out.actual_verdict == "BARK"


class TestConvergence:
    """Test Convergence matching logic."""

    def test_convergence_match(self):
        ann = Announcement(
            announcement_id="ann_000001",
            timestamp=1234567890.0,
            announced_verdict="HOWL",
            announced_q_score=85.5,
        )
        out = Outcome(
            outcome_id="out_000001",
            timestamp=1234567895.0,
            actual_verdict="HOWL",
            actual_q_score=83.2,
        )
        conv = Convergence(
            convergence_id="conv_000001",
            announcement=ann,
            outcome=out,
        )
        assert conv.match is True
        assert conv.latency_ms == 5000.0
        assert "MATCH" in repr(conv)

    def test_convergence_diverge(self):
        ann = Announcement(
            announcement_id="ann_000002",
            timestamp=1234567890.0,
            announced_verdict="HOWL",
            announced_q_score=85.5,
        )
        out = Outcome(
            outcome_id="out_000002",
            timestamp=1234567895.0,
            actual_verdict="WAG",
            actual_q_score=68.2,
        )
        conv = Convergence(
            convergence_id="conv_000002",
            announcement=ann,
            outcome=out,
        )
        assert conv.match is False
        assert "DIVERGE" in repr(conv)


class TestConvergenceValidator:
    """Test ConvergenceValidator core functionality."""

    def test_init_default_capacity(self):
        validator = ConvergenceValidator()
        assert validator.capacity == 89  # F(11)
        assert len(validator._convergences) == 0

    def test_announce_returns_id(self):
        validator = ConvergenceValidator()
        ann_id = validator.announce(
            verdict="HOWL",
            q_score=85.5,
            cell_id="code:judge:1",
            confidence=0.618,
        )
        assert ann_id.startswith("ann_")
        assert ann_id in validator._announcements
        assert validator._total_announcements == 1

    def test_announce_multiple(self):
        validator = ConvergenceValidator()
        id1 = validator.announce("HOWL", 85.5)
        id2 = validator.announce("GROWL", 40.0)
        id3 = validator.announce("WAG", 70.0)
        assert id1 != id2 != id3
        assert validator._total_announcements == 3

    def test_record_outcome_creates_convergence(self):
        validator = ConvergenceValidator()
        ann_id = validator.announce("HOWL", 85.5, cell_id="code:1")
        conv = validator.record_outcome(
            announcement_id=ann_id,
            actual_verdict="HOWL",
            actual_q_score=83.2,
            action_executed=True,
        )
        assert conv.match is True
        assert len(validator._convergences) == 1

    def test_record_outcome_divergence(self):
        validator = ConvergenceValidator()
        ann_id = validator.announce("HOWL", 85.5)
        conv = validator.record_outcome(
            announcement_id=ann_id,
            actual_verdict="BARK",
            actual_q_score=15.0,
        )
        assert conv.match is False
        assert validator._total_matches == 0

    def test_record_outcome_unknown_announcement(self):
        """Recording outcome for unknown announcement should still work."""
        validator = ConvergenceValidator()
        conv = validator.record_outcome(
            announcement_id="unknown_id",
            actual_verdict="WAG",
            actual_q_score=68.0,
        )
        # Should create convergence with UNKNOWN placeholder
        assert conv.announcement.announced_verdict == "UNKNOWN"
        assert len(validator._convergences) == 1

    def test_rolling_cap(self):
        """Test that convergences cap at capacity."""
        validator = ConvergenceValidator(capacity=5)
        for i in range(10):
            ann_id = validator.announce(f"VERDICT_{i}", float(i * 10))
            validator.record_outcome(
                announcement_id=ann_id,
                actual_verdict=f"VERDICT_{i}",
                actual_q_score=float(i * 10),
            )
        # Should only keep last 5
        assert len(validator._convergences) == 5

    def test_recent(self):
        """Test recent() method."""
        validator = ConvergenceValidator()
        for i in range(8):
            ann_id = validator.announce("WAG", 68.0)
            validator.record_outcome(ann_id, "WAG", 68.0)
        recent = validator.recent(limit=3)
        assert len(recent) == 3
        assert recent[0].convergence_id.startswith("conv_")

    def test_stats(self):
        """Test stats() method."""
        validator = ConvergenceValidator()
        ann_id1 = validator.announce("HOWL", 85.0)
        validator.record_outcome(ann_id1, "HOWL", 84.0)  # match
        ann_id2 = validator.announce("WAG", 68.0)
        validator.record_outcome(ann_id2, "GROWL", 40.0)  # diverge

        stats = validator.stats()
        assert stats["total_announced"] == 2
        assert stats["total_outcomes"] == 2
        assert stats["total_matches"] == 1
        assert stats["convergence_rate"] == 50.0
        assert len(stats["recent"]) == 2

    def test_stats_empty_validator(self):
        """Test stats() on empty validator."""
        validator = ConvergenceValidator()
        stats = validator.stats()
        assert stats["total_announced"] == 0
        assert stats["total_outcomes"] == 0
        assert stats["convergence_rate"] == 0.0


class TestConvergenceIntegration:
    """Integration tests for full announcement→outcome cycle."""

    def test_full_cycle_match(self):
        """Full cycle: announce → record_outcome → verify match."""
        validator = ConvergenceValidator()

        # Organism announces judgment
        ann_id = validator.announce(
            verdict="HOWL",
            q_score=85.5,
            cell_id="code:judge:analysis",
            action="ACCEPT",
            confidence=0.618,
        )

        # Later, record the actual outcome
        conv = validator.record_outcome(
            announcement_id=ann_id,
            actual_verdict="HOWL",
            actual_q_score=84.2,
            action_executed=True,
        )

        # Verify
        assert conv.match is True
        assert conv.latency_ms > 0
        stats = validator.stats()
        assert stats["convergence_rate"] == 100.0

    def test_full_cycle_divergence(self):
        """Full cycle: announce something different happens."""
        validator = ConvergenceValidator()

        ann_id = validator.announce(
            verdict="WAG",
            q_score=70.0,
            cell_id="code:test",
        )

        conv = validator.record_outcome(
            announcement_id=ann_id,
            actual_verdict="GROWL",
            actual_q_score=45.0,
            error="test execution failed",
        )

        assert conv.match is False
        assert conv.outcome.error == "test execution failed"
        stats = validator.stats()
        assert stats["convergence_rate"] == 0.0
