#!/usr/bin/env python3
import sys
from pathlib import Path

# Setup Python path to include the organ root directory
sys.path.append(str(Path(__file__).resolve().parent.parent))

from src.core.entities import JupyterState
from src.adapters.jupyter_adapter import JupyterApiAdapter
from src.adapters.state_adapter import FileStateAdapter
from src.adapters.cynic_kernel_adapter import CynicKernelAdapter

from src.adapters.cynic_kernel_adapter import get_cynic_env

PROJECT_DIR = Path(__file__).resolve().parent.parent.parent
STATE_FILE = PROJECT_DIR / "state" / "state.json"

cynic_env = get_cynic_env()
JUPYTER_API = cynic_env.get("JUPYTER_API", "http://127.0.0.1:8888/api/kernels")
JUPYTER_AUTH = cynic_env.get("JUPYTER_TOKEN", "<JUPYTER_TOKEN>")

def main():
    # 1. Initialize Adapters
    jupyter_adapter = JupyterApiAdapter(JUPYTER_API, JUPYTER_AUTH)
    state_adapter = FileStateAdapter(STATE_FILE)
    kernel_observer = CynicKernelAdapter()

    # 2. Domain Logic / Use Case
    # Fetch current Jupyter status
    status, active_kernels = jupyter_adapter.fetch_status()
    
    # Create the Domain Entity
    state = JupyterState.create(status, active_kernels)

    # Persist the local state
    state_adapter.save_state(state)

    # Push the observation to the CYNIC Kernel
    kernel_observer.observe(state)

if __name__ == "__main__":
    main()
