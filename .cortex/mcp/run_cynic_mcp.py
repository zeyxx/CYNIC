#!/usr/bin/env python3
"""Launch the cynic MCP proxy without a shell wrapper."""

from __future__ import annotations

import os
import sys
import json
from pathlib import Path

PROJECT_ROOT_CANDIDATE = Path(__file__).resolve().parents[2]
if str(PROJECT_ROOT_CANDIDATE) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT_CANDIDATE))

from organs.contracts.src.adapters.env import load_env_file
from organs.contracts.src.adapters.mcp_binary import (
    McpBinarySelection,
    newest_stale_source,
    select_mcp_binary,
)
from organs.contracts.src.project_root import resolve_repo_root


BUILD_REQUEST = (
    "Request an organism build/reload through the registry build runner with: cynic mcp-reload. "
    "Legacy fallback when the cynic CLI is unavailable: make mcp-reload-request. "
    "If MCP/kernel/build-runner access is broken, use the emergency repair path: make mcp-repair-local. "
    "For a repair-only debug binary, launch with CYNIC_MCP_BINARY=target/debug/cynic-mcp. "
    "Do not treat any cortex-local compile as canonical validation."
)
PYTHON_FALLBACK = PROJECT_ROOT_CANDIDATE / ".cortex" / "mcp" / "python_rest_mcp.py"


def _print_check(payload: dict[str, object]) -> None:
    print(json.dumps(payload, sort_keys=True))

def _debug_repair_selection(project_root: Path) -> McpBinarySelection | None:
    binary = project_root / "target" / "debug" / "cynic-mcp"
    exists = binary.exists()
    executable = exists and os.access(binary, os.X_OK)
    if not executable:
        return None
    stale = newest_stale_source(project_root / "crates" / "cynic-mcp", binary)
    return McpBinarySelection(binary=binary, exists=exists, executable=executable, stale_source=stale)

def main() -> int:
    load_env_file(Path.home() / ".cynic-env")
    repair_local = os.environ.get("CYNIC_MCP_REPAIR_LOCAL", "0") == "1"
    if not repair_local:
        os.environ.pop("CYNIC_PROJECT_ROOT", None)
    project_root = resolve_repo_root(PROJECT_ROOT_CANDIDATE, prefer_runtime_mirror=not repair_local)
    os.environ["CYNIC_PROJECT_ROOT"] = str(project_root)

    selection = select_mcp_binary(project_root)
    using_debug_repair = False
    if not repair_local and not selection.exists:
        local_root = PROJECT_ROOT_CANDIDATE
        debug_selection = _debug_repair_selection(local_root)
        if debug_selection is not None:
            project_root = local_root
            os.environ["CYNIC_PROJECT_ROOT"] = str(project_root)
            selection = debug_selection
            using_debug_repair = True
    binary = selection.binary

    check_only = os.environ.get("CYNIC_MCP_CHECK_ONLY", "0") == "1"
    if not (selection.exists and selection.executable):
        if check_only:
            _print_check(
                {
                    "schema": "cynic.mcp_launcher_check.v1",
                    "status": "missing",
                    "project_root": str(project_root),
                    "canonical_binary": str(binary),
                    "canonical_binary_exists": selection.exists,
                    "canonical_binary_executable": selection.executable,
                    "fallback": str(PYTHON_FALLBACK),
                    "fallback_available": True,
                    "build_request": BUILD_REQUEST,
                }
            )
            return 0
        print(f"cynic MCP binary is missing or not executable: {binary}", file=sys.stderr)
        print("Starting Python REST MCP fallback; Rust cynic-mcp remains the canonical full proxy.", file=sys.stderr)
        print(BUILD_REQUEST, file=sys.stderr)
        os.execv(sys.executable, [sys.executable, str(PYTHON_FALLBACK)])
        return 127

    if check_only:
        check_payload = {
            "schema": "cynic.mcp_launcher_check.v1",
            "status": "checking",
            "project_root": str(project_root),
            "canonical_binary": str(binary),
            "canonical_binary_exists": selection.exists,
            "canonical_binary_executable": selection.executable,
            "fallback": str(PYTHON_FALLBACK),
            "repair_local_debug_binary": using_debug_repair,
        }

    stale = selection.stale_source
    if stale is not None:
        if check_only:
            check_payload.update(
                {
                    "status": "stale",
                    "stale_source": str(stale),
                    "build_request": BUILD_REQUEST,
                }
            )
            _print_check(check_payload)
        else:
            print(f"cynic MCP binary is stale: {binary} is older than {stale}", file=sys.stderr)
            print("Refusing to start an MCP contract that can drift from repo source.", file=sys.stderr)
            print(BUILD_REQUEST, file=sys.stderr)
        return 78

    if check_only:
        check_payload["status"] = "fresh"
        _print_check(check_payload)
        return 0

    if using_debug_repair:
        print(f"Using cortex-local debug MCP repair binary: {binary}", file=sys.stderr)
        print("This is repair mode, not canonical release validation.", file=sys.stderr)

    os.execv(str(binary), [str(binary)])
    return 127


if __name__ == "__main__":
    raise SystemExit(main())
