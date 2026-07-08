"""Codex Browser MCP activation use case."""

from __future__ import annotations

import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from organs.browser.src.adapters.direct_cell_runtime import pid_alive, read_pid, wait_http_ready
from adapters.json_exposure_source import JsonExposureSource
from core.services import CodexTomlProjection, ExposureProjectionService


@dataclass(frozen=True)
class CodexBrowserActivationOptions:
    start_services: bool = False
    skip_diagnose: bool = False
    install_deps: bool = False


def activate_codex_browser_mcp(root: Path, options: CodexBrowserActivationOptions) -> int:
    activator = CodexBrowserMcpActivator(root)
    failures = 0
    failures += not activator.ensure_wrapper()
    failures += not activator.ensure_codex_config()
    if options.install_deps:
        failures += not activator.install_deps()
    if options.start_services:
        failures += not activator.systemctl("start", "organ-browser.service")
        failures += not activator.systemctl("start", "organ-browser-hub.service")
    if not options.skip_diagnose:
        failures += not activator.run_diagnose()
    if failures:
        print("activation_status=partial")
        return 1
    print("activation_status=ok")
    print("restart_required=codex")
    return 0


class CodexBrowserMcpActivator:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.config = root / ".codex/config.toml"
        self.wrapper = root / ".cortex/mcp/run_browser_mcp.py"
        self.diagnose = root / "organs/browser/src/entrypoints/diagnose_browser_mcp.py"
        self.hub = root / "organs/browser/src/entrypoints/browser_hub.py"
        self.browser = root / "organs/browser/src/entrypoints/launch_browser.py"
        self.hub_url = os.environ.get("ORGAN_BROWSER_HUB_URL", "http://127.0.0.1:40770")
        self.cdp_url = f"http://127.0.0.1:{os.environ.get('ORGAN_BROWSER_CDP_PORT', '40769')}"
        self.venv = Path(os.environ.get("ORGAN_BROWSER_VENV", str(root / ".cynic/organs/browser/.venv")))
        self.state_dir = Path(os.environ.get("ORGAN_BROWSER_STATE_DIR", str(root / ".cynic/organs/browser")))
        self.hub_pid = self.state_dir / "browser-hub.pid"
        self.browser_pid = self.state_dir / "browser.pid"
        self.exposure_source = root / "infra/agent-exposure.json"

    def ensure_wrapper(self) -> bool:
        if not self.wrapper.exists():
            print(f"fail wrapper_missing {self.wrapper}")
            return False
        try:
            self.wrapper.chmod(0o755)
        except Exception as exc:
            if os.access(self.wrapper, os.X_OK):
                print(f"ok wrapper_executable {self.wrapper} chmod_skipped={exc}")
                return True
            print(f"fail wrapper_chmod {exc}")
            return False
        print(f"ok wrapper_executable {self.wrapper}")
        return True

    def ensure_codex_config(self) -> bool:
        try:
            text = self.config.read_text(encoding="utf-8") if self.config.exists() else ""
            if "mcp_servers.cynic-browser" in text:
                print(f"ok codex_config_already_declares_browser {self.config}")
                return True
            separator = "\n" if text.endswith("\n") or not text else "\n\n"
            self.config.write_text(text + separator + self.render_browser_codex_block() + "\n", encoding="utf-8")
            print(f"ok codex_config_declares_browser {self.config}")
            return True
        except Exception as exc:
            print(f"fail codex_config_declares_browser {self.config} {exc}")
            return False

    def install_deps(self) -> bool:
        try:
            if not (self.venv / "bin/python").exists():
                if not self._run([sys.executable, "-m", "venv", str(self.venv)], "browser_venv_create", 120):
                    return False
            return self._run(
                [str(self.venv / "bin/pip"), "install", "-e", str(self.root / "organs/browser")],
                "browser_venv_deps",
                300,
            )
        except Exception as exc:
            print(f"fail browser_venv_deps {exc}")
            return False

    def render_browser_codex_block(self) -> str:
        service = ExposureProjectionService(
            source=JsonExposureSource(self.exposure_source),
            projections={"codex": CodexTomlProjection()},
        )
        block = service.render("codex")
        start = block.index("[mcp_servers.cynic-browser]")
        return block[start:].strip()

    def systemctl(self, action: str, service: str) -> bool:
        ok, detail = self._run_with_detail(["systemctl", "--user", action, service], f"systemctl_{action}_{service}", 20)
        if ok:
            return True
        if action == "start" and service == "organ-browser.service" and self._systemd_bus_missing(detail):
            print("warn systemd_user_bus_unavailable fallback=direct_browser")
            return self.start_browser_direct()
        if action == "start" and service == "organ-browser-hub.service" and self._systemd_bus_missing(detail):
            print("warn systemd_user_bus_unavailable fallback=direct_browser_hub")
            return self.start_browser_hub_direct()
        return False

    def start_browser_direct(self) -> bool:
        current_pid = read_pid(self.browser_pid)
        if wait_http_ready(f"{self.cdp_url.rstrip('/')}/json/version", timeout_seconds=2):
            print(f"ok browser_direct already_reachable pid={current_pid or 'unknown'}")
            return True
        if pid_alive(current_pid):
            print(f"fail browser_direct_stale pid={current_pid} reason=cdp_not_ready")
            return False

        python = self.venv / "bin/python"
        executable = str(python) if python.exists() and os.access(python, os.X_OK) else sys.executable
        self.state_dir.mkdir(parents=True, exist_ok=True)
        log = self.state_dir / "browser.log"
        env = os.environ.copy()
        env.setdefault("ORGAN_BROWSER_STATE_DIR", str(self.state_dir))
        env.setdefault("DISPLAY", ":0")
        env.setdefault("CHROME_BIN", "/usr/bin/google-chrome")
        env.setdefault("ORGAN_BROWSER_HEADLESS", "1")
        env.setdefault("PYTHONPATH", str(self.root))
        with log.open("ab") as output:
            proc = subprocess.Popen(
                [executable, str(self.browser)],
                cwd=self.root,
                env=env,
                stdout=output,
                stderr=subprocess.STDOUT,
                start_new_session=True,
        )
        self.browser_pid.write_text(f"{proc.pid}\n", encoding="utf-8")
        if not wait_http_ready(f"{self.cdp_url.rstrip('/')}/json/version", timeout_seconds=8):
            print(f"fail browser_direct_readiness pid={proc.pid} url={self.cdp_url}")
            return False
        print(f"ok browser_direct_started pid={proc.pid} log={log}")
        return True

    def start_browser_hub_direct(self) -> bool:
        current_pid = read_pid(self.hub_pid)
        if pid_alive(current_pid):
            if wait_http_ready(f"{self.hub_url.rstrip('/')}/status", timeout_seconds=2):
                print(f"ok browser_hub_direct already_running pid={current_pid}")
                return True
            print(f"warn browser_hub_direct_stale pid={current_pid} reason=http_not_ready")
            try:
                self.hub_pid.unlink()
            except FileNotFoundError:
                pass

        python = self.venv / "bin/python"
        executable = str(python) if python.exists() and os.access(python, os.X_OK) else sys.executable
        self.state_dir.mkdir(parents=True, exist_ok=True)
        log = self.state_dir / "browser-hub.log"
        env = os.environ.copy()
        env.setdefault("ORGAN_BROWSER_STATE_DIR", str(self.state_dir))
        env.setdefault("PYTHONPATH", str(self.root))
        with log.open("ab") as output:
            proc = subprocess.Popen(
                [executable, str(self.hub)],
                cwd=self.root,
                env=env,
                stdout=output,
                stderr=subprocess.STDOUT,
                start_new_session=True,
        )
        self.hub_pid.write_text(f"{proc.pid}\n", encoding="utf-8")
        if not wait_http_ready(f"{self.hub_url.rstrip('/')}/status", timeout_seconds=5):
            print(f"fail browser_hub_direct_readiness pid={proc.pid} url={self.hub_url}")
            return False
        print(f"ok browser_hub_direct_started pid={proc.pid} log={log}")
        return True

    def run_diagnose(self) -> bool:
        python = self.venv / "bin/python"
        executable = str(python) if python.exists() and os.access(python, os.X_OK) else sys.executable
        proc = subprocess.run([executable, str(self.diagnose)], cwd=self.root, text=True, check=False)
        return proc.returncode == 0

    def _run(self, command: list[str], label: str, timeout: int) -> bool:
        ok, _detail = self._run_with_detail(command, label, timeout)
        return ok

    def _run_with_detail(self, command: list[str], label: str, timeout: int) -> tuple[bool, str]:
        proc = subprocess.run(
            command,
            cwd=self.root,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        if proc.returncode == 0:
            print(f"ok {label}")
            return True, ""
        detail = (proc.stdout + proc.stderr).strip().replace("\n", " ")
        print(f"fail {label} {detail}")
        return False, detail

    def _systemd_bus_missing(self, detail: str) -> bool:
        return "Failed to connect to bus" in detail or "No medium found" in detail
