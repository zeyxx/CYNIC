from dataclasses import dataclass
from typing import List, Optional

@dataclass
class KanbanTask:
    id: str
    title: str
    status: str
    assignee: Optional[str] = None

@dataclass
class KanbanBoard:
    tasks: List[KanbanTask]
