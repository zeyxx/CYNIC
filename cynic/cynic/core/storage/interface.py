"""
CYNIC Storage Interface — One ABC for all storage backends.

Both SurrealDB and PostgreSQL implement this interface.
state.py references StorageInterface, never concrete classes.

φ-Law: BURN — one interface, not two parallel implementations.
"""
from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


# ════════════════════════════════════════════════════════════════════════════
# REPOSITORY INTERFACES
# ════════════════════════════════════════════════════════════════════════════

class JudgmentRepoInterface(ABC):
    @abstractmethod
    async def save(self, judgment: dict[str, Any]) -> None: ...

    @abstractmethod
    async def get(self, judgment_id: str) -> dict[str, Any] | None: ...

    @abstractmethod
    async def recent(
        self, reality: str | None = None, limit: int = 55
    ) -> list[dict[str, Any]]: ...

    @abstractmethod
    async def stats(self) -> dict[str, Any]: ...


class QTableRepoInterface(ABC):
    @abstractmethod
    async def get(self, state_key: str, action: str) -> float: ...

    @abstractmethod
    async def update(
        self, state_key: str, action: str, q_value: float
    ) -> None: ...

    @abstractmethod
    async def get_all_actions(self, state_key: str) -> dict[str, float]: ...

    @abstractmethod
    async def get_all(self) -> list[dict[str, Any]]: ...


class LearningRepoInterface(ABC):
    @abstractmethod
    async def save(self, event: dict[str, Any]) -> None: ...

    @abstractmethod
    async def recent_for_loop(
        self, loop_name: str, limit: int = 34
    ) -> list[dict]: ...

    @abstractmethod
    async def loop_stats(self) -> dict[str, int]: ...


class BenchmarkRepoInterface(ABC):
    @abstractmethod
    async def save(self, result: dict[str, Any]) -> None: ...

    @abstractmethod
    async def best_llm_for(
        self, dog_id: str, task_type: str
    ) -> str | None: ...

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
        self, limit: int = 89
    ) -> list[dict[str, Any]]: ...

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
    async def update_status(
        self, action_id: str, status: str
    ) -> None: ...


class DogSoulRepoInterface(ABC):
    @abstractmethod
    async def save(self, soul: dict[str, Any]) -> None: ...

    @abstractmethod
    async def get(self, dog_id: str) -> dict[str, Any] | None: ...

    @abstractmethod
    async def all(self) -> list[dict[str, Any]]: ...


# ════════════════════════════════════════════════════════════════════════════
# STORAGE INTERFACE — the ONE abstraction
# ════════════════════════════════════════════════════════════════════════════

class StorageInterface(ABC):
    """
    Unified storage interface for CYNIC.

    Both SurrealDB and PostgreSQL implement this.
    state.py only references StorageInterface, never concrete classes.
    """

    # ── Lifecycle ──────────────────────────────────────────────────────────

    @abstractmethod
    async def connect(self) -> None: ...

    @abstractmethod
    async def close(self) -> None: ...

    @abstractmethod
    async def ping(self) -> bool: ...

    # ── Repository accessors ──────────────────────────────────────────────

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
