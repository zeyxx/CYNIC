"""Tests for Hub client — uses mock HTTP responses."""
import pytest
from unittest.mock import patch, MagicMock

from hub_client import HubClient


class TestHubClient:
    def test_get_attribution_success(self):
        client = HubClient("http://127.0.0.1:40770")
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = {"source": "agent:navigator"}
        with patch("hub_client.requests.get", return_value=mock_resp):
            result = client.get_attribution("https://x.com/graphql/search", 1234567890.0)
        assert result == "agent:navigator"

    def test_get_attribution_hub_down(self):
        client = HubClient("http://127.0.0.1:40770")
        with patch("hub_client.requests.get", side_effect=Exception("connection refused")):
            result = client.get_attribution("https://x.com/graphql/search", 1234567890.0)
        assert result == "unknown"

    def test_create_tab(self):
        client = HubClient("http://127.0.0.1:40770")
        mock_resp = MagicMock()
        mock_resp.status_code = 201
        mock_resp.json.return_value = {"tab_id": "hub-123", "cdp_target_id": "t-123"}
        with patch("hub_client.requests.post", return_value=mock_resp):
            result = client.create_tab("agent:navigator", "https://x.com/search?q=test")
        assert result["tab_id"] == "hub-123"

    def test_create_tab_hub_down(self):
        client = HubClient("http://127.0.0.1:40770")
        with patch("hub_client.requests.post", side_effect=Exception("connection refused")):
            result = client.create_tab("agent:navigator", "https://x.com")
        assert result is None

    def test_release_tab(self):
        client = HubClient("http://127.0.0.1:40770")
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        with patch("hub_client.requests.delete", return_value=mock_resp):
            result = client.release_tab("hub-123")
        assert result is True

    def test_release_tab_hub_down(self):
        client = HubClient("http://127.0.0.1:40770")
        with patch("hub_client.requests.delete", side_effect=Exception("connection refused")):
            result = client.release_tab("hub-123")
        assert result is False
