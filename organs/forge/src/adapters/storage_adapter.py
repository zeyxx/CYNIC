import json
import dataclasses
from pathlib import Path
from ..core.entities import RepoState, Branch, Alert
from ..core.ports import IStateStorage

class JsonStateStorage(IStateStorage):
    def __init__(self, state_file: Path):
        self.state_file = state_file

    def save_state(self, state: RepoState) -> None:
        from datetime import datetime, timezone
        self.state_file.parent.mkdir(parents=True, exist_ok=True)
        # Convert dataclass to dict
        data = dataclasses.asdict(state)
        data['timestamp'] = datetime.now(timezone.utc).isoformat()
        
        # Snapshot state
        with open(self.state_file, 'w') as f:
            json.dump(data, f, indent=2)
            
        # Append to time-series dataset
        dataset_file = self.state_file.parent / "dataset.jsonl"
        with open(dataset_file, 'a') as f:
            f.write(json.dumps(data) + '\n')

    def load_state(self) -> RepoState:
        if not self.state_file.exists():
            return RepoState(current_branch="main", dirty_files=0, untracked_files=0)
            
        with open(self.state_file, 'r') as f:
            data = json.load(f)
            
        branches = [Branch(**b) for b in data.get('branches', [])]
        alerts = [Alert(**a) for a in data.get('alerts', [])]
        
        return RepoState(
            current_branch=data.get('current_branch', 'main'),
            dirty_files=data.get('dirty_files', 0),
            untracked_files=data.get('untracked_files', 0),
            branches=branches,
            gate_markers=data.get('gate_markers', {}),
            health_score=data.get('health_score', 100.0),
            alerts=alerts
        )
