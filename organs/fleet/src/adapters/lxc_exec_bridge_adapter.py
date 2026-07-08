from __future__ import annotations

import json
import stat
import shutil
from pathlib import Path

from organs.contracts.src.surface_contracts import read_json_file_or_default

from ..core.exec_bridge import LxcExecBridgePolicy


class LxcExecBridgePolicyFile:
    def __init__(self, path: Path) -> None:
        self.path = path

    def load(self) -> LxcExecBridgePolicy:
        return LxcExecBridgePolicy(read_json_file_or_default(self.path, {}))

    def render_sudoers(self) -> str:
        policy = self.load()
        return (
            "# Generated from organs/fleet/policies/lxc-exec-bridge.policy.json\n"
            "# Allows the cynic service user to invoke only the root-owned LXC exec bridge helper.\n"
            f"{policy.sudoers_rule()}\n"
        )

    def install_plan(self, helper_source: Path) -> dict[str, object]:
        policy = self.load()
        helper_target = Path(policy.helper_path)
        policy_target = Path("/etc/cynic/lxc-exec-bridge.policy.json")
        sudoers_target = Path("/etc/sudoers.d/cynic-lxc-exec-bridge")
        prepare_directories = [
            f"install -d -o root -g root -m 0755 {helper_target.parent}",
            f"install -d -o root -g root -m 0755 {policy_target.parent}",
            f"install -d -o root -g root -m 0755 {sudoers_target.parent}",
        ]
        return {
            "schema": "cynic.lxc_exec_bridge.install_plan.v1",
            "mutates": False,
            "helper_source": str(helper_source),
            "helper_target": str(helper_target),
            "policy_source": str(self.path),
            "policy_target": str(policy_target),
            "sudoers_target": str(sudoers_target),
            "sudoers_rule": policy.sudoers_rule(),
            "prepare_directories": prepare_directories,
            "required_owner": "root:root",
            "required_modes": {
                "helper": "0755",
                "policy": "0644",
                "sudoers": "0440",
            },
            "activation_checks": prepare_directories + [
                "install -o root -g root -m 0755 helper_source helper_target",
                "install -o root -g root -m 0644 policy_source policy_target",
                "install -o root -g root -m 0440 rendered_sudoers sudoers_target",
                "visudo -cf sudoers_target",
                "sudo -n helper_target --dry-run < read_only_request.json",
                "sudo -n /usr/sbin/pct status 103 must remain unauthorized for the cynic user",
            ],
        }

    def write_install_bundle(self, helper_source: Path, output_dir: Path) -> dict[str, object]:
        output_dir.mkdir(parents=True, exist_ok=True)
        helper_target = output_dir / "cynic-lxc-exec-bridge"
        policy_target = output_dir / "lxc-exec-bridge.policy.json"
        sudoers_target = output_dir / "cynic-lxc-exec-bridge.sudoers"
        manifest_target = output_dir / "manifest.json"

        self._prepare_bundle_target(helper_target)
        shutil.copyfile(helper_source, helper_target)
        helper_target.chmod(0o755)
        self._prepare_bundle_target(policy_target)
        shutil.copyfile(self.path, policy_target)
        policy_target.chmod(0o644)
        self._prepare_bundle_target(sudoers_target)
        sudoers_target.write_text(self.render_sudoers(), encoding="utf-8")
        sudoers_target.chmod(0o440)

        plan = self.install_plan(helper_source)
        manifest = {
            **plan,
            "bundle_dir": str(output_dir),
            "bundle_files": {
                "helper": str(helper_target),
                "policy": str(policy_target),
                "sudoers": str(sudoers_target),
                "manifest": str(manifest_target),
            },
        }
        manifest_target.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        manifest_target.chmod(0o644)
        return manifest

    def _prepare_bundle_target(self, path: Path) -> None:
        if path.exists():
            path.chmod(0o600)

    def filesystem_snapshot(
        self,
        helper_path: Path | None = None,
        policy_path: Path | None = None,
        sudoers_path: Path | None = None,
    ) -> dict[str, object]:
        policy = self.load()
        helper_path = helper_path or Path(policy.helper_path)
        policy_path = policy_path or Path("/etc/cynic/lxc-exec-bridge.policy.json")
        sudoers_path = sudoers_path or Path("/etc/sudoers.d/cynic-lxc-exec-bridge")
        return {
            "schema": "cynic.lxc_exec_bridge.filesystem_snapshot.v1",
            "helper": self._path_state(helper_path, executable=True),
            "policy": self._path_state(policy_path),
            "sudoers": {
                **self._path_state(sudoers_path),
                "content": sudoers_path.read_text(encoding="utf-8") if sudoers_path.exists() else "",
            },
        }

    def activation_status(self, snapshot: dict[str, object]) -> dict[str, object]:
        policy = self.load()
        expected_sudoers = policy.sudoers_rule()
        helper = snapshot.get("helper", {}) if isinstance(snapshot.get("helper"), dict) else {}
        policy_file = snapshot.get("policy", {}) if isinstance(snapshot.get("policy"), dict) else {}
        sudoers = snapshot.get("sudoers", {}) if isinstance(snapshot.get("sudoers"), dict) else {}
        checks = {
            "helper_exists": bool(helper.get("exists")),
            "helper_executable": bool(helper.get("executable")),
            "helper_mode_0755": helper.get("mode") == "0755",
            "policy_exists": bool(policy_file.get("exists")),
            "policy_mode_0644": policy_file.get("mode") == "0644",
            "sudoers_exists": bool(sudoers.get("exists")),
            "sudoers_mode_0440": sudoers.get("mode") == "0440",
            "sudoers_rule_matches": expected_sudoers in str(sudoers.get("content", "")),
            "sudoers_no_raw_pct": "/usr/sbin/pct" not in str(sudoers.get("content", ""))
            and "/usr/bin/pct" not in str(sudoers.get("content", "")),
            "sudoers_no_shell": "/bin/sh" not in str(sudoers.get("content", ""))
            and "/usr/bin/bash" not in str(sudoers.get("content", "")),
        }
        missing = [name for name, ok in checks.items() if not ok]
        return {
            "schema": "cynic.lxc_exec_bridge.activation_status.v1",
            "status": "active" if not missing else "missing_activation",
            "checks": checks,
            "missing": missing,
            "helper_path": policy.helper_path,
            "sudoers_rule": expected_sudoers,
        }

    def _path_state(self, path: Path, executable: bool = False) -> dict[str, object]:
        if not path.exists():
            return {
                "path": str(path),
                "exists": False,
                "mode": None,
                "executable": False,
            }
        mode = stat.S_IMODE(path.stat().st_mode)
        return {
            "path": str(path),
            "exists": True,
            "mode": f"{mode:04o}",
            "executable": bool(mode & stat.S_IXUSR) if executable else None,
        }
