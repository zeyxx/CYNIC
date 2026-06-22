import os
import sys
from pathlib import Path

# Fix python path for imports
repo_root = Path(__file__).parent.parent.parent.parent.parent
if str(repo_root) not in sys.path:
    sys.path.insert(0, str(repo_root))

from organs.fleet.src.adapters.registry_adapter import FileRegistryAdapter
from organs.fleet.src.adapters.kernel_adapter import HttpKernelAdapter
from organs.fleet.src.core.fleet_workflow import FleetWorkflow

def main():
    registry_path = repo_root / "infra" / "registry.json"
    
    kernel_url = os.environ.get("CYNIC_REST_ADDR", "http://127.0.0.1:3030")
    api_key = os.environ.get("CYNIC_API_KEY", "")

    registry_port = FileRegistryAdapter(registry_path)
    kernel_port = HttpKernelAdapter(kernel_url, api_key)
    
    workflow = FleetWorkflow(registry_port, kernel_port)
    dispatched = workflow.evaluate_and_dispatch()
    
    if dispatched > 0:
        print(f"Dispatched {dispatched} provisioning tasks.")
    else:
        print("No planned nodes requiring action.")

if __name__ == "__main__":
    main()
