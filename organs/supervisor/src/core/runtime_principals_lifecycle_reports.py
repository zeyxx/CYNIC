"""Lifecycle and MCP freshness report builders for runtime principals."""

from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from organs.contracts.src.surface_contracts import json_object_or_default
from organs.contracts.src.surface_contracts import json_stdout_or_envelope


@dataclass(frozen=True)
class RuntimeLifecycleReportContext:
    canonical_project_root: Path
    lifecycle_profile_domains: dict[str, str]
    lifecycle_profile_commands: dict[str, object]
    is_file: Callable[[Path], bool]
    normalize_lifecycle_blocker: Callable[[dict[str, object]], dict[str, object]]


def tail(text: str, limit: int = 1200) -> str:
    if len(text) <= limit:
        return text
    return text[-limit:]


def run_capture(
    command: list[str],
    *,
    cwd: Path,
    timeout: int = 15,
    stdout_limit: int | None = 1200,
    stderr_limit: int | None = 1200,
) -> dict[str, object]:
    def _stdout(text: str | None) -> str:
        if text is None:
            return ""
        return text if stdout_limit is None else tail(text, stdout_limit)

    def _stderr(text: str | None) -> str:
        if text is None:
            return ""
        return text if stderr_limit is None else tail(text, stderr_limit)

    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
    except FileNotFoundError as exc:
        return {
            "status": "missing",
            "command": command,
            "exit_code": 127,
            "stdout": "",
            "stderr": str(exc),
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "status": "timeout",
            "command": command,
            "exit_code": None,
            "stdout": _stdout(exc.stdout),
            "stderr": _stderr(exc.stderr),
        }
    return {
        "status": "ok" if completed.returncode == 0 else "failed",
        "command": command,
        "exit_code": completed.returncode,
        "stdout": _stdout(completed.stdout),
        "stderr": _stderr(completed.stderr),
    }


def mcp_freshness_report(context: RuntimeLifecycleReportContext) -> dict[str, object]:
    wrapper = context.canonical_project_root / ".cortex/mcp/run_cynic_mcp.py"
    if not context.is_file(wrapper):
        return {
            "status": "missing",
            "command": "CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py",
            "proof_requirements": mcp_freshness_proof_requirements("missing", "", ""),
        }
    env = os.environ.copy()
    env["CYNIC_MCP_CHECK_ONLY"] = "1"
    try:
        try:
            completed = subprocess.run(
                [str(wrapper)],
                cwd=context.canonical_project_root,
                env=env,
                check=False,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=15,
            )
        except PermissionError:
            completed = subprocess.run(
                ["python3", str(wrapper)],
                cwd=context.canonical_project_root,
                env=env,
                check=False,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=15,
            )
    except PermissionError:
        return {
            "status": "failed",
            "exit_code": 126,
            "stdout": "",
            "stderr": f"permission denied executing MCP wrapper: {wrapper}",
            "canonical_action": "cynic mcp-reload",
            "mcp_dispatch": "cynic_lifecycle_dispatch(profile=\"mcp-reload\")",
            "proof_requirements": mcp_freshness_proof_requirements(
                "failed",
                "",
                f"permission denied: {wrapper}",
            ),
        }
    except subprocess.TimeoutExpired as exc:
        return {
            "status": "timeout",
            "exit_code": None,
            "stdout": tail(exc.stdout or ""),
            "stderr": tail(exc.stderr or ""),
            "canonical_action": "cynic mcp-reload",
            "mcp_dispatch": "cynic_lifecycle_dispatch(profile=\"mcp-reload\")",
            "proof_requirements": mcp_freshness_proof_requirements("timeout", exc.stdout or "", exc.stderr or ""),
        }
    status_name = "fresh" if completed.returncode == 0 else "stale" if completed.returncode == 78 else "failed"
    return {
        "status": status_name,
        "exit_code": completed.returncode,
        "stdout": tail(completed.stdout),
        "stderr": tail(completed.stderr),
        "canonical_action": "cynic mcp-reload",
        "mcp_dispatch": "cynic_lifecycle_dispatch(profile=\"mcp-reload\")",
        "operator_fallback": "cynic mcp-reload",
        "legacy_operator_fallback": "make mcp-reload-request",
        "proof_requirements": mcp_freshness_proof_requirements(status_name, completed.stdout, completed.stderr),
    }


def mcp_freshness_proof_requirements(status_name: str, stdout: str, stderr: str) -> list[dict[str, object]]:
    selected_binary = ""
    for line in stdout.splitlines():
        if line.startswith("MCP binary:"):
            selected_binary = line.split(":", 1)[1].strip()
            break
        if line.startswith("MCP fallback:"):
            selected_binary = line.split(":", 1)[1].strip()
            break
    stale_source = ""
    marker = " is older than "
    for line in stderr.splitlines():
        if marker in line:
            stale_source = line.split(marker, 1)[1].strip()
            break
    return [
        {
            "check": "mcp_binary_selected",
            "status": "satisfied" if selected_binary else "missing",
            "command": "CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py",
            "expected": "wrapper selects the MCP runtime used by cortex agents",
            "observed": selected_binary,
        },
        {
            "check": "mcp_binary_fresh",
            "status": "satisfied" if status_name == "fresh" else "missing",
            "command": "cynic mcp-reload",
            "expected": "selected MCP runtime is available and the canonical Rust reload is queued when fallback is active",
            "observed": stale_source,
        },
        {
            "check": "mcp_reload_read_back",
            "status": "satisfied" if status_name == "fresh" else "missing",
            "command": "cynic mcp-reload --json",
            "expected": "organ-forge dispatch reports task ids or duplicate pending tasks with Kernel read-back",
            "observed": status_name,
        },
    ]


def lifecycle_profile_report(profile: str, context: RuntimeLifecycleReportContext) -> dict[str, object]:
    command = _lifecycle_profile_payload_command(profile, context)
    result = run_capture(command, cwd=context.canonical_project_root, timeout=20, stdout_limit=None)
    parsed: dict[str, object] | None = None
    if result["status"] == "ok":
        parsed = json_stdout_or_envelope(
            result,
            schema="cynic.lifecycle.profile_report.v1",
            stdout_transform=tail,
        )
        if parsed.get("schema") == "cynic.lifecycle.profile_report.v1":
            result["status"] = "invalid-json"
    payload_summaries: list[dict[str, object]] = []
    if parsed:
        for payload in parsed.get("payloads", []):
            content = payload.get("content", "{}") if isinstance(payload, dict) else "{}"
            content_payload = json_object_or_default(content)
            payload_summaries.append(
                {
                    "kind": payload.get("kind") if isinstance(payload, dict) else None,
                    "domain": payload.get("domain") if isinstance(payload, dict) else None,
                    "intent": content_payload.get("intent"),
                    "build_target": content_payload.get("build_target"),
                    "deploy_target": content_payload.get("deploy_target"),
                    "activation_requirements": content_payload.get("activation_requirements", {}),
                    "targets": content_payload.get("targets", []),
                    "coord_targets": content_payload.get("coord_targets", []),
                    "deploy": content_payload.get("deploy"),
                }
            )
    return {
        "profile": profile,
        "status": parsed.get("status", result["status"]) if parsed else result["status"],
        "exit_code": result["exit_code"],
        "intents": parsed.get("intents", []) if parsed else [],
        "payloads": payload_summaries,
        "stderr": result["stderr"],
        "mutates_mempool": False,
        "canonical_dispatch": f"cynic {profile}",
        "mcp_dispatch": f"cynic_lifecycle_dispatch(profile=\"{profile}\")",
    }


def lifecycle_operator_contract(profile: str) -> dict[str, object]:
    return {
        "inspect_command": f"cynic {profile} --dry-run",
        "dispatch_command": f"cynic {profile}",
        "dispatch_json_command": f"cynic {profile} --json",
        "operator_sequence": [
            {"step": "inspect", "command": f"cynic {profile} --dry-run", "mutates_mempool": False},
            {"step": "dispatch", "command": f"cynic {profile}", "mutates_mempool": True},
            {"step": "dispatch_json", "command": f"cynic {profile} --json", "mutates_mempool": True},
            {"step": "observe", "command": "cynic organism", "mutates_mempool": False},
        ],
        "proof_commands": ["cynic actions", "cynic organism"],
        "done_when": [
            "dry-run payloads resolve from infra/registry.json without cortex-local build decisions",
            f"cynic {profile} or cynic {profile} --json dispatch reports task ids or existing pending duplicates with read-back",
            "cynic actions and cynic organism show updated lifecycle pressure after organ-forge consumes tasks",
        ],
    }


def lifecycle_profile_dry_run(profile: str, context: RuntimeLifecycleReportContext) -> dict[str, object]:
    if profile not in context.lifecycle_profile_commands:
        raise SystemExit(f"ERROR: unknown lifecycle profile: {profile}")
    command = _lifecycle_profile_payload_command(profile, context)
    result = run_capture(command, cwd=context.canonical_project_root, timeout=20, stdout_limit=None)
    payload = json_stdout_or_envelope(
        result,
        schema="cynic.lifecycle.profile_dry_run.v1",
        stdout_transform=tail,
    )
    payload["schema"] = payload.get("schema", "cynic.lifecycle.profile_dry_run.v1")
    payload["exit_code"] = result["exit_code"]
    payload["command"] = f"cynic {profile} --dry-run"
    payload["dispatch_command"] = f"cynic {profile}"
    payload["fallback_command"] = " ".join(command)
    payload["mutates_mempool"] = False
    payload.update(lifecycle_operator_contract(profile))
    return payload


def lifecycle_activation_report(context: RuntimeLifecycleReportContext) -> dict[str, object]:
    result = run_capture(
        ["python3", "organs/forge/src/entrypoints/cli.py", "lifecycle-blockers"],
        cwd=context.canonical_project_root,
        timeout=20,
        stdout_limit=None,
    )
    parsed = json_stdout_or_envelope(
        result,
        schema="cynic.lifecycle.activation_summary.v1",
        stdout_transform=tail,
    )
    if parsed.get("schema") == "cynic.lifecycle.activation_summary.v1" and parsed.get("stdout") == tail(str(result["stdout"])):
        return {
            "status": result["status"],
            "exit_code": result["exit_code"],
            "schema": "cynic.lifecycle.activation_summary.v1",
            "stdout": tail(str(result["stdout"])),
            "stderr": result["stderr"],
            "mutates_mempool": False,
        }
    activation_requests = []
    for request in parsed.get("activation_request_summary", []):
        if isinstance(request, dict):
            activation_requests.append(
                {
                    "action": request.get("action"),
                    "node": request.get("node"),
                    "role": request.get("role"),
                    "code": request.get("code"),
                    "consumer": request.get("consumer"),
                    "priority": request.get("priority"),
                    "coord_targets": request.get("coord_targets", []),
                }
            )
    blockers = []
    for blocker in parsed.get("blockers", []):
        if isinstance(blocker, dict):
            blockers.append(context.normalize_lifecycle_blocker(blocker))
    return {
        "schema": "cynic.lifecycle.activation_summary.v1",
        "status": parsed.get("status", result["status"]),
        "exit_code": result["exit_code"],
        "activation_schema_version": parsed.get("activation_schema_version"),
        "build_target": parsed.get("build_target"),
        "deploy_target": parsed.get("deploy_target"),
        "status_nodes_ready": parsed.get("status_nodes_ready"),
        "fleet_nodes_ready": parsed.get("fleet_nodes_ready"),
        "lifecycle_nodes_ready": parsed.get("lifecycle_nodes_ready"),
        "blockers": blockers,
        "activation_request_count": parsed.get("activation_request_count", len(activation_requests)),
        "activation_request_summary": activation_requests,
        "activation_dispatch_command": "cynic activate",
        "activation_dispatch_fallback": parsed.get("activation_dispatch_command"),
        "mutates_mempool": False,
    }


def _lifecycle_profile_payload_command(profile: str, context: RuntimeLifecycleReportContext) -> list[str]:
    return [
        "python3",
        "organs/forge/src/entrypoints/cli.py",
        "lifecycle-profile-payloads",
        profile,
        context.lifecycle_profile_domains.get(profile, "lifecycle-profile-dry-run"),
        "organ-forge-request",
    ]
