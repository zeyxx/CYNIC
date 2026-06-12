#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: organ-jupyter dispatch script.
import json
import sys
import datetime
from pathlib import Path

try:
    import requests
except ImportError:
    print("requests not found. Ensure this runs in organs/jupyter/.venv", file=sys.stderr)
    sys.exit(1)

PROJECT_DIR = Path(__file__).resolve().parent.parent
STATE_FILE = PROJECT_DIR / "state" / "state.json"
JUPYTER_API = "http://127.0.0.1:8888/api/kernels"
JUPYTER_TOKEN = "cynic-internal-token-2026"

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

def main():
    # 1. Interroger l'API native de Jupyter
    headers = {"Authorization": f"token {JUPYTER_TOKEN}"}
    try:
        resp = requests.get(JUPYTER_API, headers=headers, timeout=5)
        if resp.status_code == 200:
            kernels = resp.json()
            status = "online"
        else:
            kernels = []
            status = "degraded"
    except Exception:
        kernels = []
        status = "offline"

    # 2. Mettre à jour l'état local (state.json)
    state = {
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "status": status,
        "active_kernels": len(kernels),
        "recent_notebooks": [],
        "memory_usage_mb": 0.0 # TODO: fetcher l'usage exact si metrics_enabled
    }
    
    with open(STATE_FILE, "w") as f:
        json.dump(state, f, indent=2)

    # 3. Envoyer le signal au noyau (CYNIC Kernel /observe)
    cynic_env = get_cynic_env()
    kernel_addr = cynic_env.get("CYNIC_REST_ADDR", "127.0.0.1:3030")
    api_key = cynic_env.get("CYNIC_API_KEY", "")
    
    if not kernel_addr or not api_key:
        print("Missing CYNIC configuration in ~/.cynic-env", file=sys.stderr)
        return

    observation = {
        "domain": "jupyter",
        "data": state
    }
    
    try:
        requests.post(
            f"http://{kernel_addr}/observe",
            headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
            json=observation,
            timeout=5
        )
    except Exception as e:
        print(f"Failed to push observation to kernel: {e}", file=sys.stderr)

if __name__ == "__main__":
    main()
