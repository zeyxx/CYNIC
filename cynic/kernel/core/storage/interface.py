"""
CYNIC Storage Interface â€" One ABC for all storage backends.

Both SurrealDB and PostgreSQL implement this interface.
state.py references StorageInterface, never concrete classes.

Ï-Law: BURN â€" one interface, not two parallel implementations.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any

from cynic.kernel.core.formulas import ACT_LOG_CAP, DECISION_TRACE_CAP

# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# REPOSITORY INTERFACES
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class JudgmentRepoInterface(ABC):
    @abstractmethod
    async def save(self, judgment: dict[str, Any]) -> None: ...

    @abstractmethod
    async def get(self, judgment_id: str) -> dict[str, Any] | None: ...

    @abstractmethod
    async def recent(
        self, reality: str | None = None, limit: int = DECISION_TRACE_CAP
    ) -> list[dict[str, Any]]: ...  # F(10) = 55 (imported from formulas.py)

    @abstractmethod
    async def stats(self) -> dict[str, Any]: ...


class QTableRepoInterface(ABC):
    @abstractmethod
    async def get(self, state_key: str, action: str) -> float: ...

    @abstractmethod
    async def update(self, state_key: str, action: str, q_value: float) -> None: ...

    @abstractmethod
    async def get_all_actions(self, state_key: str) -> dict[str, float]: ...

    @abstractmethod
    async def get_all(self) -> list[dict[str, Any]]: ...


class LearningRepoInterface(ABC):
    @abstractmethod
    async def save(self, event: dict[str, Any]) -> None: ...

    @abstractmethod
    async def recent_for_loop(self, loop_name: str, limit: int = 34) -> list[dict]: ...

    @abstractmethod
    async def loop_stats(self) -> dict[str, int]: ...


class BenchmarkRepoInterface(ABC):
    @abstractmethod
    async def save(self, result: dict[str, Any]) -> None: ...

    @abstractmethod
    async def best_llm_for(self, dog_id: str, task_type: str) -> str | None: ...

    @abstractmethod
    async def get_all(self) -> list[dict[str, Any]]: ...

    @abstractmethod
    async def matrix(self) -> list[dict[str, Any]]: ...


class ResidualRepoInterface(ABC):
    @abstractmethod
    async def append(self, point: dict[str, Any]) -> None: ...

    @abstractmethod
    async def recent(self, limit: int = 21) -> list[dict[str, Any]]: ...


class SDKSessionRepoInterface(ABC):
    @abstractmethod
    async def save(self, telemetry: dict[str, Any]) -> None: ...

    @abstractmethod
    async def recent(self, limit: int = 21) -> list[dict[str, Any]]: ...

    @abstractmethod
    async def stats(self) -> dict[str, Any]: ...


class ScholarRepoInterface(ABC):
    @abstractmethod
    async def append(self, entry: dict[str, Any]) -> None: ...

    @abstractmethod
    async def recent_entries(
        self, limit: int = ACT_LOG_CAP
    ) -> list[dict[str, Any]]: ...  # F(11) = 89 (imported from formulas.py)

    @abstractmethod
    async def search_similar_by_embedding(
        self,
        query_embedding: list[float],
        limit: int = 10,
        min_similarity: float = 0.38,
    ) -> list[dict[str, Any]]: ...

    @abstractmethod
    async def count(self) -> int: ...


class ActionProposalRepoInterface(ABC):
    @abstractmethod
    async def upsert(self, action: dict[str, Any]) -> None: ...

    @abstractmethod
    async def all_pending(self) -> list[dict[str, Any]]: ...

    @abstractmethod
    async def all(self) -> list[dict[str, Any]]: ...

    @abstractmethod
    async def update_status(self, action_id: str, status: str) -> None: ...


class DogSoulRepoInterface(ABC):
    @abstractmethod
    async def save(self, soul: dict[str, Any]) -> None: ...

    @abstractmethod
    async def get(self, dog_id: str) -> dict[str, Any] | None: ...

    @abstractmethod
    async def all(self) -> list[dict[str, Any]]: ...


class SecurityEventRepoInterface(ABC):
    """Repository for security events (SIEM Foundation - PHASE 2)."""

    @abstractmethod
    async def save_event(self, event: dict[str, Any]) -> str:
        """Save event and return event_id."""
        ...

    @abstractmethod
    async def get_event(self, event_id: str) -> dict[str, Any] | None:
        """Get event by ID."""
        ...

    @abstractmethod
    async def list_events(
        self, filters: dict[str, Any] | None = None, limit: int = 1000, offset: int = 0
    ) -> list[dict[str, Any]]:
        """List events with optional filters (type, actor_id, timestamp range, etc.)."""
        ...

    @abstractmethod
    async def correlate(
        self, event: dict[str, Any], window_seconds: int = 300
    ) -> list[dict[str, Any]]:
        """Find related events within time window (for anomaly detection)."""
        ...

    @abstractmethod
    async def detect_anomaly(
        self, event: dict[str, Any], baselines: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        """Detect anomalies in event based on baselines."""
        ...

    @abstractmethod
    async def get_stats(self) -> dict[str, Any]:
        """Get storage statistics (event count, disk usage, etc.)."""
        ...


class AxiomFacetRepoInterface(ABC):
    @abstractmethod
    async def save(self, facet: dict[str, Any]) -> None: ...

    @abstractmethod
    async def get_all(self, axiom: str, reality: str) -> list[dict[str, Any]]: ...


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# STORAGE INTERFACE â€" the ONE abstraction
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


class StorageInterface(ABC):
    """
    Unified storage interface for CYNIC.

    Both SurrealDB and PostgreSQL implement this.
    state.py only references StorageInterface, never concrete classes.
    """

    # â"€â"€ Lifecycle â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€

    @abstractmethod
    async def connect(self) -> None: ...

    @abstractmethod
    async def close(self) -> None: ...

    @abstractmethod
    async def ping(self) -> bool: ...

    # â"€â"€ Repository accessors â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€

    @property
    @abstractmethod
    def judgments(self) -> JudgmentRepoInterface: ...

    @property
    @abstractmethod
    def qtable(self) -> QTableRepoInterface: ...

    @property
    @abstractmethod
    def learning(self) -> LearningRepoInterface: ...

    @property
    @abstractmethod
    def benchmarks(self) -> BenchmarkRepoInterface: ...

    @property
    @abstractmethod
    def residuals(self) -> ResidualRepoInterface: ...

    @property
    @abstractmethod
    def sdk_sessions(self) -> SDKSessionRepoInterface: ...

    @property
    @abstractmethod
    def scholar(self) -> ScholarRepoInterface: ...

    @property
    @abstractmethod
    def action_proposals(self) -> ActionProposalRepoInterface: ...

    @property
    @abstractmethod
    def dog_souls(self) -> DogSoulRepoInterface: ...

    @property
    @abstractmethod
    def axiom_facets(self) -> AxiomFacetRepoInterface: ...

    @property
    @abstractmethod
    def security_events(self) -> SecurityEventRepoInterface: ...
