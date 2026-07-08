"""GitHub Release/Commit Sensor Adapter.

Polls the GitHub REST API (no auth required for public repos) to detect
new releases and commits for configured repositories.

Used by:
  - organ-radar CLI for tech watch scanning
  - Kernel lifecycle for autonomous update orchestration
"""

import os
import re
import time
from datetime import datetime, timezone

import httpx
from src.domain.entities import (
    Target,
    TargetType,
    Observation,
    ObservationStatus,
    ReleaseInfo,
)
from src.ports.sensor import SensorPort


class GitHubSensorAdapter(SensorPort):
    """Sensor that perceives GitHub repositories for new releases/commits."""

    BASE_URL = "https://api.github.com"

    def __init__(self, timeout: float = 10.0, token: str = ""):
        self.timeout = timeout
        self.token = token or os.environ.get("GITHUB_TOKEN", "")
        self._headers = {"Accept": "application/vnd.github.v3+json"}
        if self.token:
            self._headers["Authorization"] = f"token {self.token}"

    def _parse_owner_repo(self, uri: str) -> tuple[str, str]:
        """Extract owner/repo from a GitHub URL or 'owner/repo' string."""
        # https://github.com/owner/repo.git -> owner/repo
        m = re.search(r"github\.com[/]([^/]+/[^/]+)", uri)
        if m:
            parts = m.group(1).rstrip(".git").split("/")
            return (parts[0], parts[1])
        # Maybe it's just "owner/repo"
        parts = uri.rstrip("/").rstrip(".git").split("/")
        if len(parts) == 2:
            return (parts[0], parts[1])
        raise ValueError(f"Cannot parse GitHub owner/repo from: {uri}")

    def _extract_version(self, tag: str) -> str:
        """Extract a clean version string from a tag (strip leading 'v')."""
        return tag.lstrip("v")

    async def _fetch_releases(self, owner: str, repo: str, limit: int = 5) -> list[dict]:
        """Fetch the latest releases from GitHub API."""
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            resp = await client.get(
                f"{self.BASE_URL}/repos/{owner}/{repo}/releases",
                headers=self._headers,
                params={"per_page": limit},
            )
            resp.raise_for_status()
            return resp.json()

    async def _fetch_branch_commit(self, owner: str, repo: str, branch: str) -> dict:
        """Fetch the latest commit SHA for a branch."""
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            resp = await client.get(
                f"{self.BASE_URL}/repos/{owner}/{repo}/git/ref/heads/{branch}",
                headers=self._headers,
            )
            resp.raise_for_status()
            return resp.json()

    async def perceive(self, target: Target) -> Observation:
        """Perceive a GitHub target — check for new releases and commits."""
        start = time.perf_counter()

        if target.target_type != TargetType.GITHUB_REPO:
            return Observation(
                target_id=target.id,
                status=ObservationStatus.UNKNOWN,
                raw_response={"error": "Not a GitHub repo target"},
            )

        owner, repo = self._parse_owner_repo(target.uri)
        current_version = target.metadata.get("current_version")
        branch = target.metadata.get("branch", "main")

        try:
            releases = await self._fetch_releases(owner, repo)

            # Filter out prereleases and drafts unless explicitly included
            stable = [r for r in releases if not r.get("prerelease") and not r.get("draft")]
            if not stable:
                stable = releases  # Fallback if all are prereleases

            releases_info = []
            for r in stable[:3]:
                releases_info.append(ReleaseInfo(
                    version=self._extract_version(r["tag_name"]),
                    tag=r["tag_name"],
                    published_at=r["published_at"],
                    url=r["url"],
                    prerelease=r.get("prerelease", False),
                    draft=r.get("draft", False),
                    commit_sha=r.get("target_commitish", ""),
                    html_url=r.get("html_url", ""),
                ))

            latest = releases_info[0] if releases_info else None
            latency_ms = (time.perf_counter() - start) * 1000

            # Determine status
            if current_version and latest:
                if current_version != latest.version:
                    status = ObservationStatus.UPDATE_AVAILABLE
                else:
                    status = ObservationStatus.HEALTHY
            else:
                status = ObservationStatus.HEALTHY  # No baseline, just report

            return Observation(
                target_id=target.id,
                status=status,
                timestamp=datetime.now(timezone.utc),
                latency_ms=latency_ms,
                raw_response={"releases": releases[:3]},
                current_version=current_version,
                latest_version=latest.version if latest else None,
                release_info=latest,
                pending_releases=releases_info[1:],
            )

        except httpx.HTTPStatusError as e:
            latency_ms = (time.perf_counter() - start) * 1000
            return Observation(
                target_id=target.id,
                status=ObservationStatus.DOWN,
                timestamp=datetime.now(timezone.utc),
                latency_ms=latency_ms,
                raw_response={"error": str(e), "status_code": e.response.status_code},
            )
        except Exception as e:
            latency_ms = (time.perf_counter() - start) * 1000
            return Observation(
                target_id=target.id,
                status=ObservationStatus.UNKNOWN,
                timestamp=datetime.now(timezone.utc),
                latency_ms=latency_ms,
                raw_response={"error": str(e)},
            )
