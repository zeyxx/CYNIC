import requests
import sys
import dataclasses
from pathlib import Path
from ..core.ports import KernelObserverPort
from ..core.entities import JupyterState

def get_cynic_env():
    env_file = Path.home() / ".cynic-env"
    env_vars = {}
    if env_file.exists():
        with open(env_file, "r") as f:
            for line in f:
                if "=" in line and not line.strip().startswith("#"):
                    k, v = line.strip().split("=", 1)
                    v = v.strip("\"'")
                    if k.startswith("export "):
                        k = k[7:]
                    env_vars[k] = v
    return env_vars

class CynicKernelAdapter(KernelObserverPort):
    def __init__(self):
        cynic_env = get_cynic_env()
        self.kernel_addr = cynic_env.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
        self.api_key = cynic_env.get("CYNIC_API_KEY", "")

    def observe(self, state: JupyterState) -> None:
        if not self.kernel_addr or not self.api_key:
            print("Missing CYNIC configuration in ~/.cynic-env", file=sys.stderr)
            return

        observation = {
            "domain": "jupyter",
            "data": dataclasses.asdict(state)
        }
        
        try:
            requests.post(
                f"http://{self.kernel_addr}/observe",
                headers={"Authorization": f"Bearer {self.api_key}", "Content-Type": "application/json"},
                json=observation,
                timeout=5
            )
        except Exception as e:
            print(f"Failed to push observation to kernel: {e}", file=sys.stderr)
