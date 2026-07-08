#!/usr/bin/env python3
"""Diagnostic checks for browser MCP bridge readiness."""

from __future__ import annotations

import os
import sys
from pathlib import Path

try:
    from organs.browser.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


ROOT = bootstrap_repo_path()
ORGAN_VENV = Path(os.environ.get("ORGAN_BROWSER_VENV", str(ROOT / ".cynic/organs/browser/.venv")))
ORGAN_PYTHON = ORGAN_VENV / "bin/python"
if ORGAN_PYTHON.exists() and os.access(ORGAN_PYTHON, os.X_OK) and Path(sys.prefix).resolve() != ORGAN_VENV.resolve():
    os.execv(str(ORGAN_PYTHON), [str(ORGAN_PYTHON), __file__, *sys.argv[1:]])

from organs.browser.src.adapters.mcp_diagnostics import (  # noqa: E402
    check_mcp_protocol,
    first_active_user_unit,
)
from organs.browser.src.adapters.direct_cell_runtime import pid_alive, read_pid  # noqa: E402
from organs.browser.src.adapters.hub_status import read_browser_hub_status  # noqa: E402

HUB_URL = os.environ.get("ORGAN_BROWSER_HUB_URL", "http://127.0.0.1:40770")
STATE_DIR = Path(os.environ.get("ORGAN_BROWSER_STATE_DIR", str(ROOT / ".cynic/organs/browser")))
HUB_PID = STATE_DIR / "browser-hub.pid"


def check(name: str, ok: bool, detail: str = "") -> bool:
    status = "ok" if ok else "fail"
    print(f"{status} {name}{(' ' + detail) if detail else ''}")
    return ok


def main() -> int:
    failures = 0
    wrapper = ROOT / ".cortex/mcp/run_browser_mcp.py"
    config = ROOT / ".codex/config.toml"
    server = ROOT / "organs/browser/src/entrypoints/mcp_server.py"

    failures += not check("wrapper_exists", wrapper.exists(), str(wrapper))
    failures += not check("wrapper_executable", os.access(wrapper, os.X_OK), str(wrapper))
    failures += not check("server_exists", server.exists(), str(server))

    failures += not check("python_mcp_import", True, "not required: stdio JSON-RPC runtime is local")

    try:
        sys.path.insert(0, str(ROOT))
        from organs.browser.src.adapters.mcp_server_runtime import browser_policy

        failures += not check("server_import", browser_policy().get("mutation_default") == "blocked")
    except ImportError:
        failures += not check("server_import", False, "module import failed")
    except Exception as exc:
        failures += not check("server_import", False, str(exc))
    finally:
        try:
            sys.path.remove(str(ROOT))
        except ValueError:
            pass

    failures += not check("mcp_protocol", check_mcp_protocol(ROOT, wrapper))

    try:
        text = config.read_text(encoding="utf-8")
        failures += not check("codex_config_declares_browser", "mcp_servers.cynic-browser" in text)
    except Exception as exc:
        failures += not check("codex_config_declares_browser", False, str(exc))

    hub_status = read_browser_hub_status(HUB_URL)
    failures += not check("browser_hub_http", hub_status.http_ok, hub_status.observed)
    failures += not check(
        "browser_cdp_reachable",
        hub_status.http_ok and hub_status.chrome_reachable,
        hub_status.chrome_detail if hub_status.http_ok else "hub_http_failed",
    )

    active_browser_unit = first_active_user_unit(ROOT, [
        "organ-browser-hub.service",
        "hermes-browser-hub.service",
    ])

    direct_hub_pid = read_pid(HUB_PID)
    direct_hub_active = pid_alive(direct_hub_pid)
    failures += not check(
        "systemd_browser_hub_active",
        active_browser_unit is not None or direct_hub_active,
        active_browser_unit or (f"direct pid={direct_hub_pid}" if direct_hub_active else "inactive"),
    )

    return 1 if failures else 0

if __name__ == "__main__":
    raise SystemExit(main())
