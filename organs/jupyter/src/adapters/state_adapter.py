import json
import dataclasses
from pathlib import Path
from ..core.ports import StatePort
from ..core.entities import JupyterState

class FileStateAdapter(StatePort):
    def __init__(self, state_file: Path):
        self.state_file = state_file

    def save_state(self, state: JupyterState) -> None:
        self.state_file.parent.mkdir(parents=True, exist_ok=True)
        with open(self.state_file, "w") as f:
            json.dump(dataclasses.asdict(state), f, indent=2)
