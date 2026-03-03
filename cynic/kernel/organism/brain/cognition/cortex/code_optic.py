"""
CYNIC Codebase Reader - The recursive optic.
Allows the organism to ingest its own source code for meta-cognition.
"""

import os
import logging
from pathlib import Path
from typing import Dict

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.code_optic")


class CodebaseOptic:
    """
    Indexes the cynic/ directory to provide a structured context for the Brain.
    Lentille: Solutions Architect / AI Infra
    """

    def __init__(self, root_dir: str = "."):
        self.root = Path(root_dir)
        self.ignored_dirs = {
            ".git",
            "__pycache__",
            ".pytest_cache",
            ".worktrees",
            "venv",
            ".co-scientist",
        }

    def scan_codebase(self) -> Dict[str, str]:
        """
        Reads all .py files and returns a map of {filepath: content}.
        Used to feed Gemini 3's large context.
        """
        code_map = {}
        logger.info(f"Scanning codebase from root: {self.root.absolute()}")
        for path in self.root.rglob("*.py"):
            if any(part in self.ignored_dirs for part in path.parts):
                continue

            rel_path = str(path.relative_to(self.root))
            if os.name == "nt":
                rel_path = rel_path.replace("\\", "/")

            try:
                # ASCII conversion for stability
                with open(path, "rb") as f:
                    content = f.read().decode("ascii", "ignore")
                code_map[rel_path] = content
            except Exception as e:
                logger.error(f"Failed to read {path}: {e}")

        logger.info(f"Scan complete. Found {len(code_map)} files.")
        return code_map

    def get_file_summary(self) -> str:
        """Returns a tree-like string of the codebase structure."""
        summary = ["# CYNIC Codebase Topology"]
        for path in sorted(self.root.rglob("*.py")):
            if any(part in self.ignored_dirs for part in path.parts):
                continue
            summary.append(f"- {path.relative_to(self.root)}")
        return "\n".join(summary)
