"""Runtime probes for organism stack cell evidence."""

from __future__ import annotations

import os
import re
import subprocess
from pathlib import Path

from organs.browser.src.adapters.hub_status import read_browser_hub_status
from organs.contracts.src.adapters.mcp_binary import select_mcp_binary
from organs.supervisor.src.core.organism_stack_utils import rel, tail


def cell_evidence(root: Path) -> list[dict[str, object]]:
    selected = selected_mcp_binary(root)
    browser_import = browser_runtime_importable(root)
    hub_status = browser_hub_status()
    cdp_status = browser_cdp_status(hub_status)
    return [
        {
            "layer": "L3",
            "entity": "cynic-mcp",
            "kind": "binary",
            "desired": "fresh executable selected by .cortex/mcp/run_cynic_mcp.py",
            "observed": selected["observed"],
            "health": selected["health"],
            "owner": "crates/cynic-mcp",
            "next_action": "cynic mcp-reload",
        },
        {
            "layer": "L3",
            "entity": "organ-browser-mcp",
            "kind": "python_runtime",
            "desired": "importable browser MCP runtime with organ dependencies",
            "observed": browser_import["observed"],
            "health": browser_import["health"],
            "owner": "organs/browser",
            "next_action": "python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --install-deps",
        },
        {
            "layer": "L3",
            "entity": "organ-browser-hub",
            "kind": "service",
            "desired": "Browser Hub HTTP service reachable for MCP tools",
            "observed": hub_status["observed"],
            "health": hub_status["health"],
            "owner": "organs/browser",
            "next_action": "python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --start-services",
        },
        {
            "layer": "L3",
            "entity": "organ-browser-cdp",
            "kind": "service_dependency",
            "desired": "Chrome CDP reachable behind Browser Hub for perception and rendering",
            "observed": cdp_status["observed"],
            "health": cdp_status["health"],
            "owner": "organs/browser",
            "next_action": "python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --start-services",
        },
    ]


def selected_mcp_binary(root: Path) -> dict[str, str]:
    selection = select_mcp_binary(root)
    if not (selection.exists and selection.executable):
        fallback = root / ".cortex" / "mcp" / "python_rest_mcp.py"
        if fallback.exists():
            return {
                "health": "degraded",
                "observed": f"Rust cynic-mcp missing; Python REST MCP fallback available at {rel(root, fallback)}",
            }
        return {"health": "blocked", "observed": "no executable cynic-mcp binary found"}
    if selection.stale_source:
        return {
            "health": "blocked",
            "observed": f"stale binary {selection.binary} older than {rel(root, selection.stale_source)}",
        }
    return {"health": "ok", "observed": f"selected {selection.binary}"}


def browser_runtime_importable(root: Path) -> dict[str, str]:
    code = "import organs.browser.src.adapters.mcp_server_runtime"
    env = dict(os.environ)
    env["PYTHONPATH"] = os.pathsep.join([str(root), env.get("PYTHONPATH", "")])
    organ_venv = Path(os.environ.get("ORGAN_BROWSER_VENV", str(root / ".cynic/organs/browser/.venv")))
    organ_python = organ_venv / "bin/python"
    python = str(organ_python) if organ_python.exists() and os.access(organ_python, os.X_OK) else "python3"
    proc = subprocess.run(
        [python, "-c", code],
        cwd=root,
        env=env,
        text=True,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if proc.returncode == 0:
        return {"health": "ok", "observed": "browser MCP runtime importable"}
    missing = re.search(r"No module named '([^']+)'", proc.stderr)
    if missing:
        return {"health": "degraded", "observed": f"missing Python module {missing.group(1)}"}
    return {"health": "degraded", "observed": tail(proc.stderr or proc.stdout)}


def browser_hub_status() -> dict[str, object]:
    hub_url = os.environ.get("ORGAN_BROWSER_HUB_URL", "http://127.0.0.1:40770").rstrip("/")
    status = read_browser_hub_status(hub_url)
    return {
        "health": "ok" if status.http_ok else "degraded",
        "observed": status.observed,
        "payload": status.payload,
    }


def browser_cdp_status(hub_status: dict[str, object]) -> dict[str, str]:
    if hub_status.get("health") != "ok":
        return {
            "health": "degraded",
            "observed": f"Browser Hub unavailable, cannot observe Chrome CDP: {hub_status.get('observed')}",
        }
    payload = hub_status.get("payload")
    if not isinstance(payload, dict):
        return {"health": "degraded", "observed": "Browser Hub status payload missing"}
    chrome = str(payload.get("chrome") or "unknown")
    if chrome == "reachable":
        return {"health": "ok", "observed": "Chrome CDP reachable through Browser Hub"}
    blockers = payload.get("blockers")
    if isinstance(blockers, list) and blockers:
        return {"health": "degraded", "observed": f"Chrome CDP {chrome}: {tail('; '.join(str(item) for item in blockers))}"}
    return {"health": "degraded", "observed": f"Chrome CDP {chrome}"}
