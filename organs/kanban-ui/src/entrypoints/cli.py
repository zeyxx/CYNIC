#!/usr/bin/env python3
"""
Entrypoint for organ-kanban-ui.
Syncs Hermes Kanban state to an Obsidian Kanban Markdown file.
"""
import sys
from pathlib import Path

def _bootstrap_repo_path() -> Path:
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            if str(candidate) not in sys.path:
                sys.path.insert(0, str(candidate))
            return candidate
    raise RuntimeError("Could not resolve repository root for kanban ui entrypoint")


PROJECT_ROOT = _bootstrap_repo_path()
SRC_DIR = Path(__file__).resolve().parent
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))

from adapters.kanban_adapters import HermesKanbanSource, ObsidianKanbanProjector

def main():
    obsidian_file = PROJECT_ROOT / "data" / "obsidian" / "kanban.md"
    
    source = HermesKanbanSource()
    projector = ObsidianKanbanProjector(obsidian_file)
    
    print("Fetching tasks from Hermes Kanban...")
    board = source.fetch_tasks()
    
    print(f"Projecting {len(board.tasks)} tasks to Obsidian UI...")
    projector.project(board)
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
