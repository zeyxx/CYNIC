"""Tests for GASdf burn sensor."""
from __future__ import annotations

from unittest.mock import AsyncMock

import pytest

from cynic.kernel.organism.perception.integrations.gasdf.burn_sensor import GASdfBurnSensor
from cynic.kernel.organism.perception.integrations.gasdf.client import GASdfClient
from cynic.kernel.organism.perception.integrations.gasdf.types import GASdfError, GASdfStats
from cynic.kernel.protocol.lnsp.types import LNSPMessage, ObservationType


class TestGASdfBurnSensor:
    """Test suite for GASdfBurnSensor."""

    @pytest.fixture
    def mock_client(self) -> AsyncMock:
        """Create a mock GASdfClient."""
        return AsyncMock(spec=GASdfClient)

    async def test_burn_sensor_emits_observation(
        self, mock_client: AsyncMock
    ) -> None:
        """Test that sensor emits LNSP observation on successful stats fetch."""
        mock_stats = GASdfStats(
            total_burned=1000000,
            total_transactions=100,
            burned_formatted="1000000 ASDF",
            treasury={"balance": 1308900},
        )
        mock_client.get_stats.return_value = mock_stats

        sensor = GASdfBurnSensor(
            sensor_id="sensor:gasdf_burn",
            client=mock_client,
            instance_id="instance:test",
        )

        result = await sensor.observe()

        assert result is not None
        assert isinstance(result, LNSPMessage)
        assert result.payload["observation_type"] == ObservationType.ECOSYSTEM_EVENT.value
        assert result.header.source == "sensor:gasdf_burn"

    async def test_burn_sensor_observation_payload(
        self, mock_client: AsyncMock
    ) -> None:
        """Test that observation payload contains stats data."""
        mock_stats = GASdfStats(
            total_burned=5000000,
            total_transactions=500,
            burned_formatted="5000000 ASDF",
            treasury={"balance": 6544500},
        )
        mock_client.get_stats.return_value = mock_stats

        sensor = GASdfBurnSensor(
            sensor_id="sensor:gasdf_burn",
            client=mock_client,
        )

        result = await sensor.observe()

        assert result is not None
        assert "data" in result.payload
        payload_data = result.payload["data"]
        assert payload_data["total_burned"] == 5000000
        assert payload_data["total_transactions"] == 500
        assert payload_data["burned_formatted"] == "5000000 ASDF"

    async def test_burn_sensor_skips_duplicate_stats(
        self, mock_client: AsyncMock
    ) -> None:
        """Test that sensor returns None if stats haven't changed."""
        mock_stats = GASdfStats(
            total_burned=1000000,
            total_transactions=100,
            burned_formatted="1000000 ASDF",
            treasury={"balance": 1308900},
        )
        mock_client.get_stats.return_value = mock_stats

        sensor = GASdfBurnSensor(
            sensor_id="sensor:gasdf_burn",
            client=mock_client,
        )

        # First call should emit
        result1 = await sensor.observe()
        assert result1 is not None

        # Second call with same stats should return None
        result2 = await sensor.observe()
        assert result2 is None

    async def test_burn_sensor_emits_on_stat_change(
        self, mock_client: AsyncMock
    ) -> None:
        """Test that sensor emits when stats change."""
        stats1 = GASdfStats(
            total_burned=1000000,
            total_transactions=100,
            burned_formatted="1000000 ASDF",
            treasury={"balance": 1308900},
        )
        stats2 = GASdfStats(
            total_burned=2000000,
            total_transactions=200,
            burned_formatted="2000000 ASDF",
            treasury={"balance": 2617800},
        )

        sensor = GASdfBurnSensor(
            sensor_id="sensor:gasdf_burn",
            client=mock_client,
        )

        # First observation
        mock_client.get_stats.return_value = stats1
        result1 = await sensor.observe()
        assert result1 is not None

        # Stats change
        mock_client.get_stats.return_value = stats2
        result2 = await sensor.observe()
        assert result2 is not None
        assert result2.payload["data"]["total_burned"] == 2000000

    async def test_burn_sensor_handles_api_error(
        self, mock_client: AsyncMock
    ) -> None:
        """Test that sensor silently returns None on API error."""
        mock_client.get_stats.side_effect = GASdfError("API unavailable")

        sensor = GASdfBurnSensor(
            sensor_id="sensor:gasdf_burn",
            client=mock_client,
        )

        result = await sensor.observe()
        assert result is None

    async def test_burn_sensor_observation_type(
        self, mock_client: AsyncMock
    ) -> None:
        """Test that observation type is ECOSYSTEM_EVENT."""
        mock_stats = GASdfStats(
            total_burned=1000000,
            total_transactions=100,
            burned_formatted="1000000 ASDF",
            treasury={"balance": 1308900},
        )
        mock_client.get_stats.return_value = mock_stats

        sensor = GASdfBurnSensor(
            sensor_id="sensor:gasdf_burn",
            client=mock_client,
        )

        result = await sensor.observe()

        assert result is not None
        assert result.payload["observation_type"] == ObservationType.ECOSYSTEM_EVENT.value

    async def test_burn_sensor_custom_instance_id(
        self, mock_client: AsyncMock
    ) -> None:
        """Test that sensor respects custom instance_id."""
        mock_stats = GASdfStats(
            total_burned=1000000,
            total_transactions=100,
            burned_formatted="1000000 ASDF",
            treasury={"balance": 1308900},
        )
        mock_client.get_stats.return_value = mock_stats

        sensor = GASdfBurnSensor(
            sensor_id="sensor:gasdf_burn",
            client=mock_client,
            instance_id="instance:custom",
        )

        result = await sensor.observe()

        assert result is not None
        assert result.metadata.instance_id == "instance:custom"
