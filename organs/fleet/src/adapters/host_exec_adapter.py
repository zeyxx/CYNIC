from __future__ import annotations

import json
import shutil
import stat
from pathlib import Path

from organs.contracts.src.surface_contracts import read_json_file_or_default

from ..core.host_exec import HostExecPolicy


class HostExecPolicyFile:
    def __init__(self, path: Path) -> None:
        self.path = path

    def load(self) -> HostExecPolicy:
        return HostExecPolicy(read_json_file_or_default(self.path, {}))

    def render_sudoers(self) -> str:
        policy = self.load()
        return (
            "# Generated from organs/fleet/policies/host-exec.policy.json\n"
            "# Allows the cynic service user to invoke only the root-owned host exec helper.\n"
            f"{policy.sudoers_rule()}\n"
        )

    def install_plan(self, helper_source: Path) -> dict[str, object]:
        policy = self.load()
        helper_target = Path(policy.helper_path)
        policy_target = Path("/etc/cynic/host-exec.policy.json")
        sudoers_target = Path("/etc/sudoers.d/cynic-host-exec")
        prepare_directories = [
            f"install -d -o root -g root -m 0755 {helper_target.parent}",
            f"install -d -o root -g root -m 0755 {policy_target.parent}",
            f"install -d -o root -g root -m 0755 {sudoers_target.parent}",
        ]
        return {
            "schema": "cynic.host_exec.install_plan.v1",
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
                "sudo -n /usr/bin/systemctl status cynic-node.service must remain unauthorized for the cynic user",
            ],
        }

    def activation_plan(self, helper_source: Path, *, bundle_dir: Path) -> dict[str, object]:
        install_plan = self.install_plan(helper_source)
        helper_target = str(install_plan["helper_target"])
        policy_target = str(install_plan["policy_target"])
        sudoers_target = str(install_plan["sudoers_target"])
        bundle_dir_str = str(bundle_dir)
        return {
            "schema": "cynic.host_exec.activation_plan.v1",
            "mutates": False,
            "bundle_dir": bundle_dir_str,
            "bundle_expected_complete": self._bundle_complete(bundle_dir),
            "bundle_files": {
                "helper": str(bundle_dir / "cynic-host-exec"),
                "policy": str(bundle_dir / "host-exec.policy.json"),
                "sudoers": str(bundle_dir / "cynic-host-exec.sudoers"),
                "manifest": str(bundle_dir / "manifest.json"),
            },
            "preflight": [
                "python3 organs/fleet/src/entrypoints/cli.py host-exec bundle --output-dir /tmp/cynic-host-exec",
                "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status",
            ],
            "install_commands": install_plan["prepare_directories"] + [
                f"install -o root -g root -m 0755 {bundle_dir_str}/cynic-host-exec {helper_target}",
                f"install -o root -g root -m 0644 {bundle_dir_str}/host-exec.policy.json {policy_target}",
                f"install -o root -g root -m 0440 {bundle_dir_str}/cynic-host-exec.sudoers {sudoers_target}",
                f"visudo -cf {sudoers_target}",
            ],
            "postflight": [
                "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status",
            ],
            "denial_checks": [
                "sudo -n /usr/bin/systemctl status cynic-node.service",
                "sudo -n /bin/sh -c true",
            ],
            "success_criteria": [
                "activation-status reports status=active",
                "direct systemctl/shell commands remain denied for cynic",
            ],
        }

    def write_install_bundle(self, helper_source: Path, output_dir: Path) -> dict[str, object]:
        output_dir.mkdir(parents=True, exist_ok=True)
        helper_target = output_dir / "cynic-host-exec"
        policy_target = output_dir / "host-exec.policy.json"
        sudoers_target = output_dir / "cynic-host-exec.sudoers"
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

    def filesystem_snapshot(
        self,
        helper_path: Path | None = None,
        policy_path: Path | None = None,
        sudoers_path: Path | None = None,
    ) -> dict[str, object]:
        policy = self.load()
        helper_path = helper_path or Path(policy.helper_path)
        policy_path = policy_path or Path("/etc/cynic/host-exec.policy.json")
        sudoers_path = sudoers_path or Path("/etc/sudoers.d/cynic-host-exec")
        return {
            "schema": "cynic.host_exec.filesystem_snapshot.v1",
            "helper": self._path_state(helper_path, executable=True),
            "policy": self._path_state(policy_path),
            "sudoers": {
                **self._path_state(sudoers_path),
                "content": sudoers_path.read_text(encoding="utf-8") if sudoers_path.exists() else "",
            },
        }

    def activation_status(
        self,
        snapshot: dict[str, object],
        *,
        bundle_dir: Path | None = None,
    ) -> dict[str, object]:
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
            "sudoers_no_raw_systemctl": "/usr/bin/systemctl" not in str(sudoers.get("content", "")),
            "sudoers_no_shell": "/bin/sh" not in str(sudoers.get("content", ""))
            and "/usr/bin/bash" not in str(sudoers.get("content", "")),
        }
        missing = [name for name, ok in checks.items() if not ok]
        bundle_complete = self._bundle_complete(bundle_dir)
        if not bundle_complete:
            missing.append("bundle_complete")
        return {
            "schema": "cynic.host_exec.activation_status.v1",
            "status": "active" if not missing else "missing_activation",
            "checks": checks,
            "missing": missing,
            "helper_path": policy.helper_path,
            "sudoers_rule": expected_sudoers,
            "bundle_dir": str(bundle_dir) if bundle_dir else None,
            "bundle_complete": bundle_complete,
            "bundle_evidence_source": "explicit_bundle_dir" if bundle_dir else "none",
            "next_actions": self._next_actions(missing),
            "proof_requirements": self._proof_requirements(
                policy.helper_path,
                str(bundle_dir) if bundle_dir else None,
            ),
        }

    def _bundle_complete(self, bundle_dir: Path | None) -> bool:
        if bundle_dir is None:
            return False
        required = {
            "manifest.json",
            "cynic-host-exec",
            "host-exec.policy.json",
            "cynic-host-exec.sudoers",
        }
        return bundle_dir.exists() and all((bundle_dir / name).exists() for name in required)

    def _next_actions(self, missing: list[str]) -> list[str]:
        actions: list[str] = []
        if "bundle_complete" in missing:
            actions.append("build_host_exec_bundle")
        if any(
            item in missing
            for item in [
                "helper_exists",
                "helper_mode_0755",
                "helper_executable",
                "policy_exists",
                "policy_mode_0644",
                "sudoers_exists",
                "sudoers_mode_0440",
                "sudoers_rule_matches",
            ]
        ):
            actions.append("install_host_exec_bundle_on_target_node")
        if any(
            item in missing
            for item in [
                "sudoers_no_raw_systemctl",
                "sudoers_no_shell",
            ]
        ):
            actions.append("prove_direct_host_commands_remain_denied")
        return actions

    def _proof_requirements(self, helper_path: str, bundle_dir: str | None) -> list[dict[str, object]]:
        return [
            {
                "check": "bundle_complete",
                "command": "python3 organs/fleet/src/entrypoints/cli.py host-exec bundle --output-dir <dir>",
                "expected": "bundle contains manifest.json, cynic-host-exec, host-exec.policy.json and cynic-host-exec.sudoers",
                "evidence_source": bundle_dir or "<dir>",
            },
            {
                "check": "helper_and_policy_installed",
                "command": "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir <dir>",
                "expected": f"helper_path={helper_path} and sudoers_rule matches generated projection",
                "evidence_source": "local_filesystem",
            },
            {
                "check": "direct_commands_denied",
                "command": "sudo -n /usr/bin/systemctl status cynic-node.service ; sudo -n /bin/sh -c true",
                "expected": "non-helper direct systemctl/shell commands remain unauthorized for cynic",
                "evidence_source": "operator_runtime_check",
            },
        ]

    def _prepare_bundle_target(self, path: Path) -> None:
        if path.exists():
            path.chmod(0o600)

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
