#!/usr/bin/env python3
"""
Entrypoint for organ-forge.
Implements the hexagonal architecture.
"""
import sys
from pathlib import Path

# Add CYNIC root to path for local imports if needed
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from organs.forge.src.adapters.git_adapter import LocalGitPerceiver
from organs.forge.src.adapters.storage_adapter import JsonStateStorage
from organs.forge.src.core.services import HealthAuditor

def main():
    repo_path = PROJECT_ROOT
    state_file = PROJECT_ROOT / "organs" / "forge" / "state" / "state.json"
    
    # 1. Initialize Adapters
    perceiver = LocalGitPerceiver(repo_path)
    storage = JsonStateStorage(state_file)
    
    # 2. Initialize Domain Services
    auditor = HealthAuditor()
    
    print(f"Perceiving repo state at {repo_path}...")
    # 3. Execution flow
    state = perceiver.gather_state()
    
    print("Auditing state...")
    audited_state = auditor.audit(state)
    
    print(f"Saving audited state to {state_file}...")
    storage.save_state(audited_state)
    
    print(f"Done. Health Score: {audited_state.health_score}/100")
    for alert in audited_state.alerts:
        print(f"[{alert.severity}] {alert.message}")
        
    return 0

if __name__ == "__main__":
    sys.exit(main())
