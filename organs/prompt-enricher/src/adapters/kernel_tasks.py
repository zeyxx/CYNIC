"""Kernel task adapter for prompt enrichment."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RawPromptTask:
    task_id: str
    prompt: str


class KernelTaskAdapter:
    """Non-mutating adapter shell until Kernel enrichment endpoints are explicit."""

    def __init__(self, kernel_url: str) -> None:
        self.kernel_url = kernel_url.rstrip("/")

    def fetch_raw_tasks(self) -> list[RawPromptTask]:
        return []

    def mark_enriched(self, task: RawPromptTask, enriched_prompt: str) -> None:
        _ = (task, enriched_prompt)
