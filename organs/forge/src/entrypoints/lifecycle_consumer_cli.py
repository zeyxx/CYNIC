import json
import sys

from organs.forge.src.adapters.kernel_adapter import KernelRestClient
from organs.forge.src.core.lifecycle_consumer_status import (
    build_lifecycle_consumer_status_report,
    probe_local_consumer_runtime,
)


def run_lifecycle_consumer_status() -> int:
    try:
        client = KernelRestClient(require_api_key=False, strict_lifecycle=False)
        kind = client.lifecycle["task_kind"]
        domain = client.lifecycle["task_domain"]
        pending = client.kernel_tasks.list_agent_tasks(
            kind=kind,
            domain=domain,
            completed=False,
            limit=100,
        )
        completed = client.kernel_tasks.list_agent_tasks(
            kind=kind,
            domain=domain,
            completed=True,
            limit=20,
        )
        payload = build_lifecycle_consumer_status_report(
            pending_payload=pending,
            completed_payload=completed,
            kind=kind,
            domain=domain,
            runtime_probe=probe_local_consumer_runtime(),
        )
        print(json.dumps(payload, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1
