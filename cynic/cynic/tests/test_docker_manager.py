"""
Tests for DockerManager — native Python Docker SDK management.

Paradigm: Docker = CYNIC capability, no CLI friction.
"""
import asyncio
import json
import pytest
from pathlib import Path
from unittest.mock import Mock, AsyncMock, MagicMock, patch

from cynic.deployment.docker_manager import DockerManager, ContainerStatus, StackStatus


class TestDockerManagerInitialize:
    """Test Docker API connection."""

    @pytest.mark.asyncio
    async def test_initialize_success(self):
        """Should connect to Docker daemon."""
        mgr = DockerManager()

        with patch("docker.from_env") as mock_from_env:
            mock_client = MagicMock()
            mock_client.ping = Mock(return_value=None)  # ping() doesn't raise
            mock_from_env.return_value = mock_client

            result = await mgr.initialize()

            assert result is True
            assert mgr.client is not None
            mock_from_env.assert_called_once()

    @pytest.mark.asyncio
    async def test_initialize_failure(self):
        """Should handle connection failure gracefully."""
        mgr = DockerManager()

        with patch("docker.from_env") as mock_from_env:
            mock_from_env.side_effect = Exception("Daemon not running")

            result = await mgr.initialize()

            assert result is False
            assert mgr.client is None


class TestDockerManagerStatus:
    """Test container status reporting."""

    @pytest.mark.asyncio
    async def test_get_status_all_healthy(self):
        """Should report all healthy when all containers running and healthy."""
        mgr = DockerManager()

        # Mock containers
        mock_containers = {}
        for service in ["cynic-kernel", "cynic-ollama", "cynic-surrealdb"]:
            mock_container = MagicMock()
            mock_container.name = service
            mock_container.status = "running"
            mock_container.image.tags = [f"{service}:latest"]
            mock_container.attrs = {
                "State": {
                    "Health": {"Status": "healthy"},
                    "StartedAt": "2026-02-20T10:00:00.000Z",
                },
                "HostConfig": {"PortBindings": {"8000/tcp": [{"HostPort": "8000"}]}},
            }
            mock_containers[service] = mock_container

        mock_client = MagicMock()
        mock_client.containers.get = lambda name: mock_containers[name]
        mgr.client = mock_client

        status = await mgr.get_status()

        assert status.all_healthy is True
        assert len(status.failures) == 0
        assert len(status.containers) == 3
        for service in mgr.CYNIC_SERVICES:
            assert service in status.containers
            assert status.containers[service].status == "running"

    @pytest.mark.asyncio
    async def test_get_status_unhealthy(self):
        """Should report failures when service down."""
        mgr = DockerManager()

        mock_containers = {}
        # Only create 2 services, 1 will be "not_found"
        for service in ["cynic-kernel", "cynic-ollama"]:
            mock_container = MagicMock()
            mock_container.name = service
            mock_container.status = "running"
            mock_container.image.tags = [f"{service}:latest"]
            mock_container.attrs = {
                "State": {"Health": {"Status": "healthy"}},
                "HostConfig": {"PortBindings": {}},
            }
            mock_containers[service] = mock_container

        def get_container(name):
            if name in mock_containers:
                return mock_containers[name]
            from docker.errors import NotFound

            raise NotFound(f"{name} not found")

        mock_client = MagicMock()
        mock_client.containers.get = get_container
        mgr.client = mock_client

        status = await mgr.get_status()

        assert status.all_healthy is False
        assert len(status.failures) > 0
        assert any("cynic-surrealdb" in f for f in status.failures)


class TestDockerManagerRestart:
    """Test service restart."""

    @pytest.mark.asyncio
    async def test_restart_service_success(self):
        """Should restart service and wait for health."""
        mgr = DockerManager()

        mock_container = MagicMock()
        mock_container.status = "running"
        mock_container.attrs = {"State": {"Health": {"Status": "healthy"}}}

        def reload():
            mock_container.status = "running"

        mock_container.reload = reload
        mock_container.restart = Mock()

        mock_client = MagicMock()
        mock_client.containers.get = Mock(return_value=mock_container)
        mgr.client = mock_client

        result = await mgr.restart_service("cynic-kernel", wait_healthy_s=5.0)

        assert result is True
        mock_container.restart.assert_called_once()

    @pytest.mark.asyncio
    async def test_restart_service_timeout(self):
        """Should timeout if service not healthy."""
        mgr = DockerManager()

        mock_container = MagicMock()
        mock_container.status = "restarting"  # Never becomes healthy
        mock_container.attrs = {"State": {}}

        def reload():
            pass  # Status stays "restarting"

        mock_container.reload = reload
        mock_container.restart = Mock()

        mock_client = MagicMock()
        mock_client.containers.get = Mock(return_value=mock_container)
        mgr.client = mock_client

        result = await mgr.restart_service("cynic-kernel", wait_healthy_s=1.0)

        assert result is False


class TestDockerManagerStatusPersistence:
    """Test status logging."""

    def test_write_status_log(self, tmp_path):
        """Should persist status to JSON file."""
        mgr = DockerManager()
        mgr.log_dir = tmp_path / "deployments"
        mgr.log_dir.mkdir(parents=True, exist_ok=True)

        status = StackStatus(
            timestamp="2026-02-20T10:00:00",
            containers={"cynic-kernel": ContainerStatus(name="cynic-kernel", image="kernel:latest", status="running")},
            all_healthy=True,
            failures=[],
        )

        mgr._write_status_log(status)

        # Should have written a file
        files = list(mgr.log_dir.glob("*-status.json"))
        assert len(files) == 1

        # Verify content
        with open(files[0]) as f:
            data = json.load(f)
            assert data["all_healthy"] is True
            assert "cynic-kernel" in data["containers"]


class TestDockerManagerLogsRetrieval:
    """Test log retrieval."""

    @pytest.mark.asyncio
    async def test_get_logs_success(self):
        """Should retrieve logs from container."""
        mgr = DockerManager()

        mock_container = MagicMock()
        mock_container.logs = Mock(return_value="[INFO] Service started\n[INFO] Ready")

        mock_client = MagicMock()
        mock_client.containers.get = Mock(return_value=mock_container)
        mgr.client = mock_client

        logs = await mgr.get_logs("cynic-kernel", lines=50)

        assert logs is not None
        assert "Service started" in logs
        mock_container.logs.assert_called_once_with(tail=50, decode=True)

    @pytest.mark.asyncio
    async def test_get_logs_failure(self):
        """Should handle log retrieval errors."""
        mgr = DockerManager()

        mock_client = MagicMock()
        mock_client.containers.get = Mock(side_effect=Exception("Container not found"))
        mgr.client = mock_client

        logs = await mgr.get_logs("cynic-kernel")

        assert logs is None


@pytest.mark.asyncio
async def test_container_status_serialization():
    """Should serialize ContainerStatus to dict."""
    cs = ContainerStatus(
        name="cynic-kernel",
        image="kernel:latest",
        status="running",
        health="healthy",
        ports=["8000→8000"],
        uptime_s=3600.0,
    )

    data = cs.to_dict()

    assert data["name"] == "cynic-kernel"
    assert data["status"] == "running"
    assert data["health"] == "healthy"
    assert data["uptime_s"] == 3600.0
