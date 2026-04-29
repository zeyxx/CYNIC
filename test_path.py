import os
from pathlib import Path
organ_dir_base = os.environ.get("CYNIC_ORGAN_DIR", str(Path.home() / ".cynic" / "organs" / "hermes"))
organ_name = "x"
organ_dir = Path(organ_dir_base) / organ_name
tasks_dir = organ_dir / "agent-tasks"
print(f"organ_dir: {organ_dir}")
print(f"tasks_dir: {tasks_dir}")
print(f"exists: {tasks_dir.exists()}")
print(f"content: {[str(f.name) for f in tasks_dir.glob('*.json')]}")
