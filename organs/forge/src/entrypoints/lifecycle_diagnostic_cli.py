import json
import os
import subprocess
import sys

from organs.forge.src.core.lifecycle_diagnostic_payloads import (
    context_with_bootstrap_source,
    lifecycle_blockers_payload as _lifecycle_blockers_payload,
    organism_gap_map_payload as _organism_gap_map_payload,
    next_repair_batch_payload as _next_repair_batch_payload,
    lifecycle_root_install_sequence_payload as _lifecycle_root_install_sequence_payload,
    lifecycle_root_install_script as _lifecycle_root_install_script,
    lifecycle_dispatch_error_payload as _lifecycle_dispatch_error_payload,
    lifecycle_error_context,
    lifecycle_mcp_freshness_payload,
)
from organs.forge.src.core.lifecycle_bootstrap_source import probe_bootstrap_source
from organs.forge.src.core.lifecycle_consumer_status import probe_local_consumer_runtime
from organs.forge.src.core.lifecycle_constellation import build_lifecycle_constellation_report
from organs.forge.src.core.lifecycle_readiness import probe_local_network_runtime
from organs.forge.src.core.lifecycle_runtime_artifacts import build_runtime_artifact_manifest
from organs.forge.src.adapters.kernel_adapter import (
    KernelRestClient,
    build_lifecycle_activation_requests,
    build_lifecycle_runtime_paths,
    build_lifecycle_service_evidence_patch,
    lifecycle_health_status_result,
    normalize_kernel_base_url,
    validate_lifecycle_policy,
    verify_lifecycle_runtime_paths,
)
from organs.forge.src.adapters.registry_adapter import load_registry

def _run(func, check_ok=True) -> int:
    try:
        res = func()
        if isinstance(res, dict):
            print(json.dumps(res, separators=(",", ":")))
            is_ok = not check_ok or res.get("status") == "ok" or res.get("accepted")
            return 0 if is_ok else 1
        print(res)
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1


def run_lifecycle_check(strict: bool = True) -> int:
    try:
        kernel_client = KernelRestClient(require_api_key=False, strict_lifecycle=strict)
        if not strict:
            kernel_client.inspect_fleet()
        print(json.dumps(context_with_bootstrap_source(kernel_client.lifecycle_context()), separators=(",", ":")))
        return 0
    except Exception as e:
        payload = {"status": "error", "error": str(e)}
        if strict:
            payload.update(lifecycle_error_context())
        print(json.dumps(payload, separators=(",", ":")), file=sys.stderr)
        return 1

def _activation_snapshot() -> dict:
    activation = build_lifecycle_activation_requests(
        inspect_fleet=True,
        include_network_probe=True,
    )
    activation["bootstrap_source"] = probe_bootstrap_source()
    activation["runtime_probe"] = probe_local_consumer_runtime()
    activation["network_probe"] = probe_local_network_runtime()
    return activation


def lifecycle_dispatch_error_payload(exc: Exception) -> dict:
    return _lifecycle_dispatch_error_payload(str(exc), lifecycle_error_context())


def lifecycle_blockers_payload() -> dict:
    return _lifecycle_blockers_payload(_activation_snapshot())


def run_lifecycle_blockers() -> int:
    return _run(lifecycle_blockers_payload)


def organism_gap_map_payload() -> dict:
    return _organism_gap_map_payload(_activation_snapshot())


def run_organism_gap_map() -> int:
    return _run(organism_gap_map_payload, check_ok=False)


def organism_next_repair_batch_payload() -> dict:
    return _next_repair_batch_payload(organism_gap_map_payload())


def run_organism_next_repair_batch() -> int:
    return _run(organism_next_repair_batch_payload, check_ok=False)


def lifecycle_root_install_sequence_payload() -> dict:
    return _lifecycle_root_install_sequence_payload(_activation_snapshot())


def run_lifecycle_root_install_sequence() -> int:
    return _run(lifecycle_root_install_sequence_payload, check_ok=False)


def lifecycle_root_install_script() -> str:
    return _lifecycle_root_install_script(lifecycle_root_install_sequence_payload())


def run_lifecycle_root_install_script() -> int:
    return _run(lifecycle_root_install_script, check_ok=False)


def run_lifecycle_constellation() -> int:
    return _run(lambda: build_lifecycle_constellation_report(load_registry()))


def run_lifecycle_mcp_freshness() -> int:
    return _run(lifecycle_mcp_freshness_payload)


def run_lifecycle_runtime_artifacts() -> int:
    return _run(lambda: build_runtime_artifact_manifest(load_registry()), check_ok=False)


def run_lifecycle_runtime_paths() -> int:
    return _run(build_lifecycle_runtime_paths)


def run_lifecycle_bootstrap_source_check() -> int:
    return _run(probe_bootstrap_source)


def run_lifecycle_verify_runtime_paths(node_name: str) -> int:
    return _run(lambda: verify_lifecycle_runtime_paths(node_name))


def run_lifecycle_service_evidence_patch(node_name: str, service_states: dict | None = None) -> int:
    return _run(lambda: build_lifecycle_service_evidence_patch(node_name, service_states=service_states))


def run_lifecycle_lint() -> int:
    return _run(validate_lifecycle_policy, check_ok=False)


def run_lifecycle_health_accepted(status_code: str) -> int:
    return _run(lambda: lifecycle_health_status_result(int(status_code)))


def run_kernel_url(raw: str | None = None) -> int:
    try:
        print(normalize_kernel_base_url(raw or os.environ.get("CYNIC_REST_ADDR", "")))
        return 0
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1
