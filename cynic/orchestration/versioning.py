"""
Version Management — SemVer, releases, migrations.

CYNIC manages its own versions and migrations.
"""
from __future__ import annotations

import json
import logging
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)

CYNIC_ROOT = Path(__file__).parent.parent.parent.parent
VERSION_FILE = CYNIC_ROOT / ".version"


@dataclass
class Version:
    """Semantic version."""
    major: int
    minor: int
    patch: int
    prerelease: Optional[str] = None

    def __str__(self) -> str:
        v = f"{self.major}.{self.minor}.{self.patch}"
        if self.prerelease:
            v += f"-{self.prerelease}"
        return v

    @staticmethod
    def parse(version_str: str) -> Version:
        """Parse version string like '1.0.0' or '1.0.0-rc1'."""
        prerelease = None
        if "-" in version_str:
            version_str, prerelease = version_str.split("-", 1)

        parts = version_str.split(".")
        return Version(
            major=int(parts[0]),
            minor=int(parts[1]) if len(parts) > 1 else 0,
            patch=int(parts[2]) if len(parts) > 2 else 0,
            prerelease=prerelease,
        )

    def to_dict(self):
        return asdict(self)


@dataclass
class Release:
    """Release record."""
    version: str
    timestamp: datetime
    notes: str
    docker_image: str
    db_migrations: list[str]  # e.g., ["V001__initial.sql"]
    status: str  # "draft", "published", "stable"

    def to_dict(self):
        return {**asdict(self), "timestamp": self.timestamp.isoformat()}


class VersionManager:
    """Manages CYNIC versioning and releases."""

    def __init__(self, version_file: Path = VERSION_FILE):
        self.version_file = version_file
        self.version_file.mkdir(exist_ok=True)
        self._current_version = self._load_version()

    def _load_version(self) -> Version:
        """Load current version from file or default to 1.0.0."""
        version_path = self.version_file / "VERSION"

        if version_path.exists():
            version_str = version_path.read_text().strip()
            return Version.parse(version_str)

        return Version(1, 0, 0)

    def _save_version(self, version: Version) -> None:
        """Save version to file."""
        version_path = self.version_file / "VERSION"
        version_path.write_text(str(version))
        logger.info(f"Version updated: {version}")

    async def bump_patch(self) -> Version:
        """Bump patch version (1.0.0 → 1.0.1)."""
        self._current_version.patch += 1
        self._save_version(self._current_version)
        return self._current_version

    async def bump_minor(self) -> Version:
        """Bump minor version (1.0.0 → 1.1.0)."""
        self._current_version.minor += 1
        self._current_version.patch = 0
        self._save_version(self._current_version)
        return self._current_version

    async def bump_major(self) -> Version:
        """Bump major version (1.0.0 → 2.0.0)."""
        self._current_version.major += 1
        self._current_version.minor = 0
        self._current_version.patch = 0
        self._save_version(self._current_version)
        return self._current_version

    async def create_release(
        self,
        notes: str,
        docker_image: str,
        db_migrations: Optional[list[str]] = None,
    ) -> Release:
        """
        Create a release record.

        Args:
            notes: Release notes
            docker_image: Docker image tag (e.g., "cynic-kernel:1.0.0")
            db_migrations: List of migration files

        Returns:
            Release record
        """
        db_migrations = db_migrations or []

        release = Release(
            version=str(self._current_version),
            timestamp=datetime.now(),
            notes=notes,
            docker_image=docker_image,
            db_migrations=db_migrations,
            status="published",
        )

        # Save to changelog
        changelog_path = self.version_file / "CHANGELOG.md"
        changelog = changelog_path.read_text() if changelog_path.exists() else ""

        entry = f"""
## [{release.version}] - {release.timestamp.strftime('%Y-%m-%d')}

### Release
- Docker Image: {docker_image}
- Migrations: {', '.join(db_migrations) if db_migrations else 'None'}

### Notes
{notes}

"""

        changelog_path.write_text(entry + changelog)
        logger.info(f"Release {release.version} created")

        return release

    def get_current(self) -> Version:
        """Get current version."""
        return self._current_version

    def get_all_releases(self) -> list[Release]:
        """Get all releases from CHANGELOG."""
        changelog_path = self.version_file / "CHANGELOG.md"

        if not changelog_path.exists():
            return []

        # Parse CHANGELOG (simplified)
        # TODO: Proper markdown parsing
        return []

    async def apply_migration(self, migration_file: str) -> bool:
        """
        Apply a database migration.

        Args:
            migration_file: Migration file name (e.g., "V001__initial.sql")

        Returns:
            Success status
        """
        migrations_dir = self.version_file / "migrations"
        migration_path = migrations_dir / migration_file

        if not migration_path.exists():
            logger.error(f"Migration file not found: {migration_file}")
            return False

        logger.info(f"Applying migration: {migration_file}")
        # TODO: Execute migration via psql or asyncpg
        return True
