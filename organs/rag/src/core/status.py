"""Non-mutating retrieval-memory status model."""

from __future__ import annotations

from dataclasses import asdict, dataclass


STATE_FILENAMES = (".json", ".jsonl", ".sqlite", ".db", ".parquet", ".csv")


@dataclass(frozen=True)
class RagStatus:
    active_retrieval: bool
    local_state_artifacts: tuple[str, ...]
    blockers: tuple[str, ...]

    @property
    def ok(self) -> bool:
        return not self.active_retrieval and not self.local_state_artifacts

    def to_dict(self) -> dict[str, object]:
        payload = asdict(self)
        payload["schema"] = "cynic.rag.status.v1"
        payload["mutates"] = False
        payload["ok"] = self.ok
        return payload


def build_status(local_files: list[str]) -> RagStatus:
    artifacts = tuple(sorted(path for path in local_files if path.endswith(STATE_FILENAMES)))
    blockers: list[str] = []
    if artifacts:
        blockers.append("local retrieval state artifacts remain inside organs/rag")
    return RagStatus(
        active_retrieval=False,
        local_state_artifacts=artifacts,
        blockers=tuple(blockers),
    )

