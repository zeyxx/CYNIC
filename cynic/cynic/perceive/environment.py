"""
EnvironmentIntrospector — CYNIC observes its habitat

CYNIC knows its playground like it knows itself:
  - Filesystem (C:, D:, free space)
  - Docker status (service, volumes, containers)
  - Storage (database connections, state)
  - Network (Ollama, PostgreSQL, external APIs)
  - Resources (CPU, memory, disk)

When something is new/unknown, CYNIC asks the user how to adapt.

Axiom: FIDELITY (know thyself) + VERIFY (check reality)
"""

import os
import sys
import json
import psutil
import shutil
import logging
from pathlib import Path
from typing import Dict, Any, Optional
from datetime import datetime
from dataclasses import dataclass, asdict

logger = logging.getLogger(__name__)

# Try Docker SDK (graceful fallback if unavailable)
try:
    import docker
    DOCKER_AVAILABLE = True
except ImportError:
    DOCKER_AVAILABLE = False

# Try aiohttp for async HTTP (fallback to sync if unavailable)
try:
    import aiohttp
    AIOHTTP_AVAILABLE = True
except ImportError:
    AIOHTTP_AVAILABLE = False


@dataclass
class FileSystemStatus:
    """Filesystem reality."""
    c_drive_total_gb: float
    c_drive_free_gb: float
    c_drive_used_percent: float
    d_drive_total_gb: float
    d_drive_free_gb: float
    d_drive_used_percent: float
    cynic_home: str
    cynic_home_exists: bool
    cynic_home_size_mb: float


@dataclass
class DockerStatus:
    """Docker daemon and container status."""
    daemon_running: bool
    daemon_error: Optional[str]
    containers_count: int
    containers_running: int
    volumes_count: int
    volumes: list


@dataclass
class StorageStatus:
    """Database and persistence status."""
    postgres_available: bool
    postgres_error: Optional[str]
    surrealdb_available: bool
    surrealdb_error: Optional[str]
    state_files_count: int


@dataclass
class NetworkStatus:
    """Network services availability."""
    ollama_available: bool
    ollama_url: Optional[str]
    ollama_error: Optional[str]


@dataclass
class ResourceStatus:
    """CPU, memory, disk resources."""
    cpu_percent: float
    memory_percent: float
    memory_available_gb: float


@dataclass
class EnvironmentSnapshot:
    """Complete habitat snapshot."""
    timestamp: str
    filesystem: FileSystemStatus
    docker: DockerStatus
    storage: StorageStatus
    network: NetworkStatus
    resources: ResourceStatus


class EnvironmentIntrospector:
    """CYNIC observes its environment."""

    def __init__(self):
        self.discoveries_log = Path.home() / ".cynic" / "discoveries.json"
        self.environment_log = Path.home() / ".cynic" / "environments.jsonl"

    async def analyze_environment(self) -> EnvironmentSnapshot:
        """Full environmental analysis — CYNIC knows its playground."""
        return EnvironmentSnapshot(
            timestamp=datetime.now().isoformat(),
            filesystem=self._check_filesystem(),
            docker=await self._check_docker(),
            storage=await self._check_storage(),
            network=await self._check_network(),
            resources=self._check_resources(),
        )

    def _check_filesystem(self) -> FileSystemStatus:
        """Check C: and D: drives with error logging."""
        c_stat = None
        d_stat = None

        try:
            c_stat = shutil.disk_usage("C:/")
        except (OSError, ValueError) as e:
            logger.warning(f"Cannot read C: drive: {e}")

        try:
            d_stat = shutil.disk_usage("D:/")
        except (OSError, ValueError) as e:
            logger.warning(f"Cannot read D: drive: {e}")

        cynic_home = Path.home() / ".cynic"
        cynic_size_mb = self._calculate_directory_size(cynic_home)

        def safe_percent(used: float, total: float) -> float:
            """Safely calculate percentage (guards division by zero)."""
            return (100 - (used / total * 100)) if total > 0 else 0.0

        return FileSystemStatus(
            c_drive_total_gb=c_stat.total / 1e9 if c_stat else 0,
            c_drive_free_gb=c_stat.free / 1e9 if c_stat else 0,
            c_drive_used_percent=safe_percent(c_stat.free, c_stat.total) if c_stat else 0,
            d_drive_total_gb=d_stat.total / 1e9 if d_stat else 0,
            d_drive_free_gb=d_stat.free / 1e9 if d_stat else 0,
            d_drive_used_percent=safe_percent(d_stat.free, d_stat.total) if d_stat else 0,
            cynic_home=str(cynic_home),
            cynic_home_exists=cynic_home.exists(),
            cynic_home_size_mb=cynic_size_mb,
        )

    async def _check_docker(self) -> DockerStatus:
        """Check Docker daemon and containers."""
        if not DOCKER_AVAILABLE:
            return DockerStatus(
                daemon_running=False,
                daemon_error="Docker SDK not installed",
                containers_count=0,
                containers_running=0,
                volumes_count=0,
                volumes=[],
            )

        try:
            client = docker.from_env(timeout=5)
            containers = client.containers.list(all=True)
            volumes = client.volumes.list()

            running = sum(1 for c in containers if c.status == "running")

            return DockerStatus(
                daemon_running=True,
                daemon_error=None,
                containers_count=len(containers),
                containers_running=running,
                volumes_count=len(volumes),
                volumes=[v.name for v in volumes],
            )
        except Exception as e:
            return DockerStatus(
                daemon_running=False,
                daemon_error=str(e),
                containers_count=0,
                containers_running=0,
                volumes_count=0,
                volumes=[],
            )

    async def _check_storage(self) -> StorageStatus:
        """Check PostgreSQL and SurrealDB availability."""
        postgres_available = False
        postgres_error = None

        try:
            import asyncpg

            # Read from environment (never hardcode credentials)
            user = os.getenv("POSTGRES_USER", "cynic")
            password = os.getenv("POSTGRES_PASSWORD")
            database = os.getenv("POSTGRES_DB", "cynic_py")
            host = os.getenv("POSTGRES_HOST", "localhost")
            port = int(os.getenv("POSTGRES_PORT", "5433"))

            if not password:
                postgres_error = "POSTGRES_PASSWORD env var not set (required)"
                return StorageStatus(
                    postgres_available=False,
                    postgres_error=postgres_error,
                    surrealdb_available=False,
                    surrealdb_error=None,
                    state_files_count=0,
                )

            # Try connecting to PostgreSQL
            conn = await asyncpg.connect(
                user=user,
                password=password,
                database=database,
                host=host,
                port=port,
                timeout=2,
            )
            await conn.close()
            postgres_available = True
        except Exception as e:
            postgres_error = str(e)

        # Count state files
        state_dir = Path.home() / ".cynic"
        state_files = list(state_dir.glob("*.json")) + list(state_dir.glob("*.jsonl"))

        return StorageStatus(
            postgres_available=postgres_available,
            postgres_error=postgres_error,
            surrealdb_available=False,  # TODO: implement SurrealDB check
            surrealdb_error="Not implemented yet",
            state_files_count=len(state_files),
        )

    async def _check_network(self) -> NetworkStatus:
        """Check external service availability (non-blocking)."""
        ollama_url = os.getenv("OLLAMA_URL", "http://localhost:11434")
        ollama_available = False
        ollama_error = None

        try:
            if AIOHTTP_AVAILABLE:
                # Use async HTTP client (non-blocking)
                async with aiohttp.ClientSession() as session:
                    try:
                        async with session.get(
                            ollama_url,
                            timeout=aiohttp.ClientTimeout(total=2)
                        ) as resp:
                            if resp.status == 200:
                                ollama_available = True
                            else:
                                ollama_error = f"HTTP {resp.status}"
                    except asyncio.TimeoutError:
                        ollama_error = "Timeout (2s)"
            else:
                # Fallback: sync urllib (blocking, but better than nothing)
                import urllib.request
                urllib.request.urlopen(ollama_url, timeout=2)
                ollama_available = True
        except Exception as e:
            ollama_error = str(e)

        return NetworkStatus(
            ollama_available=ollama_available,
            ollama_url=ollama_url,
            ollama_error=ollama_error,
        )

    def _calculate_directory_size(self, directory: Path) -> float:
        """Calculate directory size with safety limits (no hanging on large dirs)."""
        if not directory.exists():
            return 0.0

        total_size = 0
        file_count = 0
        MAX_FILES = 100000  # Safety limit

        try:
            for dirpath, dirnames, filenames in os.walk(directory):
                if file_count >= MAX_FILES:
                    logger.warning(f"Directory size calculation hit safety limit ({MAX_FILES} files)")
                    break

                for filename in filenames:
                    filepath = os.path.join(dirpath, filename)
                    try:
                        total_size += os.path.getsize(filepath)
                        file_count += 1
                    except (OSError, FileNotFoundError):
                        pass  # Skip inaccessible files

            return total_size / 1024 / 1024
        except Exception as e:
            logger.warning(f"Error calculating size of {directory}: {e}")
            return 0.0

    def _check_resources(self) -> ResourceStatus:
        """Check CPU, memory, disk usage (non-blocking)."""
        # NOTE: cpu_percent(interval=None) = instant reading, no blocking
        return ResourceStatus(
            cpu_percent=psutil.cpu_percent(interval=None),
            memory_percent=psutil.virtual_memory().percent,
            memory_available_gb=psutil.virtual_memory().available / 1e9,
        )

    async def log_environment(self, snapshot: EnvironmentSnapshot) -> None:
        """Persist environment snapshot to JSONL log with validation."""
        try:
            self.environment_log.parent.mkdir(parents=True, exist_ok=True)

            # Verify write access (fail early with clear error)
            test_file = self.environment_log.parent / ".cynic_write_test"
            try:
                test_file.write_text("")
                test_file.unlink()
            except (OSError, PermissionError) as e:
                logger.error(f"Cannot write to {self.environment_log.parent}: {e}")
                return  # Fail gracefully - don't crash CYNIC

            # Append to JSONL (atomic append-only)
            with open(self.environment_log, "a") as f:
                f.write(json.dumps(asdict(snapshot)) + "\n")
        except (OSError, PermissionError) as e:
            logger.error(f"Failed to log environment: {e}")

    def get_discoveries_log(self) -> Dict[str, Any]:
        """Read discoveries log."""
        if not self.discoveries_log.exists():
            return {"discoveries": [], "remembered": {}}

        try:
            return json.loads(self.discoveries_log.read_text())
        except Exception:
            return {"discoveries": [], "remembered": {}}

    def save_discovery(self, key: str, value: Any) -> None:
        """Log a discovered configuration."""
        log = self.get_discoveries_log()

        # Record discovery
        log["discoveries"].append({
            "timestamp": datetime.now().isoformat(),
            "key": key,
            "value": value,
        })

        # Keep rolling cap of F(11)=89 discoveries
        log["discoveries"] = log["discoveries"][-89:]

        # Save
        self.discoveries_log.parent.mkdir(parents=True, exist_ok=True)
        self.discoveries_log.write_text(json.dumps(log, indent=2))

    def get_known_preference(self, key: str) -> Optional[Any]:
        """Check if user already told us how to handle this."""
        log = self.get_discoveries_log()
        return log.get("remembered", {}).get(key)

    def save_user_preference(self, key: str, value: Any) -> None:
        """Remember user's preference for this setting."""
        log = self.get_discoveries_log()
        if "remembered" not in log:
            log["remembered"] = {}

        log["remembered"][key] = value
        self.discoveries_log.write_text(json.dumps(log, indent=2))


# Convenience functions
_introspector = None


def get_introspector() -> EnvironmentIntrospector:
    """Get singleton introspector."""
    global _introspector
    if _introspector is None:
        _introspector = EnvironmentIntrospector()
    return _introspector
