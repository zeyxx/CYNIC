from dataclasses import dataclass
from typing import Dict, Any, List, Optional
from datetime import datetime

@dataclass
class LogEntry:
    agent_id: str
    session_id: str
    timestamp: datetime
    step_type: str
    content: str
    tool_calls: List[Dict[str, Any]]
    is_error: bool

@dataclass
class Anomaly:
    level: str # 'heuristic', 'ml', 'llm'
    description: str
    occurrences: int
    raw_entries: List[LogEntry]
    confidence_score: float
