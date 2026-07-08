# Tier 1
import json
from pathlib import Path
from typing import Dict
import sys

# Add the project root for relative imports
sys.path.append(str(Path(__file__).parent.parent.parent.parent))

from organs.supervisor.src.core.ports import DocsSourcePort

class FilesystemDocsAdapter(DocsSourcePort):
    """An adapter for reading/writing documentation from the local filesystem."""

    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.agent_exposure_path = self.project_root / "infra" / "agent-exposure.json"
        self.agents_md_path = self.project_root / "AGENTS.md"

    def read_exposure_data(self) -> Dict:
        if not self.agent_exposure_path.exists():
            print(f"Error: {self.agent_exposure_path} not found.")
            return {}
        with open(self.agent_exposure_path, "r") as f:
            return json.load(f)

    def read_agents_md(self) -> str:
        if not self.agents_md_path.exists():
            return ""
        return self.agents_md_path.read_text()

    def write_agents_md(self, content: str):
        self.agents_md_path.write_text(content)
