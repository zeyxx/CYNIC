"""Repo-health primitives for the forge/anvil organ entrypoint."""

from __future__ import annotations

from dataclasses import dataclass
from time import time
from typing import Any


@dataclass(frozen=True)
class DirtyCounts:
    total: int
    modified: int
    untracked: int
    deleted: int
    added: int
    renamed: int


@dataclass(frozen=True)
class BranchCounts:
    local: int
    remote: int


@dataclass(frozen=True)
class RepoAnvilSnapshot:
    repo: str
    branch: str
    head: str
    dirty: DirtyCounts
    branches: BranchCounts
    gates: dict[str, int]
    timestamp: str


class RepoAnvilService:
    def health_score(self, snapshot: RepoAnvilSnapshot, *, now: float | None = None) -> float:
        current_time = time() if now is None else now
        dirty_penalty = min(snapshot.dirty.total / 1000.0, 0.4)
        branch_penalty = min(max(snapshot.branches.local - 5, 0) / 10.0, 0.3)
        stale_threshold = 30 * 86400
        gate_penalty = (
            0.1
            if any(value > 0 and (current_time - value) > stale_threshold for value in snapshot.gates.values())
            else 0.0
        )
        return max(1.0 - dirty_penalty - branch_penalty - gate_penalty, 0.0)

    def perception_payload(self, snapshot: RepoAnvilSnapshot, *, now: float | None = None) -> dict[str, Any]:
        return {
            "timestamp": snapshot.timestamp,
            "repo": snapshot.repo,
            "branch": snapshot.branch,
            "head": snapshot.head,
            "dirty": {
                "total": snapshot.dirty.total,
                "modified": snapshot.dirty.modified,
                "untracked": snapshot.dirty.untracked,
                "deleted": snapshot.dirty.deleted,
                "added": snapshot.dirty.added,
                "renamed": snapshot.dirty.renamed,
            },
            "branches": {
                "local": snapshot.branches.local,
                "remote": snapshot.branches.remote,
            },
            "gates": snapshot.gates,
            "health_score": round(self.health_score(snapshot, now=now), 4),
        }

    def audit_entry(
        self,
        *,
        timestamp: str,
        action: str,
        payload: dict[str, Any],
        result: str,
    ) -> dict[str, Any]:
        return {
            "timestamp": timestamp,
            "action": action,
            "payload": payload,
            "result": result,
        }
