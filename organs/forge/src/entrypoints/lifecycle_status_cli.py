import json
import sys

from organs.forge.src.adapters.registry_adapter import lifecycle_transition_contract
from organs.forge.src.core.lifecycle_status import build_lifecycle_status_report
from organs.forge.src.entrypoints.lifecycle_diagnostic_cli import lifecycle_blockers_payload


def run_lifecycle_status() -> int:
    try:
        payload = build_lifecycle_status_report(
            lifecycle_transition_contract(),
            lifecycle_blockers_payload(),
        )
        print(json.dumps(payload, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1
