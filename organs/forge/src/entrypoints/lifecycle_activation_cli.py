import json
import sys

from organs.contracts.src.surface_contracts import json_object_or_default
from organs.forge.src.adapters.kernel_adapter import (
    apply_lifecycle_activation_evidence_patch,
    build_lifecycle_activation_plan,
    build_lifecycle_activation_requests,
    dispatch_lifecycle_activation_requests,
    persist_lifecycle_activation_evidence_patch,
    persist_lifecycle_registry_node_activation,
)
from organs.forge.src.entrypoints.lifecycle_diagnostic_cli import lifecycle_dispatch_error_payload


def run_lifecycle_activation_plan() -> int:
    try:
        payload = build_lifecycle_activation_plan(
            inspect_fleet=True,
            include_network_probe=True,
        )
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_activation_requests() -> int:
    try:
        payload = build_lifecycle_activation_requests(
            inspect_fleet=True,
            include_network_probe=True,
        )
        print(json.dumps(payload, separators=(",", ":")))
        return 0 if payload["status"] == "ok" else 1
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_activation_dispatch() -> int:
    try:
        payload = dispatch_lifecycle_activation_requests(
            inspect_fleet=True,
            include_network_probe=True,
        )
        print(json.dumps(payload, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps(lifecycle_dispatch_error_payload(e), separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_validate_activation_evidence_patch(raw_patch: str) -> int:
    try:
        patch = json_object_or_default(raw_patch)
        result = apply_lifecycle_activation_evidence_patch(patch)
        print(
            json.dumps(
                {
                    "status": result["status"],
                    "node": result["node"],
                    "role": result["role"],
                    "accepted_runtime_paths": result["accepted_runtime_paths"],
                    "accepted_services": result["accepted_services"],
                },
                separators=(",", ":"),
            )
        )
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_apply_activation_evidence_patch(
    raw_patch: str,
    registry_path: str | None = None,
) -> int:
    try:
        patch = json_object_or_default(raw_patch)
        result = persist_lifecycle_activation_evidence_patch(
            patch,
            registry_path=registry_path,
        )
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_activate_node(
    node_name: str,
    registry_path: str | None = None,
) -> int:
    try:
        result = persist_lifecycle_registry_node_activation(
            node_name,
            registry_path=registry_path,
        )
        print(json.dumps(result, separators=(",", ":")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1
