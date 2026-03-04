"""
CYNIC Discovery Daemon - The Agent of Curiosity.
Scans the organism's own body (codebase) to find areas for improvement.
Focuses on 'Heresies' and 'Complexity'.
"""
from __future__ import annotations
import os
from pathlib import Path
import logging
from typing import List, Dict, Any

logger = logging.getLogger("cynic.organism.discovery")

class DiscoveryDaemon:
    def __init__(self, root_dir: str = "."):
        self.root = Path(root_dir)
        self.ignored_dirs = {".git", ".venv", "__pycache__", "audit", "docs"}

    def find_potential_missions(self) -> List[Dict[str, Any]]:
        """Identifies files that violate CYNIC's Engineering Lenses."""
        missions = []
        
        for path in self.root.rglob("*.py"):
            if any(ignored in path.parts for ignored in self.ignored_dirs):
                continue
                
            with open(path, "r", encoding="utf-8") as f:
                content = f.read()
                
            # 1. Lens: BURN (Simplicity) - Check for giants
            if len(content.splitlines()) > 300:
                missions.append({
                    "axiom": "BURN",
                    "target": str(path),
                    "description": f"File is too large ({len(content.splitlines())} lines). Needs modularization."
                })
                
            # 2. Lens: VERIFY (Type Safety) - Check for flou (Any)
            if "Any" in content and "typing" in content:
                missions.append({
                    "axiom": "VERIFY",
                    "target": str(path),
                    "description": "Detected 'Any' types. Needs strict typing."
                })

            # 3. Lens: BACKEND (Silence) - Check for Heresy (except: pass)
            if "except:" in content and "pass" in content:
                missions.append({
                    "axiom": "BACKEND",
                    "target": str(path),
                    "description": "Detected silent failure (except: pass). Needs ANOMALY reporting."
                })

        # Sort by urgency (BURN first)
        return missions
