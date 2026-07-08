# Tier 1
import subprocess
from typing import List
from pathlib import Path
import sys

# Add the project root for relative imports
sys.path.append(str(Path(__file__).parent.parent.parent.parent))

from organs.supervisor.src.core.ports import VersionControlPort

class GitAdapter(VersionControlPort):
    """An adapter for interacting with Git."""

    def get_staged_files(self) -> List[str]:
        try:
            result = subprocess.run(
                ["git", "diff", "--cached", "--name-only", "--diff-filter=ACM"],
                capture_output=True, text=True, check=True
            )
            return [f for f in result.stdout.strip().split("\n") if f]
        except (subprocess.CalledProcessError, FileNotFoundError) as e:
            print(f"Error getting staged files from git: {e}")
            return []

    def get_staged_content(self) -> str:
        try:
            result = subprocess.run(
                ["git", "diff", "--cached"],
                capture_output=True, text=True, check=True
            )
            return result.stdout
        except (subprocess.CalledProcessError, FileNotFoundError) as e:
            print(f"Error getting staged content from git: {e}")
            return ""
