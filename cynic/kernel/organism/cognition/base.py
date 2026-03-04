"""
CYNIC Cognitive Base - The Knowledge Engine of the Organism.
Loads architectural principles from industry-standard sources (Wisdom Nodes).
"""
from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Dict, List, Optional
from dataclasses import dataclass

logger = logging.getLogger("cynic.organism.cognition")

@dataclass
class WisdomNode:
    id: str
    source: str
    axiom: str
    principle: str
    description: str

class CognitiveBase:
    """
    Registry of high-level architectural constraints and best practices.
    """
    def __init__(self, storage_path: str = "audit/cognitive_wisdom.json"):
        self.path = Path(storage_path)
        self.nodes: List[WisdomNode] = []
        self.load()

    def load(self):
        """Loads wisdom nodes from JSON."""
        if self.path.exists():
            try:
                with open(self.path, "r") as f:
                    data = json.load(f)
                    self.nodes = [WisdomNode(**node) for node in data.get("wisdom_nodes", [])]
                logger.info(f"CognitiveBase: Loaded {len(self.nodes)} wisdom nodes.")
            except Exception as e:
                logger.error(f"Failed to load CognitiveBase: {e}")

    def get_principles_for_axiom(self, axiom: str) -> List[WisdomNode]:
        """Returns all principles relevant to a specific CYNIC Axiom."""
        return [node for node in self.nodes if node.axiom == axiom]

    def get_all_principles(self) -> List[WisdomNode]:
        return self.nodes

# Global instance for the kernel
_base: Optional[CognitiveBase] = None

def get_cognitive_base() -> CognitiveBase:
    global _base
    if _base is None:
        _base = CognitiveBase()
    return _base
