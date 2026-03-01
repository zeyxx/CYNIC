"""
CYNIC DogSoul — Cross-Session Identity Memory (β2)

Each Dog maintains a SOUL.md file at:
    ~/.cynic/dogs/{dog_id}/soul.md

The file uses YAML front matter for structured stats and Markdown body
for accumulated narrative wisdom. This allows the Dog to "remember" who
it is across sessions — total judgments, average Q-Score, session count,
strengths, and watchouts.
"""
from __future__ import annotations

import json
import logging
import time
from dataclasses import dataclass, field
from pathlib import Path

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
    
    # Heuristic weights (learned from feedback)
    axiom_weights: dict[str, float] = field(default_factory=dict)
    
    def to_dict(self) -> dict:
        return {
            "dog_id": self.dog_id,
            "total_judgments": self.total_judgments,
            "avg_q_score": round(self.avg_q_score, 2),
            "confidence_factor": round(self.confidence_factor, 3),
            "last_seen": self.last_seen,
            "axiom_weights": self.axiom_weights
        }

    def save(self):
        """Persist soul to disk."""
        path = Path.home() / ".cynic" / "dogs" / self.dog_id.lower() / "soul.json"
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            with open(path, "w") as f:
                json.dump(self.to_dict(), f, indent=2)
        except Exception as e:
            logger.error(f"Failed to save soul for {self.dog_id}: {e}")

    @classmethod
    def load(cls, dog_id: str) -> DogSoul:
        """Load soul from disk or return a fresh one."""
        path = Path.home() / ".cynic" / "dogs" / dog_id.lower() / "soul.json"
        if path.exists():
            try:
                with open(path) as f:
                    data = json.load(f)
                    return cls(**data)
            except Exception as e:
                logger.warning(f"Failed to load soul for {dog_id}, using fresh one: {e}")
        return cls(dog_id=dog_id)
