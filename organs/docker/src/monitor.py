#!/usr/bin/env python3
# Tier 3
"""
Docker Surface Manager (organ-docker)
Monitors local containers and attempts to heal declared CYNIC infrastructure.
"""

import subprocess
import json
import os
import sys
from pathlib import Path
from datetime import datetime, timezone

ORGANS_DIR = Path(__file__).resolve().parent.parent
STATE_FILE = ORGANS_DIR / "state" / "state.json"
EXPECTED_COMPOSE_DIRS = [
    ORGANS_DIR.parent.parent / "infra/docker/cynic-portal"
]

def run_sudo_docker(*args):
    """Run a docker command with sudo."""
    cmd = ["sudo", "-n", "docker"] + list(args)
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"Docker command failed: {res.stderr}")
        return None
    return res.stdout.strip()

def run_sudo_docker_compose(compose_dir, *args):
    """Run docker compose in a specific directory."""
    cmd = ["sudo", "-n", "docker", "compose", "-f", str(compose_dir / "docker-compose.yml")] + list(args)
    res = subprocess.run(cmd, capture_output=True, text=True)
    return res.returncode == 0

def get_all_containers():
    out = run_sudo_docker("ps", "-a", "--format", "{{json .}}")
    if not out:
        return []
    
    containers = []
    for line in out.splitlines():
        if not line.strip(): continue
        try:
            containers.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    return containers

def heal_infrastructure(containers):
    """Check if expected infrastructure is running, if not attempt restart."""
    running_names = [c.get("Names") for c in containers if "Up" in c.get("Status", "")]
    
    actions_taken = []
    
    for compose_dir in EXPECTED_COMPOSE_DIRS:
        if not compose_dir.exists():
            continue
            
        # Try to guess expected service names from compose file loosely (or just do `docker compose ps`)
        # Simpler: just run `docker compose up -d` if things look dead. 
        # But to be precise, we can check if cynic-portal is up.
        if compose_dir.name == "cynic-portal":
            if not any("cynic-portal" in name for name in running_names):
                actions_taken.append(f"Healing {compose_dir.name}: containers missing or stopped.")
                success = run_sudo_docker_compose(compose_dir, "up", "-d")
                if success:
                    actions_taken.append(f"Successfully restarted {compose_dir.name}.")
                else:
                    actions_taken.append(f"Failed to restart {compose_dir.name}.")
                    
    return actions_taken

def main():
    containers = get_all_containers()
    
    # 1. State snapshot
    state = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "total_containers": len(containers),
        "running": len([c for c in containers if "Up" in c.get("Status", "")]),
        "containers": containers
    }
    
    # 2. Heal
    actions = heal_infrastructure(containers)
    state["last_actions"] = actions
    
    # 3. Save state
    STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(json.dumps(state, indent=2))
    
    dataset_file = STATE_FILE.parent / "dataset.jsonl"
    with open(dataset_file, "a") as f:
        f.write(json.dumps(state) + "\n")
    
    print(f"Docker surface audited. {len(containers)} total containers. Actions: {actions}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
