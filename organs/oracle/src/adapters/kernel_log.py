"""Kernel log reader adapter for transparency anchoring."""

from __future__ import annotations

from pathlib import Path

from ..core.anchoring import StateRoot, latest_state_root_from_jsonl


class KernelLogStateRootReader:
    def __init__(self, log_path: Path) -> None:
        self.log_path = log_path

    def latest(self) -> StateRoot:
        if not self.log_path.exists():
            raise FileNotFoundError(f"Log file not found at {self.log_path}")
        with self.log_path.open("r", encoding="utf-8") as stream:
            return latest_state_root_from_jsonl(stream, source=str(self.log_path))

