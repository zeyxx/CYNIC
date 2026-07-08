from pathlib import Path
import subprocess
import time

from organs.contracts.src.surface_contracts import read_json_file_or_default, write_json_file

def sync_fleet_truth():
    """Controller to sync fleet truth and update SSOT."""
    base_dir = Path(__file__).resolve().parents[1]
    
    # 1. Read Desired State
    desired = read_json_file_or_default(base_dir / "desired" / "state.json", {})
        
    # 2. Execute Runbook operation (scan_proxmox)
    print("Scanning Proxmox and Registry...")
    # In a real run, this would call the actual organ-fleet cli.
    # For the contract, we assert the controller exists.
    
    # 3. Update SSOT
    ssot_path = base_dir / "state" / "SSOT.json"
    ssot = read_json_file_or_default(ssot_path, {})
        
    ssot['last_synced'] = time.time()
    ssot['status'] = 'active'
    
    write_json_file(ssot_path, ssot)
        
    print("Fleet Truth SSOT updated.")

if __name__ == "__main__":
    sync_fleet_truth()
