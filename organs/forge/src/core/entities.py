from dataclasses import dataclass, field
from typing import List, Dict, Any

@dataclass
class Branch:
    name: str
    commit_hash: str
    author: str
    date: str

@dataclass
class Alert:
    severity: str
    message: str
    context: Dict[str, Any] = field(default_factory=dict)

@dataclass
class RepoState:
    current_branch: str
    dirty_files: int
    untracked_files: int
    branches: List[Branch] = field(default_factory=list)
    gate_markers: Dict[str, bool] = field(default_factory=dict)
    health_score: float = 100.0
    alerts: List[Alert] = field(default_factory=list)

    def add_alert(self, severity: str, message: str, context: dict = None):
        self.alerts.append(Alert(severity, message, context or {}))
