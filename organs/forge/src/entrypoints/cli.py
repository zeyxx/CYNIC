#!/usr/bin/env python3
"""
Entrypoint for organ-forge.
Implements the hexagonal architecture.
"""
import sys
import argparse
import asyncio
from pathlib import Path

# Add CYNIC root to path for local imports if needed
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from organs.forge.src.adapters.git_adapter import LocalGitPerceiver
from organs.forge.src.adapters.storage_adapter import JsonStateStorage
from organs.forge.src.adapters.kernel_adapter import KernelWebSocketClient
from organs.forge.src.core.services import HealthAuditor, BuildDelegationService

def run_audit(repo_path, state_file):
    # 1. Initialize Adapters
    perceiver = LocalGitPerceiver(repo_path)
    storage = JsonStateStorage(state_file)
    
    # 2. Initialize Domain Services
    auditor = HealthAuditor()
    
    print(f"Perceiving repo state at {repo_path}...")
    state = perceiver.gather_state()
    
    print("Auditing state...")
    audited_state = auditor.audit(state)
    
    print(f"Saving audited state to {state_file}...")
    storage.save_state(audited_state)
    
    print(f"Done. Health Score: {audited_state.health_score}/100")
    for alert in audited_state.alerts:
        print(f"[{alert.severity}] {alert.message}")
        
    return 0

async def run_delegate(repo_path, intent):
    perceiver = LocalGitPerceiver(repo_path)
    kernel_client = KernelWebSocketClient()
    
    service = BuildDelegationService(perceiver, kernel_client)
    print(f"▶ Delegating task '{intent}' to CYNIC Hub...")
    try:
        await service.delegate_task(intent)
        print(f"\n▶ Task '{intent}' completed successfully.")
        return 0
    except Exception as e:
        print(f"\n[!] Task '{intent}' failed: {e}", file=sys.stderr)
        return 1

def main():
    parser = argparse.ArgumentParser(description="CYNIC Forge Organ CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)
    
    # Audit Command
    subparsers.add_parser("audit", help="Audit the repository state")
    
    # Delegate Command
    delegate_parser = subparsers.add_parser("delegate", help="Delegate a build task to the distributed network")
    
    # Add an intent argument properly
    delegate_parser.add_argument("intent", help="The intent (e.g., build-and-test)")
    
    args = parser.parse_args()
    
    repo_path = PROJECT_ROOT
    state_file = PROJECT_ROOT / "organs" / "forge" / "state" / "state.json"
    
    if args.command == "audit":
        return run_audit(repo_path, state_file)
    elif args.command == "delegate":
        return asyncio.run(run_delegate(repo_path, args.intent))

if __name__ == "__main__":
    sys.exit(main())
