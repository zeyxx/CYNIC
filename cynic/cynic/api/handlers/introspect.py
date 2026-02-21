"""
Handler Group Introspection — Architectural self-awareness.

Enables SelfProber and KernelMirror to:
1. Query handler coupling topology
2. Detect coupling growth (alerts if handler adds new dependencies)
3. Analyze architecture health
4. Propose refactoring (move handler to different tier, split domain, etc.)

This is Layer 9 of the organism (Immune System) applied to handler architecture.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.api.handlers.base import HandlerGroup


@dataclass
class HandlerAnalysis:
    """Single handler's architectural signature."""
    name: str
    handler_count: int
    dependencies: frozenset[str]
    events: list[str]
    complexity_score: float  # 0-100: combines handler_count + dependency_count

    @classmethod
    def from_handler(cls, group: HandlerGroup) -> HandlerAnalysis:
        """Extract analysis from a handler group."""
        subs = group.subscriptions()
        deps = group.dependencies()

        # Complexity: penalize both handler count and dependency count
        # Formula: (handler_count * 10) + (dependency_count * 5)
        # Capped at 100 for proportionality
        complexity = min((len(subs) * 10) + (len(deps) * 5), 100.0)

        return cls(
            name=group.name,
            handler_count=len(subs),
            dependencies=deps,
            events=[e.value for e, _ in subs],
            complexity_score=complexity,
        )

    def is_complex(self, threshold: float = 50.0) -> bool:
        """Is this handler complex (above threshold)?"""
        return self.complexity_score > threshold

    def to_dict(self) -> dict:
        """Serialize for storage/API."""
        return {
            "name": self.name,
            "handler_count": self.handler_count,
            "dependency_count": len(self.dependencies),
            "dependencies": sorted(self.dependencies),
            "event_count": len(self.events),
            "events": self.events,
            "complexity_score": round(self.complexity_score, 1),
        }


@dataclass
class ArchitectureSnapshot:
    """Moment-in-time view of handler architecture."""
    timestamp: float
    handler_analyses: list[HandlerAnalysis]
    total_handlers: int
    total_dependencies: set[str]
    most_complex_handler: Optional[HandlerAnalysis]
    average_complexity: float
    complexity_variance: float

    def to_dict(self) -> dict:
        """Serialize for storage."""
        return {
            "timestamp": self.timestamp,
            "handlers": [a.to_dict() for a in self.handler_analyses],
            "total_handlers": self.total_handlers,
            "total_dependencies": len(self.total_dependencies),
            "unique_dependencies": sorted(self.total_dependencies),
            "most_complex": self.most_complex_handler.name if self.most_complex_handler else None,
            "most_complex_score": (
                self.most_complex_handler.complexity_score
                if self.most_complex_handler
                else 0.0
            ),
            "average_complexity": round(self.average_complexity, 1),
            "complexity_variance": round(self.complexity_variance, 2),
        }


@dataclass
class CouplingGrowth:
    """Detected coupling change between two snapshots."""
    handler_name: str
    prev_dependency_count: int
    new_dependency_count: int
    added_dependencies: frozenset[str]
    removed_dependencies: frozenset[str]
    complexity_delta: float

    @property
    def is_growth(self) -> bool:
        """True if coupling increased."""
        return len(self.added_dependencies) > len(self.removed_dependencies)

    @property
    def is_shrinkage(self) -> bool:
        """True if coupling decreased."""
        return len(self.removed_dependencies) > len(self.added_dependencies)

    @property
    def severity_score(self) -> float:
        """0-100: how concerning is this change?

        Formula: (added_deps * 15) - (removed_deps * 5)
        Higher score = more concerning growth.
        """
        return min(
            (len(self.added_dependencies) * 15) - (len(self.removed_dependencies) * 5),
            100.0,
        )

    def to_dict(self) -> dict:
        """Serialize."""
        return {
            "handler_name": self.handler_name,
            "prev_dependency_count": self.prev_dependency_count,
            "new_dependency_count": self.new_dependency_count,
            "added_dependencies": sorted(self.added_dependencies),
            "removed_dependencies": sorted(self.removed_dependencies),
            "complexity_delta": round(self.complexity_delta, 1),
            "severity_score": round(self.severity_score, 1),
        }


class HandlerArchitectureIntrospector:
    """
    Architectural auditor for handlers.

    Methods:
    - snapshot(): current handler topology
    - detect_coupling_growth(): compare two snapshots
    - find_complex_handlers(): identify refactoring candidates
    """

    def snapshot(self, handler_groups: list[HandlerGroup]) -> ArchitectureSnapshot:
        """Create moment-in-time snapshot of handler architecture."""
        import time

        analyses = [HandlerAnalysis.from_handler(group) for group in handler_groups]
        all_deps = set()
        for analysis in analyses:
            all_deps.update(analysis.dependencies)

        total_handlers = sum(a.handler_count for a in analyses)

        # Complexity statistics
        if analyses:
            scores = [a.complexity_score for a in analyses]
            avg_score = sum(scores) / len(scores)
            variance = (
                sum((s - avg_score) ** 2 for s in scores) / len(scores)
                if len(scores) > 1
                else 0.0
            )
            most_complex = max(analyses, key=lambda a: a.complexity_score)
        else:
            avg_score = 0.0
            variance = 0.0
            most_complex = None

        return ArchitectureSnapshot(
            timestamp=time.time(),
            handler_analyses=analyses,
            total_handlers=total_handlers,
            total_dependencies=all_deps,
            most_complex_handler=most_complex,
            average_complexity=avg_score,
            complexity_variance=variance,
        )

    def detect_coupling_growth(
        self,
        prev_snapshot: ArchitectureSnapshot,
        curr_snapshot: ArchitectureSnapshot,
    ) -> list[CouplingGrowth]:
        """Compare two snapshots, return list of coupling changes.

        Sorted by severity_score (most concerning first).
        """
        prev_by_name = {a.name: a for a in prev_snapshot.handler_analyses}
        curr_by_name = {a.name: a for a in curr_snapshot.handler_analyses}

        changes: list[CouplingGrowth] = []

        # Check existing handlers for coupling changes
        for handler_name, curr_analysis in curr_by_name.items():
            if handler_name not in prev_by_name:
                # New handler — not a "growth" in existing handler
                continue

            prev_analysis = prev_by_name[handler_name]
            added = curr_analysis.dependencies - prev_analysis.dependencies
            removed = prev_analysis.dependencies - curr_analysis.dependencies

            if added or removed:
                changes.append(
                    CouplingGrowth(
                        handler_name=handler_name,
                        prev_dependency_count=len(prev_analysis.dependencies),
                        new_dependency_count=len(curr_analysis.dependencies),
                        added_dependencies=frozenset(added),
                        removed_dependencies=frozenset(removed),
                        complexity_delta=(
                            curr_analysis.complexity_score
                            - prev_analysis.complexity_score
                        ),
                    )
                )

        # Sort by severity (most concerning first)
        return sorted(changes, key=lambda c: c.severity_score, reverse=True)

    def find_complex_handlers(
        self,
        snapshot: ArchitectureSnapshot,
        threshold: float = 50.0,
    ) -> list[HandlerAnalysis]:
        """Find handlers exceeding complexity threshold.

        Candidates for refactoring (split into smaller groups, move dependencies).
        """
        return [a for a in snapshot.handler_analyses if a.is_complex(threshold)]

    def health_score(self, snapshot: ArchitectureSnapshot) -> float:
        """0-100: overall architecture health.

        Factors:
        - Average complexity (lower is better)
        - Variance (lower is better — consistent complexity)
        - No outliers (max_complexity should be < 80)

        Formula:
        health = 100 - (avg_complexity * 0.4) - (variance * 0.3) - outlier_penalty
        """
        max_score = (
            max([a.complexity_score for a in snapshot.handler_analyses])
            if snapshot.handler_analyses
            else 0.0
        )
        outlier_penalty = max(0.0, (max_score - 80.0) * 0.5)

        health = (
            100.0
            - (snapshot.average_complexity * 0.4)
            - (snapshot.complexity_variance * 0.3)
            - outlier_penalty
        )
        return max(0.0, min(100.0, health))
