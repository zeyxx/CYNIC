#!/usr/bin/env python3
# Tier 3
"""
Docker Surface Manager (organ-docker)
Monitors local containers and attempts to heal declared CYNIC infrastructure.
"""

import sys
from pathlib import Path
from datetime import datetime, timezone

from src.core.entities import DockerState
from src.adapters.docker_adapter import SystemDockerAdapter
from src.adapters.state_adapter import FileStateAdapter

ORGANS_DIR = Path(__file__).resolve().parent.parent
STATE_FILE = ORGANS_DIR / "state" / "state.json"

def main():
    docker_adapter = SystemDockerAdapter()
    state_adapter = FileStateAdapter(STATE_FILE)

    containers = docker_adapter.get_all_containers()
    
    # 1. State snapshot
    timestamp = datetime.now(timezone.utc).isoformat()
    total_containers = len(containers)
    running_containers = sum(1 for c in containers if c.is_up)
    
    # 2. Heal
    actions = []
    running_names = [c.name for c in containers if c.is_up]
    
    if not any("cynic-portal" in name for name in running_names):
        actions.append("Healing cynic-portal: containers missing or stopped.")
        success = docker_adapter.heal_service("cynic-portal")
        if success:
            actions.append("Successfully restarted cynic-portal.")
        else:
            actions.append("Failed to restart cynic-portal.")
            
    state = DockerState(
        timestamp=timestamp,
        total_containers=total_containers,
        running=running_containers,
        containers=[c.raw_data for c in containers],
        last_actions=actions
    )
    
    # 3. Save state
    state_adapter.save_state(state)
    
    print(f"Docker surface audited. {len(containers)} total containers. Actions: {actions}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
