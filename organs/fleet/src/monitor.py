import json
import os
import requests
import sys
from pathlib import Path

def main():
    repo_root = Path(__file__).parent.parent.parent.parent
    registry_path = repo_root / "infra" / "registry.json"

    with open(registry_path, "r") as f:
        registry = json.load(f)

    # Search for nodes that are 'planned'
    planned_nodes = []
    for target in registry.get("lifecycle", {}).get("targets", []):
        if target.get("status") == "planned":
            planned_nodes.append(target)

    # In our specific structure, the target is in the root list
    if isinstance(registry, list):
        for node in registry:
            if node.get("status") == "planned":
                planned_nodes.append(node)

    if not planned_nodes:
        print("No planned nodes found.")
        return

    # Create a task for each planned node
    for node in planned_nodes:
        node_name = node.get("name")
        print(f"Found planned node: {node_name}. Dispatching provisioning task...")

        task_payload = {
            "kind": "hermes",
            "domain": "organ-fleet",
            "content": json.dumps({
                "objective": f"Provision {node_name} on its target host, establish dedicated user, harden SSH, and secure Tailscale.",
                "node_spec": node,
                "due_at": "2026-06-25T00:00:00Z",
                "owner": "organ-fleet-hermes-agent",
                "done_when": [
                    "Node is reachable via SSH using the dedicated user.",
                    "Root login is disabled via SSH.",
                    "Tailscale IP is active and reachable.",
                    "Registry status updated to 'active' with activation evidence."
                ]
            }),
            "agent_id": "organ-fleet-monitor"
        }

        kernel_url = os.environ.get("CYNIC_REST_ADDR", "http://127.0.0.1:3030")
        api_key = os.environ.get("CYNIC_API_KEY", "")

        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}"
        }

        try:
            resp = requests.post(f"{kernel_url}/agent-tasks", json=task_payload, headers=headers)
            if resp.status_code in (200, 201):
                print(f"Successfully dispatched task for {node_name}.")
            else:
                print(f"Failed to dispatch task for {node_name}: {resp.text}")
        except Exception as e:
            print(f"Error connecting to kernel: {e}")

if __name__ == "__main__":
    main()
