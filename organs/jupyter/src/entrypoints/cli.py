#!/usr/bin/env python3
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[4]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.jupyter.src.adapters.cynic_kernel_adapter import (
    CynicKernelAdapter,
    get_cynic_env,
)
from organs.jupyter.src.adapters.jupyter_adapter import JupyterApiAdapter
from organs.jupyter.src.adapters.state_adapter import FileStateAdapter
from organs.jupyter.src.core.entities import JupyterState


STATE_FILE = PROJECT_ROOT / "organs" / "jupyter" / "state" / "state.json"


def main() -> None:
    cynic_env = get_cynic_env()
    jupyter_api = cynic_env.get("JUPYTER_API", "http://127.0.0.1:8888/api/kernels")
    jupyter_token = cynic_env.get("JUPYTER_TOKEN", "<JUPYTER_TOKEN>")

    jupyter_adapter = JupyterApiAdapter(jupyter_api, jupyter_token)
    state_adapter = FileStateAdapter(STATE_FILE)
    kernel_observer = CynicKernelAdapter()

    status, active_kernels = jupyter_adapter.fetch_status()
    state = JupyterState.create(status, active_kernels)

    state_adapter.save_state(state)
    kernel_observer.observe(state)


if __name__ == "__main__":
    main()
