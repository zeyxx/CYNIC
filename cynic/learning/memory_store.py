# cynic/learning/memory_store.py
"""Storage for relationship memory persistence."""

from __future__ import annotations

import json
import asyncio
from pathlib import Path
from typing import Optional

from cynic.learning.relationship_memory import RelationshipMemory


class MemoryStore:
    """Persistent storage for relationship memory."""

    def __init__(self, store_dir: Path):
        self.store_dir = store_dir
        self.store_dir.mkdir(parents=True, exist_ok=True)
        self.memory_path = self.store_dir / "relationship_memory.json"

    async def initialize(self) -> None:
        """Initialize memory storage."""
        def _init():
            if not self.memory_path.exists():
                # Create default memory
                default = {
                    "user_values": {
                        "PHI": 0.5,
                        "BURN": 0.5,
                        "FIDELITY": 0.5,
                        "VERIFY": 0.5,
                        "CULTURE": 0.5
                    },
                    "user_preferences": {},
                    "user_style": "balanced",
                    "communication_style": {
                        "verbosity": "balanced",
                        "formality": "casual"
                    },
                    "learning_rate": 0.01,
                    "knowledge_areas": []
                }
                self.memory_path.write_text(json.dumps(default, indent=2))

        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, _init)

    async def save_memory(self, memory: RelationshipMemory) -> None:
        """Save relationship memory to disk."""
        def _save():
            data = {
                "user_values": memory.user_values,
                "user_preferences": memory.user_preferences,
                "user_style": memory.user_style,
                "communication_style": memory.communication_style,
                "learning_rate": memory.learning_rate,
                "knowledge_areas": memory.knowledge_areas
            }
            self.memory_path.write_text(json.dumps(data, indent=2))

        loop = asyncio.get_event_loop()
        await loop.run_in_executor(None, _save)

    async def load_memory(self) -> RelationshipMemory:
        """Load relationship memory from disk."""
        def _load():
            if not self.memory_path.exists():
                # Return default if missing
                return RelationshipMemory(
                    user_values={
                        "PHI": 0.5, "BURN": 0.5, "FIDELITY": 0.5,
                        "VERIFY": 0.5, "CULTURE": 0.5
                    },
                    user_preferences={},
                    user_style="balanced",
                    communication_style={"verbosity": "balanced"},
                    learning_rate=0.01
                )

            data = json.loads(self.memory_path.read_text())
            return RelationshipMemory(
                user_values=data.get("user_values", {}),
                user_preferences=data.get("user_preferences", {}),
                user_style=data.get("user_style", "balanced"),
                communication_style=data.get("communication_style", {}),
                learning_rate=data.get("learning_rate", 0.01),
                knowledge_areas=data.get("knowledge_areas", [])
            )

        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, _load)


# Global memory store instance
_memory_store: Optional[MemoryStore] = None


async def get_memory_store() -> MemoryStore:
    """Get or create global memory store."""
    global _memory_store
    if _memory_store is None:
        store_path = Path.home() / ".cynic" / "phase2"
        _memory_store = MemoryStore(store_path)
        await _memory_store.initialize()
    return _memory_store
