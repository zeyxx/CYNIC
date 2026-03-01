"""Experiment log for tracking novel approaches and results."""

from __future__ import annotations

import asyncio
import json
from dataclasses import asdict, dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class Experiment:
    """Immutable experiment record."""

    hypothesis: str
    """The hypothesis being tested"""

    approach: list[str]
    """List of approaches tried"""

    results: dict[str, Any]
    """Results: user_satisfaction, q_score_accuracy, fairness_metric, etc"""

    status: str
    """successful, failed, inconclusive"""

    iterations: int
    """How many times the approach was tried"""

    timestamp: float = field(default_factory=lambda: datetime.now().timestamp())

    @property
    def is_immutable(self) -> bool:
        """Experiments are always immutable."""
        return True


class ExperimentLog:
    """Append-only log of experiments."""

    def __init__(self, store_dir: Path):
        self.store_dir = store_dir
        self.store_dir.mkdir(parents=True, exist_ok=True)
        self.log_path = self.store_dir / "experiment_log.jsonl"

    async def append(self, experiment: Experiment) -> int:
        """Append experiment and return line number."""

        def _append():
            with open(self.log_path, "a") as f:
                exp_dict = asdict(experiment)
                f.write(json.dumps(exp_dict) + "\n")

            # Return approximate ID (line count)
            with open(self.log_path) as f:
                return len(f.readlines())

        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, _append)

    async def get_all(self) -> list[Experiment]:
        """Load all experiments."""

        def _load_all():
            if not self.log_path.exists():
                return []

            experiments = []
            with open(self.log_path) as f:
                for line in f:
                    if line.strip():
                        data = json.loads(line)
                        experiments.append(Experiment(**data))
            return experiments

        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, _load_all)

    async def get_experiments_by_status(self, status: str) -> list[Experiment]:
        """Get experiments with specific status."""
        all_exp = await self.get_all()
        return [e for e in all_exp if e.status == status]

    async def get_recent(self, n: int = 10) -> list[Experiment]:
        """Get last N experiments."""
        all_exp = await self.get_all()
        return all_exp[-n:]

    async def get_by_hypothesis_pattern(self, pattern: str) -> list[Experiment]:
        """Get experiments matching hypothesis pattern."""
        all_exp = await self.get_all()
        return [e for e in all_exp if pattern.lower() in e.hypothesis.lower()]
