"""Executor for registry-derived lifecycle bootstrap contracts."""

from __future__ import annotations

from dataclasses import dataclass
import os
from pathlib import Path
from typing import Protocol


class BootstrapTransport(Protocol):
    def execute(
        self,
        *,
        node: str,
        command: str,
        timeout_seconds: int,
    ) -> dict:
        """Execute a command on a registry node through the canonical fleet transport."""


@dataclass(frozen=True)
class BootstrapCommand:
    stage: str
    command: str
    timeout_seconds: int = 300

    def as_fleet_node_exec_request(self, node: str) -> dict:
        return {
            "tool": "fleet_node_exec",
            "node": node,
            "command": self.command,
            "timeout_seconds": self.timeout_seconds,
        }


def _expand_runtime_path(path: str, *, principal: str, project_slug: str) -> str:
    expanded = path.replace("${CYNIC_PROJECT_SLUG}", project_slug)
    # Use Path.home() which corresponds to the user running the script,
    # assuming 'principal' is the same user. For remote execution,
    # this might need adjustment if principal represents a different user.
    home_dir = Path.home()
    if expanded.startswith("%h/"):
        return str(home_dir / expanded[3:])
    if expanded == "%h":
        return str(home_dir)
    # The following lines are problematic as they hardcode a user.
    # We will assume that if the path is not relative to home, it should
    # be treated as-is, or there's a different logic for remote paths.
    

    return expanded


def build_bootstrap_commands(contract: dict) -> list[BootstrapCommand]:
    if contract.get("schema") != "cynic.lifecycle.build_runner_bootstrap.v1":
        raise RuntimeError("unsupported bootstrap contract schema")
    node = contract.get("node")
    if not node:
        raise RuntimeError("bootstrap contract node missing")

    repo = contract.get("repo", {})
    toolchain = contract.get("toolchain", {})
    build = contract.get("build", {})
    watchdog = contract.get("watchdog", {})
    principal = contract.get("access", {}).get("principal", "cynic")
    project_slug = repo.get("project_slug", os.environ.get("CYNIC_PROJECT_SLUG", "CYNIC"))
    repo_path = _expand_runtime_path(
        repo.get("path", str(Path.home() / ".cynic/repos/CYNIC")),
        principal=principal,
        project_slug=project_slug,
    )
    source_path_env = repo.get("source_path_env", "CYNIC_PROJECT_ROOT")
    repo_source = repo.get("source", {})
    rust_toolchain = toolchain.get("rust_toolchain")
    rust_target = toolchain.get("rust_release_target")
    if not rust_toolchain or not rust_target:
        raise RuntimeError("bootstrap contract toolchain incomplete")

    host_packages = " ".join(toolchain.get("host_packages", []))
    commands = [
        BootstrapCommand(
            "runtime_identity",
            f"id -u {principal} >/dev/null && test -d {Path.home()}",
            30,
        ),
    ]
    if repo.get("materialization") == "rsync":
        commands.append(
            BootstrapCommand(
                "repo_materialize",
                "src=${"
                + source_path_env
                + ':-}; dest="'
                + repo_path
                + '"; test -n "$src" && test -d "$src/.git" '
                + '&& if [ "$src" != "$dest" ]; then '
                + 'tmp="${dest}.tmp"; rm -rf "$tmp" && mkdir -p "$(dirname "$dest")" "$tmp" '
                + '&& cp -a "$src"/. "$tmp"/ && rm -rf "$dest" && mv "$tmp" "$dest"; '
                + "fi",
                300,
            )
        )
    elif repo.get("materialization") == "git_fetch":
        remote = repo_source.get("remote")
        ref = repo_source.get("ref", "main")
        ref_env = repo_source.get("ref_env", "CYNIC_FORGEJO_BOOTSTRAP_REF")
        fallback_remote_env = repo_source.get("fallback_remote_env", "CYNIC_FORGEJO_BOOTSTRAP_REMOTE")
        if not remote:
            raise RuntimeError("git_fetch bootstrap requires repo.source.remote")
        commands.append(
            BootstrapCommand(
                "repo_materialize",
                'remote="'
                + remote
                + '"; ref_default="'
                + ref
                + '"; ref_env="'
                + ref_env
                + '"; eval "ref=\\${'
                + ref_env
                + ':-$ref_default}"; dest="'
                + repo_path
                + '"; fallback_env="'
                + fallback_remote_env
                + '"; fallback_remote="${'
                + fallback_remote_env
                + ':-}"; selected_remote="$remote"; '
                + 'if ! git ls-remote "$selected_remote" HEAD >/dev/null 2>&1; then '
                + 'if [ -n "$fallback_remote" ]; then selected_remote="$fallback_remote"; else exit 1; fi; fi; '
                + 'mkdir -p "$(dirname "$dest")" '
                + '&& if [ ! -d "$dest/.git" ]; then '
                + 'rm -rf "$dest" && git clone --origin origin --branch "$ref" --single-branch "$selected_remote" "$dest"; '
                + 'else cd "$dest" && git remote set-url origin "$selected_remote" '
                + '&& git fetch --prune origin "$ref" '
                + '&& git checkout -B "$ref" "origin/$ref" '
                + '&& git reset --hard "origin/$ref"; fi',
                300,
            )
        )
    commands.append(
        BootstrapCommand(
            "repo_materialized",
            f"test -d {repo_path}/.git "
            f"&& test -f {repo_path}/.git/HEAD "
            f"&& test -f {repo_path}/.git/config "
            f"&& test -d {repo_path}/.git/objects "
            f"&& test -f {repo_path}/AGENTS.md "
            f"&& test -f {repo_path}/infra/registry.json",
            30,
        )
    )
    commands.append(
        BootstrapCommand(
            "repo_registry_contract_present",
            "python3 - <<'PY'\n"
            "import json\n"
            "from pathlib import Path\n"
            f"registry = json.loads(Path({repo_path!r}, 'infra', 'registry.json').read_text())\n"
            "missing = []\n"
            "lifecycle_root = registry.get('lifecycle')\n"
            "if not isinstance(lifecycle_root, dict):\n"
            "    missing.append('lifecycle')\n"
            "    lifecycle_root = {}\n"
            "lifecycle = lifecycle_root.get('kernel_release')\n"
            "if not isinstance(lifecycle, dict):\n"
            "    missing.append('lifecycle.kernel_release')\n"
            "    lifecycle = {}\n"
            "build_policy = lifecycle.get('build_policy')\n"
            "if not isinstance(build_policy, dict):\n"
            "    missing.append('lifecycle.kernel_release.build_policy')\n"
            "    build_policy = {}\n"
            "bootstrap = build_policy.get('bootstrap')\n"
            "if not isinstance(bootstrap, dict):\n"
            "    missing.append('lifecycle.kernel_release.build_policy.bootstrap')\n"
            "    bootstrap = {}\n"
            "if lifecycle.get('orchestrator_organ') != 'organ-forge':\n"
            "    missing.append('lifecycle.kernel_release.orchestrator_organ=organ-forge')\n"
            "if not bootstrap.get('project_slug'):\n"
            "    missing.append('lifecycle.kernel_release.build_policy.bootstrap.project_slug')\n"
            "if not bootstrap.get('repo_materialization'):\n"
            "    missing.append('lifecycle.kernel_release.build_policy.bootstrap.repo_materialization')\n"
            "if missing:\n"
            "    raise SystemExit('remote lifecycle registry contract missing: ' + ', '.join(missing))\n"
            "PY",
            30,
        )
    )
    if host_packages:
        commands.append(
            BootstrapCommand(
                "host_packages_present",
                "dpkg-query -W " + host_packages,
                60,
            )
        )
    commands.extend(
        [
            BootstrapCommand(
                "rust_toolchain_present",
                f"{Path.home() / '.cargo/bin/rustc'} +{rust_toolchain} --version "
                f"&& {Path.home() / '.cargo/bin/cargo'} +{rust_toolchain} --version",
                60,
            ),
            BootstrapCommand(
                "rust_musl_target_present",
                f"{Path.home() / '.cargo/bin/rustup'} target list --installed "
                f"| grep -Fx {rust_target}",
                60,
            ),
        ]
    )
    for index, command in enumerate(build.get("commands", []), start=1):
        commands.append(
            BootstrapCommand(
                f"build_command_{index}",
                f"cd {repo_path} && {command}",
                1800,
            )
        )
    commands.append(
        BootstrapCommand(
            "watchdog_binary_static",
            f"cd {repo_path} "
            f"&& test -x {watchdog.get('binary')} "
            f"&& file {watchdog.get('binary')} | grep -E 'static|static-pie'",
            60,
        )
    )
    return commands


class BuildRunnerBootstrapExecutor:
    def __init__(self, transport: BootstrapTransport | None = None):
        self.transport = transport

    def execute(self, contract: dict) -> dict:
        commands = build_bootstrap_commands(contract)
        node = contract["node"]
        requests = [command.as_fleet_node_exec_request(node) for command in commands]
        if self.transport is None:
            access = contract.get("access", {})
            return {
                "status": "requires_transport",
                "transport": access.get(
                    "preferred_transport",
                    "fleet_node_exec",
                ),
                "transport_command_env": access.get(
                    "transport_command_env",
                    "CYNIC_FLEET_NODE_EXEC_COMMAND",
                ),
                "transport_command_default": access.get(
                    "transport_command_default",
                    "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                ),
                "node": node,
                "requests": requests,
            }

        results = []
        for command in commands:
            result = self.transport.execute(
                node=node,
                command=command.command,
                timeout_seconds=command.timeout_seconds,
            )
            results.append(
                {
                    "stage": command.stage,
                    "ok": bool(result.get("ok", result.get("status") == "ok")),
                    "result": result,
                }
            )
            if not results[-1]["ok"]:
                return {
                    "status": "blocked",
                    "node": node,
                    "failed_stage": command.stage,
                    "results": results,
                    "requests": requests,
                }

        return {
            "status": "ok",
            "node": node,
            "results": results,
            "requests": requests,
        }
