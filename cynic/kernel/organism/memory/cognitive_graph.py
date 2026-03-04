"""
CYNIC Cognitive Graph (AgentKeeper Doctrine).
Stores persistent semantic facts discovered by agents to ensure cross-model continuity.
"""
from __future__ import annotations
import json
from pathlib import Path
from dataclasses import dataclass, asdict, field
from datetime import datetime
from typing import Dict, List, Optional

@dataclass
class SemanticFact:
    subject: str
    fact: str
    criticality: int = 1 # 1-5, importance for prompt injection
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())

class CognitiveGraph:
    def __init__(self, storage_path: str = "audit/cognitive_graph.json"):
        self.path = Path(storage_path)
        self.facts: List[SemanticFact] = []
        self.load()

    def load(self):
        if self.path.exists():
            try:
                with open(self.path, "r") as f:
                    data = json.load(f)
                    self.facts = [SemanticFact(**item) for item in data]
            except: pass

    def persist(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with open(self.path, "w") as f:
            json.dump([asdict(f) for f in self.facts], f, indent=2)

    def add_fact(self, subject: str, fact: str, criticality: int = 1):
        # Update if subject exists, or add new
        for existing in self.facts:
            if existing.subject == subject:
                existing.fact = fact
                existing.criticality = criticality
                existing.timestamp = datetime.now().isoformat()
                self.persist()
                return
        self.facts.append(SemanticFact(subject, fact, criticality))
        self.persist()

    def get_relevant_context(self, task_description: str) -> str:
        """Retrieves facts relevant to the task for prompt injection."""
        # Simple keyword matching for now (industrial would use vector search)
        relevant = [f.fact for f in self.facts if f.subject.lower() in task_description.lower()]
        if not relevant:
            # Fallback to high-criticality facts
            relevant = [f.fact for f in self.facts if f.criticality >= 4]
            
        if not relevant: return ""
        return "\n--- RELEVANT COGNITIVE CONTEXT ---\n" + "\n".join(relevant) + "\n"
