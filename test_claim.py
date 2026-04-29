import json
import os
from pathlib import Path
from datetime import datetime

tasks_dir = Path("/home/user/.cynic/organs/hermes/x/agent-tasks")
for task_file in tasks_dir.glob("*.json"):
    lock_file = tasks_dir / f".{task_file.stem}.lock"
    print(f"Task: {task_file.name}")
    print(f"Lock exists: {lock_file.exists()}")
    if lock_file.exists():
        print(f"Lock content: {lock_file.read_text()}")
