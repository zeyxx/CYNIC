#!/usr/bin/env python3
"""
Entrypoint for organ-kanban-ui.
Syncs Hermes Kanban state to an Obsidian Kanban Markdown file.
"""
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent.parent
SRC_DIR = Path(__file__).resolve().parent.parent
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
