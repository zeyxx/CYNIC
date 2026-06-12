import json
import subprocess
from pathlib import Path
from core.entities import KanbanTask, KanbanBoard
from core.ports import IKanbanSource, IKanbanProjector

STATUS_COLUMNS = [
    "triage", "ready", "todo", "running", "blocked", "done", "archived"
]

class HermesKanbanSource(IKanbanSource):
    def fetch_tasks(self) -> KanbanBoard:
        try:
            res = subprocess.run(["hermes", "kanban", "ls", "--json"], capture_output=True, text=True, check=True)
            data = json.loads(res.stdout)
            
            tasks = []
            for t in data:
                tasks.append(KanbanTask(
                    id=t.get("id", "?"),
                    title=t.get("title", "Untitled"),
                    status=t.get("status", "triage"),
                    assignee=t.get("assignee")
                ))
            return KanbanBoard(tasks=tasks)
        except Exception as e:
            print(f"Error fetching tasks from hermes: {e}")
            return KanbanBoard(tasks=[])

class ObsidianKanbanProjector(IKanbanProjector):
    def __init__(self, obsidian_file: Path):
        self.obsidian_file = obsidian_file

    def _format_card(self, task: KanbanTask) -> str:
        checkbox = "[x]" if task.status in ("done", "archived") else "[ ]"
        assignee_str = f" @{task.assignee}" if task.assignee else ""
        return f"- {checkbox} **[{task.id}]** {task.title}{assignee_str}"

    def project(self, board: KanbanBoard) -> None:
        self.obsidian_file.parent.mkdir(parents=True, exist_ok=True)
        
        grouped = {col: [] for col in STATUS_COLUMNS}
        
        for t in board.tasks:
            st = t.status if t.status in STATUS_COLUMNS else "triage"
            grouped[st].append(t)
            
        lines = [
            "---",
            "kanban-plugin: basic",
            "---",
            ""
        ]
        
        for col in STATUS_COLUMNS:
            lines.append(f"## {col.capitalize()}")
            for t in grouped[col]:
                lines.append(self._format_card(t))
            lines.append("")
            
        self.obsidian_file.write_text("\n".join(lines))
        print(f"Projected {len(board.tasks)} tasks to {self.obsidian_file}")
