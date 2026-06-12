import json
from pathlib import Path
from src.core.ports import StatePort
from src.core.entities import DockerState

class FileStateAdapter(StatePort):
    def __init__(self, state_file_path: Path):
        self.state_file_path = state_file_path
        self.dataset_file_path = state_file_path.parent / "dataset.jsonl"

    def save_state(self, state: DockerState) -> None:
        self.state_file_path.parent.mkdir(parents=True, exist_ok=True)
        
        state_dict = {
            "timestamp": state.timestamp,
            "total_containers": state.total_containers,
            "running": state.running,
            "containers": state.containers,
            "last_actions": state.last_actions
        }
        
        self.state_file_path.write_text(json.dumps(state_dict, indent=2))
        
        with open(self.dataset_file_path, "a") as f:
            f.write(json.dumps(state_dict) + "\n")
