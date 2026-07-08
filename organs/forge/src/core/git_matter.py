"""Pure Git matter decision drafting for organ-forge.

The supervisor owns read-only perception through cynic.git_matter_report.v1.
This module only converts those batches into reviewable draft decisions.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Protocol

from organs.contracts.src.surface_contracts import ReportSurfaceSpec


GIT_MATTER_REPORT_SCHEMA = "cynic.git_matter_report.v1"
GIT_MATTER_DECISIONS_SCHEMA = "cynic.git_matter_decisions.v1"

GIT_MATTER_DECISIONS_SURFACE = ReportSurfaceSpec(
    schema=GIT_MATTER_DECISIONS_SCHEMA,
    command="organ-forge git-matter-decisions",
    mutates=False,
    mutates_mempool=False,
    usage="organ-forge git-matter-decisions < cynic.git_matter_report.v1.json",
)

VALID_DECISIONS = frozenset({"track", "quarantine", "split", "inspect", "discard"})
VALID_RISKS = frozenset({"low", "medium", "high"})
VALID_OWNERS = frozenset({"organ", "crate", "contract", "infra", "repo"})


@dataclass(frozen=True)
class GitMatterBatch:
    batch_id: str
    family: str
    topological_class: str
    status: str
    path_count: int
    sample_paths: tuple[str, ...]

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "GitMatterBatch":
        batch_id = str(data.get("batch_id", "")).strip()
        family = str(data.get("family", "")).strip()
        topological_class = str(data.get("topological_class", "")).strip()
        status = str(data.get("status", "")).strip()
        sample_paths = tuple(str(path) for path in data.get("sample_paths", []))
        path_count = int(data.get("path_count", 0))

        missing = [
            name
            for name, value in (
                ("batch_id", batch_id),
                ("family", family),
                ("topological_class", topological_class),
                ("status", status),
            )
            if not value
        ]
        if missing:
            raise RuntimeError(f"git matter batch missing fields: {','.join(missing)}")
        if path_count < 0:
            raise RuntimeError("git matter batch path_count must be non-negative")
        return cls(
            batch_id=batch_id,
            family=family,
            topological_class=topological_class,
            status=status,
            path_count=path_count,
            sample_paths=sample_paths,
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "batch_id": self.batch_id,
            "family": self.family,
            "topological_class": self.topological_class,
            "status": self.status,
            "path_count": self.path_count,
            "sample_paths": list(self.sample_paths),
        }


@dataclass(frozen=True)
class GitMatterReview:
    decision: str
    risk: str
    owner: str
    rationale: str
    next_probe: str

    def __post_init__(self) -> None:
        if self.decision not in VALID_DECISIONS:
            raise RuntimeError(f"unsupported git matter decision: {self.decision}")
        if self.risk not in VALID_RISKS:
            raise RuntimeError(f"unsupported git matter risk: {self.risk}")
        if self.owner not in VALID_OWNERS:
            raise RuntimeError(f"unsupported git matter owner: {self.owner}")
        if not self.rationale.strip():
            raise RuntimeError("git matter review rationale missing")
        if not self.next_probe.strip():
            raise RuntimeError("git matter review next_probe missing")

    def to_dict(self) -> dict[str, str]:
        return {
            "decision": self.decision,
            "risk": self.risk,
            "owner": self.owner,
            "rationale": self.rationale,
            "next_probe": self.next_probe,
        }


@dataclass(frozen=True)
class GitMatterDecision:
    batch: GitMatterBatch
    review: GitMatterReview

    def to_dict(self) -> dict[str, Any]:
        payload = self.batch.to_dict()
        payload.update(self.review.to_dict())
        return payload


class InferenceReviewPort(Protocol):
    def review_git_matter_batch(self, batch: GitMatterBatch) -> GitMatterReview:
        """Return a bounded review for one supervisor-produced batch."""


class DeterministicGitMatterReviewer:
    """Conservative reviewer used before sovereign inference is wired in."""

    def review_git_matter_batch(self, batch: GitMatterBatch) -> GitMatterReview:
        if batch.topological_class in {"runtime_source", "kernel_source"}:
            return GitMatterReview(
                decision="inspect",
                risk="high",
                owner="crate",
                rationale="runtime or kernel source requires explicit review before closure",
                next_probe="inspect patch and lifecycle blockers for this batch",
            )
        if batch.topological_class == "organ_body":
            return GitMatterReview(
                decision="inspect",
                risk="medium",
                owner="organ",
                rationale="organ body changes need owner review before track or quarantine",
                next_probe="inspect organ manifest boundary and changed paths",
            )
        if batch.topological_class == "contract":
            return GitMatterReview(
                decision="inspect",
                risk="medium",
                owner="contract",
                rationale="contract changes can alter authority or validation rules",
                next_probe="run topology and contract validators for this batch",
            )
        if batch.status == "untracked":
            return GitMatterReview(
                decision="quarantine",
                risk="low",
                owner="repo",
                rationale="unknown untracked repo matter defaults to quarantine review",
                next_probe="classify source authority before promotion",
            )
        return GitMatterReview(
            decision="inspect",
            risk="low",
            owner="repo",
            rationale="visible Git matter has no specialized forge rule yet",
            next_probe="inspect batch metadata before planning closure",
        )


def git_matter_batches_from_report(report: dict[str, Any]) -> list[GitMatterBatch]:
    if report.get("schema") != GIT_MATTER_REPORT_SCHEMA:
        raise RuntimeError("expected cynic.git_matter_report.v1")
    batches = report.get("inference_batches")
    if not isinstance(batches, list):
        raise RuntimeError("git matter report inference_batches must be a list")
    return [GitMatterBatch.from_dict(batch) for batch in batches]


def draft_git_matter_decisions(
    report: dict[str, Any],
    reviewer: InferenceReviewPort | None = None,
) -> dict[str, Any]:
    reviewer = reviewer or DeterministicGitMatterReviewer()
    batches = git_matter_batches_from_report(report)
    decisions = [
        GitMatterDecision(
            batch=batch,
            review=reviewer.review_git_matter_batch(batch),
        ).to_dict()
        for batch in batches
    ]
    return {
        "schema": GIT_MATTER_DECISIONS_SCHEMA,
        "source_schema": GIT_MATTER_REPORT_SCHEMA,
        "status": "draft",
        "authority": "organ-forge",
        "mutates_git": False,
        "mutates_mempool": False,
        "visible_path_total": report.get("visible_path_total", 0),
        "decision_count": len(decisions),
        "decisions": decisions,
        "blockers": list(report.get("blockers", [])),
        "surface_contract": GIT_MATTER_DECISIONS_SURFACE.as_manifest(),
    }
