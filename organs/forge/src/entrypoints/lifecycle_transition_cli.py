import json
import sys

from organs.forge.src.adapters.registry_adapter import lifecycle_transition_contract
from organs.forge.src.adapters.registry_adapter import lifecycle_transition_debt
from organs.forge.src.entrypoints.lifecycle_diagnostic_cli import lifecycle_blockers_payload


def run_lifecycle_transition_contract() -> int:
    try:
        payload = lifecycle_transition_contract()
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_transition_debt() -> int:
    try:
        activation = lifecycle_blockers_payload()
        payload = lifecycle_transition_debt(activation_report=activation)
        print(json.dumps(payload, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1
