from dataclasses import dataclass, field
from datetime import datetime
from typing import List, Dict, Any, Optional

@dataclass
class HumanGate:
    required: bool
    reason: str

@dataclass
class NavigationIntent:
    id: str
    created_at: datetime
    source: str
    objective: str
    url: str
    allowed_actions: List[str]
    forbidden_actions: List[str]
    evidence_required: List[str]
    human_gate: HumanGate
    context: Dict[str, Any] = field(default_factory=dict)

@dataclass
class EvidenceItem:
    kind: str
    value: str
    redaction: Optional[str] = None

@dataclass
class PageEvidence:
    task_id: str
    status: str  # "completed" | "blocked" | "failed" | "needs_human_gate"
    observed_at: datetime
    url: str
    evidence: List[EvidenceItem] = field(default_factory=list)
    mutations_performed: List[str] = field(default_factory=list)
    blockers: List[str] = field(default_factory=list)
    summary: Optional[str] = None

@dataclass
class RedactionRule:
    pattern: str
    replacement: str
    description: str

@dataclass
class ActionPolicy:
    layer: str  # "search" | "extract" | "interact" | "mutate"
    allowed_actions: List[str]
    forbidden_actions: List[str]
    requires_approval: bool

@dataclass
class BrowserBackendCapability:
    name: str
    policy_layer: str
    mutation_risk: str
    requires_human_gate: bool = False

@dataclass
class BrowserToolCall:
    task_id: str
    tool_name: str
    arguments: Dict[str, Any]
    policy_layer: str
    human_gate: Optional[HumanGate] = None

@dataclass
class BrowserToolResult:
    task_id: str
    tool_name: str
    status: str
    observed_at: datetime
    payload: Dict[str, Any] = field(default_factory=dict)
    evidence: List[EvidenceItem] = field(default_factory=list)
    mutations_performed: List[str] = field(default_factory=list)
    blockers: List[str] = field(default_factory=list)
