"""
CYNIC DogSoul — Cross-Session Identity Memory.

Unified Identity: Now supports SurrealDB as the primary memory,
falling back to local JSON for bootstrapping or local dev.
"""

from __future__ import annotations

import json
import logging
import time
from dataclasses import dataclass, field
from pathlib import Path

from cynic.kernel.core.storage.interface import DogSoulRepoInterface

logger = logging.getLogger("cynic.kernel.core.soul")


@dataclass
class DogSoul:
    """Represents the persistent identity and wisdom of a Sefirotic Dog."""

    dog_id: str
    total_judgments: int = 0
    avg_q_score: float = 50.0
    confidence_factor: float = 0.382
    accumulated_wisdom: str = ""
    last_seen: float = field(default_factory=time.time)
    axiom_weights: dict[str, float] = field(default_factory=dict)

    def to_dict(self) -> dict:
        return {
            "dog_id": self.dog_id,
            "total_judgments": self.total_judgments,
            "avg_q_score": round(self.avg_q_score, 2),
            "confidence_factor": round(self.confidence_factor, 3),
            "last_seen": self.last_seen,
            "axiom_weights": self.axiom_weights,
            "accumulated_wisdom": self.accumulated_wisdom,
        }

    async def save(self, repo: DogSoulRepoInterface | None = None):
        """Persist soul to SurrealDB (primary) or Disk (fallback)."""
        if repo:
            try:
                await repo.save(self.to_dict())
                return
            except Exception as e:
                logger.error(f"Failed to save soul to DB: {e}")

        # Fallback to local JSON
        path = Path.home() / ".cynic" / "dogs" / self.dog_id.lower() / "soul.json"
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            with open(path, "w") as f:
                json.dump(self.to_dict(), f, indent=2)
        except Exception as e:
            logger.error(f"Failed to save soul to disk for {self.dog_id}: {e}")

    @classmethod
    async def load(cls, dog_id: str, repo: DogSoulRepoInterface | None = None) -> DogSoul:
        """Load soul from SurrealDB (primary) or Disk (fallback)."""
        if repo:
            try:
                data = await repo.get(dog_id)
                if data:
                    return cls(**data)
            except Exception as e:
                logger.warning(f"Failed to load soul from DB for {dog_id}: {e}")

        # Fallback to disk
        path = Path.home() / ".cynic" / "dogs" / dog_id.lower() / "soul.json"
        if path.exists():
            try:
                with open(path) as f:
                    data = json.load(f)
                    return cls(**data)
            except Exception as e:
                logger.warning(f"Failed to load soul from disk, using fresh one: {e}")

        return cls(dog_id=dog_id)
