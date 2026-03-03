"""Tests for SecurityEventRepo (SIEM Foundation — PHASE 2)."""

import pytest
import time
from unittest.mock import AsyncMock

from cynic.kernel.core.storage.interface import SecurityEventRepoInterface
from cynic.kernel.core.storage.surreal import SecurityEventRepo


@pytest.fixture
def mock_db():
    """Create a mock SurrealDB client."""
    db = AsyncMock()
    return db


@pytest.fixture
def security_repo(mock_db):
    """Create a SecurityEventRepo with mock database."""
    return SecurityEventRepo(mock_db)


class TestSecurityEventRepoInterface:
    """Test that SecurityEventRepo implements the interface correctly."""

    def test_repo_implements_interface(self, security_repo):
        """Verify SecurityEventRepo implements SecurityEventRepoInterface."""
        assert isinstance(security_repo, SecurityEventRepoInterface)

    @pytest.mark.asyncio
    async def test_save_event_returns_id(self, security_repo, mock_db):
        """Test save_event returns an event ID."""
        event = {
            "type": "judgment_created",
            "actor_id": "dog_1",
            "q_score": 75.5,
        }

        event_id = await security_repo.save_event(event)

        assert event_id is not None
        assert isinstance(event_id, str)
        assert len(event_id) > 0
        # Verify DB create was called
        mock_db.create.assert_called_once()

    @pytest.mark.asyncio
    async def test_save_event_includes_timestamp(self, security_repo, mock_db):
        """Test save_event adds timestamp to event."""
        event = {
            "type": "governance_vote",
            "actor_id": "user_123",
        }

        await security_repo.save_event(event)

        # Check what was passed to create
        call_args = mock_db.create.call_args
        saved_event = call_args[0][1]  # Second argument is the event dict

        assert "timestamp" in saved_event
        assert isinstance(saved_event["timestamp"], (int, float))

    @pytest.mark.asyncio
    async def test_get_event_found(self, security_repo, mock_db):
        """Test get_event returns event when found."""
        mock_db.select.return_value = [
            {
                "result": [
                    {
                        "id": "evt-1",
                        "type": "judgment_created",
                        "actor_id": "dog_1",
                    }
                ]
            }
        ]

        event = await security_repo.get_event("evt-1")

        assert event is not None
        assert event["id"] == "evt-1"
        assert event["type"] == "judgment_created"

    @pytest.mark.asyncio
    async def test_get_event_not_found(self, security_repo, mock_db):
        """Test get_event returns None when event not found."""
        mock_db.select.return_value = []

        event = await security_repo.get_event("nonexistent")

        assert event is None

    @pytest.mark.asyncio
    async def test_list_events_no_filters(self, security_repo, mock_db):
        """Test list_events without filters."""
        mock_db.query.return_value = [
            {
                "result": [
                    {"id": "evt-1", "type": "judgment_created"},
                    {"id": "evt-2", "type": "governance_vote"},
                ]
            }
        ]

        events = await security_repo.list_events()

        assert len(events) == 2
        assert events[0]["id"] == "evt-1"
        mock_db.query.assert_called_once()

    @pytest.mark.asyncio
    async def test_list_events_with_type_filter(self, security_repo, mock_db):
        """Test list_events with type filter."""
        mock_db.query.return_value = [
            {"result": [{"id": "evt-1", "type": "judgment_created"}]}
        ]

        events = await security_repo.list_events(filters={"type": "judgment_created"})

        assert len(events) == 1
        # Verify query included type filter
        call_args = mock_db.query.call_args
        query_str = call_args[0][0]
        assert "type = $type" in query_str

    @pytest.mark.asyncio
    async def test_list_events_with_actor_id_filter(self, security_repo, mock_db):
        """Test list_events with actor_id filter."""
        mock_db.query.return_value = [
            {"result": [{"id": "evt-1", "actor_id": "dog_1"}]}
        ]

        events = await security_repo.list_events(filters={"actor_id": "dog_1"})

        assert len(events) == 1
        call_args = mock_db.query.call_args
        query_str = call_args[0][0]
        assert "actor_id = $actor_id" in query_str

    @pytest.mark.asyncio
    async def test_list_events_with_timestamp_range(self, security_repo, mock_db):
        """Test list_events with timestamp range filter."""
        mock_db.query.return_value = [{"result": [{"id": "evt-1", "timestamp": 1000}]}]

        now = time.time()
        events = await security_repo.list_events(
            filters={
                "timestamp_gte": now - 3600,
                "timestamp_lte": now,
            }
        )

        assert len(events) == 1
        call_args = mock_db.query.call_args
        query_str = call_args[0][0]
        assert "timestamp >= $timestamp_gte" in query_str
        assert "timestamp <= $timestamp_lte" in query_str

    @pytest.mark.asyncio
    async def test_list_events_pagination(self, security_repo, mock_db):
        """Test list_events respects limit and offset."""
        mock_db.query.return_value = []

        await security_repo.list_events(limit=50, offset=100)

        call_args = mock_db.query.call_args
        params = call_args[0][1]
        assert params["limit"] == 50
        assert params["offset"] == 100

    @pytest.mark.asyncio
    async def test_correlate_finds_related_events(self, security_repo, mock_db):
        """Test correlate finds events from same actor within time window."""
        mock_db.query.return_value = [
            {
                "result": [
                    {"id": "evt-1", "actor_id": "dog_1", "timestamp": 1000},
                    {"id": "evt-2", "actor_id": "dog_1", "timestamp": 1010},
                ]
            }
        ]

        event = {"actor_id": "dog_1", "timestamp": 1005}
        related = await security_repo.correlate(event, window_seconds=300)

        assert len(related) == 2
        mock_db.query.assert_called_once()

    @pytest.mark.asyncio
    async def test_correlate_no_actor_id(self, security_repo, mock_db):
        """Test correlate returns empty list if event has no actor_id."""
        event = {"timestamp": 1000}  # No actor_id

        related = await security_repo.correlate(event)

        assert related == []
        mock_db.query.assert_not_called()

    @pytest.mark.asyncio
    async def test_correlate_window_boundary(self, security_repo, mock_db):
        """Test correlate respects time window boundaries."""
        mock_db.query.return_value = []

        event = {"actor_id": "dog_1", "timestamp": 1000}
        await security_repo.correlate(event, window_seconds=500)

        call_args = mock_db.query.call_args
        params = call_args[0][1]
        # Check time window is correct
        assert params["time_start"] == 500
        assert params["time_end"] == 1000

    @pytest.mark.asyncio
    async def test_detect_anomaly_basic(self, security_repo, mock_db):
        """Test detect_anomaly identifies anomalies."""
        event = {"field1": 250}  # 2.5x baseline
        baselines = {"field1": 100}  # Expected value

        anomaly_result = await security_repo.detect_anomaly(event, baselines)

        assert anomaly_result["is_anomalous"] is True
        assert "field1" in anomaly_result["anomalies"]

    @pytest.mark.asyncio
    async def test_detect_anomaly_no_anomalies(self, security_repo, mock_db):
        """Test detect_anomaly finds no anomalies when values match."""
        event = {"field1": 100}
        baselines = {"field1": 100}

        anomaly_result = await security_repo.detect_anomaly(event, baselines)

        assert anomaly_result["is_anomalous"] is False
        assert len(anomaly_result["anomalies"]) == 0

    @pytest.mark.asyncio
    async def test_detect_anomaly_threshold(self, security_repo, mock_db):
        """Test detect_anomaly threshold is 2x difference."""
        event = {"field1": 150}
        baselines = {"field1": 100}  # 1.5x, should not be anomalous

        anomaly_result = await security_repo.detect_anomaly(event, baselines)

        assert anomaly_result["is_anomalous"] is False

        # Now test with 2.5x
        event = {"field1": 250}
        anomaly_result = await security_repo.detect_anomaly(event, baselines)

        assert anomaly_result["is_anomalous"] is True

    @pytest.mark.asyncio
    async def test_detect_anomaly_no_baselines(self, security_repo, mock_db):
        """Test detect_anomaly with no baselines provided."""
        event = {"field1": 200}

        anomaly_result = await security_repo.detect_anomaly(event, None)

        assert anomaly_result["is_anomalous"] is False
        assert len(anomaly_result["anomalies"]) == 0

    @pytest.mark.asyncio
    async def test_detect_anomaly_score(self, security_repo, mock_db):
        """Test detect_anomaly calculates anomaly score."""
        event = {"field1": 250, "field2": 250}
        baselines = {"field1": 100, "field2": 100}

        anomaly_result = await security_repo.detect_anomaly(event, baselines)

        assert "anomaly_score" in anomaly_result
        assert 0 <= anomaly_result["anomaly_score"] <= 1.0
        # 2 anomalies out of 2 baselines = 1.0
        assert anomaly_result["anomaly_score"] == 1.0

    @pytest.mark.asyncio
    async def test_get_stats(self, security_repo, mock_db):
        """Test get_stats returns event statistics."""
        # Mock the type count query
        mock_db.query.side_effect = [
            [
                {
                    "result": [
                        {"event_type": "judgment_created", "count_by_type": 10},
                        {"event_type": "governance_vote", "count_by_type": 5},
                    ]
                }
            ],
            [{"result": [{"total": 15}]}],
        ]

        stats = await security_repo.get_stats()

        assert "total_events" in stats
        assert "by_type" in stats
        assert stats["total_events"] == 15
        assert stats["by_type"]["judgment_created"] == 10
        assert stats["by_type"]["governance_vote"] == 5

    @pytest.mark.asyncio
    async def test_get_stats_empty_database(self, security_repo, mock_db):
        """Test get_stats with no events."""
        mock_db.query.side_effect = [
            [{"result": []}],
            [{"result": [{"total": 0}]}],
        ]

        stats = await security_repo.get_stats()

        assert stats["total_events"] == 0
        assert stats["by_type"] == {}


class TestSecurityEventIntegration:
    """Integration tests for security event workflow."""

    @pytest.mark.asyncio
    async def test_save_then_get_event(self, security_repo, mock_db):
        """Test save event and retrieve it."""
        # Setup mock for save and get
        saved_id = None

        async def capture_create(record_id, event_data):
            nonlocal saved_id
            saved_id = record_id.split(":")[-1]

        mock_db.create = AsyncMock(side_effect=capture_create)
        mock_db.select = AsyncMock(
            return_value=[
                {
                    "result": [
                        {
                            "id": "evt-123",
                            "type": "judgment_created",
                            "actor_id": "dog_1",
                            "timestamp": 1000,
                        }
                    ]
                }
            ]
        )

        # Save event
        event = {"type": "judgment_created", "actor_id": "dog_1"}
        event_id = await security_repo.save_event(event)

        # Get event
        retrieved = await security_repo.get_event(event_id)

        assert retrieved is not None
        assert retrieved["type"] == "judgment_created"

    @pytest.mark.asyncio
    async def test_event_workflow_save_list_correlate(self, security_repo, mock_db):
        """Test complete workflow: save → list → correlate."""
        # Save
        mock_db.create = AsyncMock()

        event = {
            "type": "judgment_created",
            "actor_id": "dog_1",
            "timestamp": 1000,
        }
        event_id = await security_repo.save_event(event)
        assert event_id is not None

        # List events
        mock_db.query.return_value = [{"result": [event]}]
        events = await security_repo.list_events(filters={"actor_id": "dog_1"})
        assert len(events) > 0

        # Correlate events
        mock_db.query.return_value = [{"result": [event]}]
        related = await security_repo.correlate(event)
        assert len(related) > 0


class TestSecurityEventEdgeCases:
    """Test edge cases and error handling."""

    @pytest.mark.asyncio
    async def test_save_event_with_special_characters(self, security_repo, mock_db):
        """Test saving event with special characters in fields."""
        event = {
            "type": "attack_detected",
            "message": "SQL injection attempt: ' OR '1'='1",
            "actor_id": "attacker:123:456",
        }

        event_id = await security_repo.save_event(event)

        assert event_id is not None
        mock_db.create.assert_called_once()

    @pytest.mark.asyncio
    async def test_list_events_multiple_filters(self, security_repo, mock_db):
        """Test list_events with multiple filters combined."""
        mock_db.query.return_value = []

        await security_repo.list_events(
            filters={
                "type": "judgment_created",
                "actor_id": "dog_1",
                "timestamp_gte": 1000,
                "timestamp_lte": 2000,
            }
        )

        call_args = mock_db.query.call_args
        query_str = call_args[0][0]
        assert "type = $type" in query_str
        assert "actor_id = $actor_id" in query_str
        assert "timestamp >= $timestamp_gte" in query_str
        assert "timestamp <= $timestamp_lte" in query_str

    @pytest.mark.asyncio
    async def test_correlate_with_default_window(self, security_repo, mock_db):
        """Test correlate uses default 300 second window."""
        mock_db.query.return_value = []

        event = {"actor_id": "dog_1", "timestamp": 1000}
        await security_repo.correlate(event)

        call_args = mock_db.query.call_args
        params = call_args[0][1]
        assert params["time_start"] == 700  # 1000 - 300
        assert params["time_end"] == 1000

    @pytest.mark.asyncio
    async def test_detect_anomaly_with_zero_baseline(self, security_repo, mock_db):
        """Test detect_anomaly handles zero baseline values."""
        event = {"field1": 100}
        baselines = {"field1": 0}  # Zero baseline

        anomaly_result = await security_repo.detect_anomaly(event, baselines)

        # Should not crash, should handle gracefully
        assert "anomaly_score" in anomaly_result
