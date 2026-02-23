"""
Mock Docker Client for Test Environments

Provides a minimal Docker API mock when docker-py is not installed.
Used as a fallback in docker_manager.py when running in test environments.

"Le chien simule sans avoir besoin du vrai Docker" — κυνικός
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Optional, List, Dict, Any, Union


class DockerException(Exception):
    """Mock base Docker exception (simulates docker.errors.DockerException)."""
    pass


class NotFound(DockerException):
    """Mock NotFound exception (simulates docker.errors.NotFound)."""
    pass


class ContainerStatus(str, Enum):
    """Mock container status constants."""
    CREATED = "created"
    RUNNING = "running"
    PAUSED = "paused"
    STOPPED = "stopped"
    EXITED = "exited"
    DEAD = "dead"


@dataclass
class MockContainer:
    """Minimal mock container object."""
    id: str = "mock-container-id"
    name: str = "mock-container"
    status: str = ContainerStatus.RUNNING
    image: str = "mock-image:latest"
    ports: List[str] = field(default_factory=list)
    attrs: Dict[str, Any] = field(default_factory=lambda: {
        "State": {
            "Health": {"Status": "healthy"},
            "StartedAt": "2026-02-20T10:00:00.000Z",
        },
        "HostConfig": {"PortBindings": {}},
    })

    def reload(self) -> None:
        """Mock reload operation (no-op)."""
        pass

    def restart(self, timeout: int = 10) -> None:
        """Mock restart operation (no-op)."""
        pass

    def logs(self, tail: int = 50, decode: bool = True) -> Union[str, bytes]:
        """Return mock logs."""
        return "[MOCK] No logs available in mock environment\n"


@dataclass
class MockContainerCollection:
    """Minimal mock containers collection."""
    _containers: Dict[str, MockContainer] = field(default_factory=dict)

    def get(self, name: str) -> MockContainer:
        """Get a container by name (raises NotFound if not exists)."""
        if name in self._containers:
            return self._containers[name]
        # Simulate docker.errors.NotFound behavior
        raise NotFound(f"Container '{name}' not found in mock environment")

    def add(self, name: str, container: MockContainer) -> None:
        """Add a container to the mock collection."""
        self._containers[name] = container


@dataclass
class MockImage:
    """Minimal mock image object."""
    tags: List[str] = field(default_factory=lambda: ["mock-image:latest"])


@dataclass
class MockImageCollection:
    """Minimal mock images collection (stub)."""
    pass


@dataclass
class MockClient:
    """Mock Docker client for test environments.

    Provides basic container and image stubs when docker-py is not available.
    """
    containers: MockContainerCollection = field(default_factory=MockContainerCollection)
    images: MockImageCollection = field(default_factory=MockImageCollection)

    def ping(self) -> None:
        """Mock ping (always succeeds in test environment)."""
        pass


def from_env() -> MockClient:
    """Factory function to create a mock Docker client.

    Returns:
        MockClient: A minimal Docker client mock suitable for testing.
    """
    return MockClient()


# Create a namespace mock for docker.errors (for compatibility with real docker module)
class _ErrorsNamespace:
    """Mock errors namespace (simulates docker.errors module)."""
    DockerException = DockerException
    NotFound = NotFound


errors = _ErrorsNamespace()
