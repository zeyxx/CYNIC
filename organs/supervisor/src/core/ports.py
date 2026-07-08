# Tier 1
from abc import ABC, abstractmethod
from typing import List, Dict

class VersionControlPort(ABC):
    """An interface for interacting with a version control system."""

    @abstractmethod
    def get_staged_files(self) -> List[str]:
        """Returns a list of file paths that are staged."""
        pass

    @abstractmethod
    def get_staged_content(self) -> str:
        """Returns the content diff of staged changes."""
        pass

class DocsSourcePort(ABC):
    """An interface for reading and writing documentation and its sources."""

    @abstractmethod
    def read_exposure_data(self) -> Dict:
        """Reads the agent exposure JSON data."""
        pass
    
    @abstractmethod
    def read_agents_md(self) -> str:
        """Reads the content of AGENTS.md."""
        pass

    @abstractmethod
    def write_agents_md(self, content: str):
        """Writes content to AGENTS.md."""
        pass
