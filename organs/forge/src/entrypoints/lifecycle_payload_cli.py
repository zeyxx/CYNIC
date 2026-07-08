import json
import sys

from organs.contracts.src.surface_contracts import json_object_or_default
from organs.forge.src.adapters.kernel_adapter import (
    build_lifecycle_profile_payloads,
    build_lifecycle_task_payload,
    lifecycle_profile_intents,
    validate_lifecycle_task_payload,
)


def run_lifecycle_task_payload(intent: str, domain: str, agent_id: str) -> int:
    try:
        payload = build_lifecycle_task_payload(
            intent=intent,
            domain=domain,
            agent_id=agent_id,
        )
        print(json.dumps(payload, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_validate_task_payload(raw_payload: str) -> int:
    try:
        payload = json_object_or_default(raw_payload)
        result = validate_lifecycle_task_payload(payload)
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_profile_payloads(profile: str, domain: str, agent_id: str) -> int:
    try:
        result = build_lifecycle_profile_payloads(
            profile=profile,
            domain=domain,
            agent_id=agent_id,
        )
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_profile_intents(profile: str) -> int:
    try:
        intents = lifecycle_profile_intents(profile)
        print(
            json.dumps(
                {"status": "ok", "profile": profile, "intents": intents},
                separators=(",", ":"),
            )
        )
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1
