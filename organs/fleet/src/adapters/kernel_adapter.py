import os
import json
import requests
from ..core.entities import FleetNode
from ..core.ports import KernelPort

class HttpKernelAdapter(KernelPort):
    def __init__(self, rest_addr: str, api_key: str):
        self.rest_addr = rest_addr
        self.api_key = api_key

    def dispatch_provisioning_task(self, node: FleetNode) -> bool:
        task_payload = {
            "kind": "hermes",
            "domain": "organ-fleet",
            "content": json.dumps({
                "objective": f"Provision {node.name} on its target host, establish dedicated user, harden SSH, and secure Tailscale.",
                "node_spec": node.raw_spec,
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

        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {self.api_key}"
        }

        try:
            resp = requests.post(f"{self.rest_addr}/agent-tasks", json=task_payload, headers=headers)
            return resp.status_code in (200, 201)
        except Exception:
            return False
