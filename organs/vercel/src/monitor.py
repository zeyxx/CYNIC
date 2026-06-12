#!/usr/bin/env python3
# Tier 3
"""
Vercel Edge Manager (organ-vercel)
Monitors CYNIC frontend deployments on Vercel via REST API.
"""

import sys
from pathlib import Path
from datetime import datetime, timezone

from src.core.entities import VercelState
from src.adapters.vercel_adapter import RestVercelAdapter
from src.adapters.state_adapter import FileStateAdapter

ORGANS_DIR = Path(__file__).resolve().parent.parent
STATE_FILE = ORGANS_DIR / "state" / "state.json"

def main():
    vercel_adapter = RestVercelAdapter()
    state_adapter = FileStateAdapter(STATE_FILE)

    deployments = vercel_adapter.fetch_deployments()
    
    timestamp = datetime.now(timezone.utc).isoformat()
    total_fetched = len(deployments)
    failing = sum(1 for d in deployments if d.state == "ERROR")
    ready = sum(1 for d in deployments if d.state == "READY")
    
    state = VercelState(
        timestamp=timestamp,
        total_fetched=total_fetched,
        failing=failing,
        ready=ready,
        deployments=[{
            "id": d.id,
            "name": d.name,
            "url": d.url,
            "state": d.state,
            "created": d.created
        } for d in deployments]
    )
    
    state_adapter.save_state(state)
    
    print(f"Vercel surface audited. {total_fetched} deployments found. {failing} failing.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
