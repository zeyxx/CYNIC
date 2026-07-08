"""Bounded lifecycle intent executors for organ-forge."""

from __future__ import annotations

import hashlib
import json
import os
import pwd
import shutil
import subprocess
import time
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, Sequence

from .runtime_dna import build_runtime_dna_report
from .runtime_host_guard import build_runner_host_guard


MAX_OUTPUT_CHARS = 12000


@dataclass(frozen=True)
class LifecycleCommand:
    name: str
    command: list[str]
    timeout_seconds: int
    env: dict[str, str] | None = None


@dataclass(frozen=True)
class McpBuildPlan:
    profile: str
    command: list[str]
    expected_artifact: Path


class LifecycleIntentExecutor:
    def __init__(
        self,
        repo_path: str | Path,
        runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
        build_node: Mapping[str, Any] | None = None,
        observed_hosts: Sequence[str] | None = None,
    ):
        self.repo_path = Path(repo_path)
        self.runner = runner
        self.build_node = dict(build_node or {})
        self.observed_hosts = list(observed_hosts) if observed_hosts is not None else None

    def execute(self, task: Any) -> dict:
        host_guard = self._build_runner_host_guard(task)
        if host_guard is not None:
            return {
                "schema": "cynic.lifecycle_intent_execution.v1",
                **host_guard,
                "intent": task.content.intent if hasattr(task, "content") else None,
                "request_profile": task.content.request_profile if hasattr(task, "content") else None,
                "task_domain": getattr(task, "domain", None),
            }
        intent = task.content.intent
        if intent == "build-and-reload-mcp":
            return self._execute_mcp_reload(task, proof_only=False)
        if intent == "mcp-artifact-materialization-proof":
            return self._execute_mcp_reload(task, proof_only=True)
        if intent == "classify-release-matter-for-sovereign-forgejo-proposal":
            return self._execute_release_matter_classification(task)
        return {
            "status": "blocked",
            "error_kind": "lifecycle_intent_executor_not_implemented",
            "error": f"no organ-forge lifecycle executor is implemented for intent: {intent}",
            "intent": intent,
            "request_profile": task.content.request_profile,
            "deploy_target": task.content.deploy_target,
            "coord_targets": task.content.coord_targets,
        }

    def _build_runner_host_guard(self, task: Any) -> dict[str, Any] | None:
        return build_runner_host_guard(
            getattr(task.content, "build_target", ""),
            build_node=self.build_node,
            observed_hosts=self.observed_hosts,
            registry_build_target=getattr(task.content, "build_target", ""),
            intent=getattr(task.content, "intent", None),
            task_domain=getattr(task, "domain", None),
        )

    def _execute_release_matter_classification(self, task: Any) -> dict:
        runtime_dna = build_runtime_dna_report(self.repo_path, runner=self.runner)
        execution_context = _build_runtime_execution_context(self.repo_path)
        git_steps = [
            self._run_command(LifecycleCommand("git_status_porcelain", ["git", "status", "--porcelain"], 30)),
            self._run_command(LifecycleCommand("git_branch", ["git", "branch", "--show-current"], 30)),
            self._run_command(LifecycleCommand("git_remote", ["git", "remote", "-v"], 30)),
        ]
        status_step = git_steps[0]
        dirty_entries = _parse_git_status_porcelain(status_step.get("stdout", ""))
        groups = _classify_release_matter(dirty_entries)
        forgejo_remotes = _forgejo_remotes(git_steps[2].get("stdout", ""))
        proposal_plan = {
            "schema": "cynic.sovereign_forgejo_commit_proposal_plan.v1",
            "status": "ready_for_operator_review",
            "target_remote_policy": "forgejo_sovereign_remote_required",
            "forgejo_remotes": forgejo_remotes,
            "cortex_commit_allowed": False,
            "destructive_sync_allowed": False,
            "recommended_actor": "organ-forge-hermes-agent",
            "recommended_sequence": [
                "review grouped matter",
                "claim group targets",
                "create Forgejo-targeted branch or proposal",
                "run group-specific tests",
                "record Kernel observation with proposal refs",
                "commit/push through sovereign organ-forge identity after approval",
            ],
        }
        blocking_reason = None
        if status_step.get("status") != "ok":
            blocking_reason = "git_status_unreadable"
        elif not forgejo_remotes:
            blocking_reason = "forgejo_remote_not_detected"

        status = "blocked" if blocking_reason else "ok"
        return {
            "schema": "cynic.release_matter_classification.v1",
            "status": status,
            "intent": task.content.intent,
            "request_profile": task.content.request_profile,
            "repo_path": str(self.repo_path),
            "runtime_dna": runtime_dna,
            "execution_context": execution_context,
            "steps": git_steps,
            "dirty_count": len(dirty_entries),
            "groups": groups,
            "proposal_plan": proposal_plan,
            "proof": {
                "no_destructive_git_operation": True,
                "no_cortex_commit": True,
                "classification_only": True,
                "task_domain": getattr(task, "domain", ""),
                "task_id": getattr(task, "id", ""),
            },
            "read_back": {
                "kind": "completed_agent_task_result_required",
                "status": "pending_kernel_result_submission",
            },
            **({"error_kind": blocking_reason} if blocking_reason else {}),
        }

    def _execute_mcp_reload(self, task: Any, *, proof_only: bool) -> dict:
        runtime_dna = build_runtime_dna_report(self.repo_path, runner=self.runner)
        execution_context = _build_runtime_execution_context(self.repo_path)
        guard = _mcp_reload_runtime_guard(runtime_dna)
        if guard is not None:
            return {
                "schema": "cynic.lifecycle_intent_execution.v1",
                "status": "blocked",
                "intent": task.content.intent,
                "request_profile": task.content.request_profile,
                "repo_path": str(self.repo_path),
                "runtime_dna": runtime_dna,
                "execution_context": execution_context,
                "build_triggered": False,
                "steps": [],
                "artifact": {"status": "missing", "path": ""},
                "artifacts": [],
                "read_back": {
                    "kind": "mcp_freshness",
                    "status": "blocked",
                    "payload": None,
                },
                **guard,
            }
        build_env = {
            "CYNIC_PROJECT_ROOT": str(self.repo_path),
            "CARGO_TARGET_DIR": str(self.repo_path / "target"),
        }
        pre_check = LifecycleCommand(
            "pre_freshness_check",
            ["python3", ".cortex/mcp/run_cynic_mcp.py"],
            30,
            {"CYNIC_MCP_CHECK_ONLY": "1"},
        )
        post_check = LifecycleCommand(
            "post_freshness_check",
            ["python3", ".cortex/mcp/run_cynic_mcp.py"],
            30,
            {"CYNIC_MCP_CHECK_ONLY": "1"},
        )

        steps = [self._run_command(pre_check)]
        pre_payload = _json_payload(steps[0].get("stdout", ""))
        pre_artifact = _build_binary_proof(pre_payload)
        build_plan = _mcp_build_plan(self.repo_path, pre_payload)
        pre_read_back_status = (
            pre_payload.get("status") if isinstance(pre_payload, dict) else "invalid_json"
        )
        build_triggered = not proof_only
        if build_triggered:
            steps.append(
                self._run_command(
                    LifecycleCommand(
                        "build_cynic_mcp",
                        build_plan.command,
                        int(os.environ.get("CYNIC_FORGE_MCP_BUILD_TIMEOUT_SECONDS", "900")),
                        build_env,
                    )
                )
            )
            steps.append(self._run_command(post_check))

        failed = next((step for step in steps if step["status"] != "ok"), None)
        read_back_payload = (
            _json_payload(steps[-1].get("stdout", "")) if build_triggered else pre_payload
        )
        artifact = _build_binary_proof(read_back_payload)
        read_back_status = (
            read_back_payload.get("status")
            if isinstance(read_back_payload, dict)
            else "invalid_json"
        )
        expected_artifact = build_plan.expected_artifact
        projection_guard = _mcp_binary_projection_guard(artifact, expected_artifact)
        if failed:
            status = "failed"
            error_kind = "lifecycle_intent_command_failed"
        elif read_back_status != "fresh":
            status = "failed"
            error_kind = "mcp_freshness_readback_failed"
        elif artifact.get("status") != "ok":
            status = "failed"
            error_kind = "mcp_binary_proof_failed"
        elif projection_guard is not None:
            status = "failed"
            error_kind = "mcp_binary_projection_mismatch"
        else:
            status = "ok"
            error_kind = None

        result = {
            "schema": "cynic.lifecycle_intent_execution.v1",
            "status": status,
            "intent": task.content.intent,
            "request_profile": task.content.request_profile,
            "repo_path": str(self.repo_path),
            "runtime_dna": runtime_dna,
            "execution_context": execution_context,
            "build_triggered": build_triggered,
            "steps": steps,
            "artifact": artifact,
            "artifacts": [artifact["path"]] if artifact.get("path") else [],
            "build_profile": build_plan.profile,
            "read_back": {
                "kind": "mcp_freshness",
                "status": read_back_status,
                "payload": read_back_payload if isinstance(read_back_payload, dict) else None,
            },
            "expected_artifact_path": str(expected_artifact),
        }
        if projection_guard is not None:
            result.update(projection_guard)
        if error_kind:
            result["error_kind"] = error_kind
        return result

    def _run_command(self, command: LifecycleCommand) -> dict:
        started = time.monotonic()
        env = os.environ.copy()
        env.update(command.env or {})
        env["PYTHONDONTWRITEBYTECODE"] = "1"
        cwd = str(self.repo_path)
        try:
            completed = self.runner(
                command.command,
                cwd=cwd,
                env=env,
                capture_output=True,
                text=True,
                timeout=command.timeout_seconds,
                check=False,
            )
            return _completed_step(command, started, completed)
        except subprocess.TimeoutExpired as exc:
            duration_ms = int((time.monotonic() - started) * 1000)
            return {
                "name": command.name,
                "status": "timeout",
                "command": command.command,
                "timeout_seconds": command.timeout_seconds,
                "duration_ms": duration_ms,
                "returncode": None,
                "stdout": _clip(exc.stdout or ""),
                "stderr": _clip(exc.stderr or ""),
            }
        except OSError as exc:
            retried = _retry_with_resolved_binary(command, cwd, env, self.runner)
            if retried is not None:
                completed, resolved_command = retried
                step = _completed_step(command, started, completed)
                step["command"] = resolved_command
                step["resolved_binary_retry"] = True
                return step
            duration_ms = int((time.monotonic() - started) * 1000)
            return {
                "name": command.name,
                "status": "failed",
                "command": command.command,
                "timeout_seconds": command.timeout_seconds,
                "duration_ms": duration_ms,
                "returncode": None,
                "stdout": "",
                "stderr": _clip(str(exc)),
            }


def _mcp_reload_runtime_guard(runtime_dna: dict[str, Any]) -> dict[str, Any] | None:
    if runtime_dna.get("status") != "ok":
        return {
            "error_kind": "runtime_dna_not_clean_for_mcp_reload",
            "error": (
                "organ-forge refused MCP reload because the runtime checkout is not a clean "
                "fast-forwardable bootstrap source"
            ),
        }

    expected_ref = os.environ.get("CYNIC_FORGEJO_BOOTSTRAP_REF", "").strip()
    if not expected_ref:
        return None

    observed_branch = runtime_dna.get("branch", "")
    observed_upstream = runtime_dna.get("upstream", "")
    expected_upstream = f"origin/{expected_ref}"
    if observed_branch == expected_ref and observed_upstream in ("", expected_upstream):
        return None

    return {
        "error_kind": "runtime_bootstrap_ref_mismatch",
        "error": (
            "organ-forge refused MCP reload because the runtime checkout does not match the "
            "declared bootstrap ref"
        ),
        "expected_bootstrap_ref": expected_ref,
        "observed_branch": observed_branch,
        "observed_upstream": observed_upstream,
    }


def _mcp_build_plan(repo_path: Path, payload: dict | None) -> McpBuildPlan:
    release_artifact = repo_path / "target" / "release" / "cynic-mcp"
    debug_artifact = repo_path / "target" / "debug" / "cynic-mcp"
    if not isinstance(payload, dict):
        return McpBuildPlan(
            profile="release",
            command=["cargo", "build", "-p", "cynic-mcp", "--release"],
            expected_artifact=release_artifact,
        )

    canonical_binary = payload.get("canonical_binary")
    repair_local_debug_binary = payload.get("repair_local_debug_binary") is True
    if isinstance(canonical_binary, str) and canonical_binary:
        observed = Path(canonical_binary).expanduser()
        try:
            observed_resolved = observed.resolve(strict=False)
        except OSError:
            observed_resolved = observed
        if repair_local_debug_binary and observed_resolved == debug_artifact.resolve():
            return McpBuildPlan(
                profile="debug_repair",
                command=["cargo", "build", "-p", "cynic-mcp"],
                expected_artifact=debug_artifact,
            )

    return McpBuildPlan(
        profile="release",
        command=["cargo", "build", "-p", "cynic-mcp", "--release"],
        expected_artifact=release_artifact,
    )


def _mcp_binary_projection_guard(
    artifact: dict[str, Any],
    expected_artifact: Path,
) -> dict[str, Any] | None:
    observed_path = artifact.get("path")
    if not isinstance(observed_path, str) or not observed_path:
        return None
    observed = Path(observed_path).expanduser().resolve()
    expected = expected_artifact.expanduser().resolve()
    if observed == expected:
        return None
    return {
        "error": (
            "organ-forge read back a fresh MCP binary outside the canonical runtime mirror "
            "artifact path"
        ),
        "observed_artifact_path": str(observed),
    }


def _clip(value: str) -> str:
    if len(value) <= MAX_OUTPUT_CHARS:
        return value
    return value[-MAX_OUTPUT_CHARS:]


def _completed_step(
    command: LifecycleCommand,
    started: float,
    completed: subprocess.CompletedProcess,
) -> dict:
    duration_ms = int((time.monotonic() - started) * 1000)
    step_status = "ok" if completed.returncode == 0 else "failed"
    structured_payload = _json_payload(completed.stdout)
    if (
        completed.returncode == 78
        and command.env
        and command.env.get("CYNIC_MCP_CHECK_ONLY") == "1"
        and isinstance(structured_payload, dict)
        and structured_payload.get("schema") == "cynic.mcp_launcher_check.v1"
    ):
        step_status = "ok"
    return {
        "name": command.name,
        "status": step_status,
        "command": command.command,
        "timeout_seconds": command.timeout_seconds,
        "duration_ms": duration_ms,
        "returncode": completed.returncode,
        "stdout": _clip(completed.stdout),
        "stderr": _clip(completed.stderr),
        "structured_payload": structured_payload if step_status == "ok" else None,
    }


def _json_payload(value: str) -> dict | None:
    try:
        payload = json.loads(value.strip() or "{}")
    except json.JSONDecodeError:
        return None
    return payload if isinstance(payload, dict) else None


def _build_binary_proof(payload: dict | None) -> dict:
    if not isinstance(payload, dict):
        return {
            "schema": "cynic.binary_artifact_proof.v1",
            "status": "invalid_read_back",
            "path": None,
            "reason": "post-build MCP read-back did not return JSON",
        }

    binary = payload.get("canonical_binary")
    if not isinstance(binary, str) or not binary.strip():
        return {
            "schema": "cynic.binary_artifact_proof.v1",
            "status": "missing_path",
            "path": None,
            "reason": "post-build MCP read-back did not expose canonical_binary",
            "read_back_status": payload.get("status"),
        }

    path = Path(binary).expanduser()
    exists = path.exists()
    executable = exists and os.access(path, os.X_OK)
    proof = {
        "schema": "cynic.binary_artifact_proof.v1",
        "status": "ok" if exists and executable else "missing",
        "path": str(path),
        "read_back_status": payload.get("status"),
        "exists": exists,
        "executable": executable,
        "reported_fallback": payload.get("fallback"),
    }
    if not exists:
        proof["reason"] = "canonical_binary is not present on the runner filesystem"
        return proof
    if not executable:
        proof["status"] = "not_executable"
        proof["reason"] = "canonical_binary exists but is not executable"
        return proof

    stat_result = path.stat()
    proof["size_bytes"] = stat_result.st_size
    proof["mtime_epoch_ms"] = int(stat_result.st_mtime * 1000)
    proof["sha256"] = _sha256_file(path)
    return proof


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _retry_with_resolved_binary(
    command: LifecycleCommand,
    cwd: str,
    env: dict[str, str],
    runner: Callable[..., subprocess.CompletedProcess],
) -> tuple[subprocess.CompletedProcess, list[str]] | None:
    if not command.command:
        return None
    binary = command.command[0]
    if binary != "cargo":
        return None
    resolved = _resolve_cargo_binary(env)
    if resolved is None:
        return None
    retry_command = [resolved, *command.command[1:]]
    completed = runner(
        retry_command,
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
        timeout=command.timeout_seconds,
        check=False,
    )
    return completed, retry_command


def _resolve_cargo_binary(env: dict[str, str]) -> str | None:
    path_bin = shutil.which("cargo", path=env.get("PATH"))
    if path_bin:
        return path_bin
    home = Path(env.get("HOME", str(Path.home()))).expanduser()
    for candidate in (
        home / ".cargo" / "bin" / "cargo",
        Path(Path.home() / ".cargo/bin/cargo"),
    ):
        if candidate.exists() and os.access(candidate, os.X_OK):
            return str(candidate)
    return None


def _build_runtime_execution_context(repo_path: Path) -> dict:
    resolved_repo = repo_path.resolve()
    home = Path.home().resolve()
    try:
        username = pwd.getpwuid(os.getuid()).pw_name
    except KeyError:
        username = str(os.getuid())
    return {
        "schema": "cynic.runtime_execution_context.v1",
        "repo_path": str(resolved_repo),
        "repo_exists": resolved_repo.exists(),
        "cwd": str(Path.cwd().resolve()),
        "cwd_exists": Path.cwd().exists(),
        "home": str(home),
        "home_exists": home.exists(),
        "user": username,
        "uid": os.getuid(),
        "hostname": os.uname().nodename,
        "python": os.environ.get("PYTHON", "python3"),
    }


def _parse_git_status_porcelain(output: str) -> list[dict[str, str]]:
    entries: list[dict[str, str]] = []
    for line in output.splitlines():
        if not line:
            continue
        status = line[:2]
        path = line[3:] if len(line) > 3 else ""
        if " -> " in path:
            old_path, new_path = path.split(" -> ", 1)
        else:
            old_path, new_path = "", path
        entries.append(
            {
                "status": status.strip() or "unknown",
                "path": new_path,
                "old_path": old_path,
                "raw": line,
            }
        )
    return entries


def _classify_release_matter(entries: list[dict[str, str]]) -> list[dict[str, Any]]:
    groups = [
        ("mcp_runtime", _path_prefixes(".cortex/mcp/", "crates/cynic-mcp/", ".codex/config.toml")),
        ("organ_forge_lifecycle", _path_prefixes("organs/forge/")),
        ("command_surfaces", _path_prefixes("Makefile", "make/surfaces/", "axioms/command-surfaces-v1.json")),
        ("organ_memory", _path_prefixes("axioms/organ-memory-v1.json", "organs/forge/src/core/organ_memory.py")),
        ("repo_topology_axioms", _path_prefixes("axioms/", "organs/supervisor/")),
        ("runtime_infra", _path_prefixes("infra/", "organs/lifecycle/", "organs/fleet/", "organs/keep/")),
    ]
    buckets: dict[str, list[dict[str, str]]] = {name: [] for name, _ in groups}
    buckets["application_product_drift"] = []
    for entry in entries:
        path = entry["path"]
        matched = False
        for name, predicate in groups:
            if predicate(path):
                buckets[name].append(entry)
                matched = True
                break
        if not matched:
            buckets["application_product_drift"].append(entry)
    return [
        {
            "name": name,
            "count": len(items),
            "sample": items[:40],
            "risk": _group_risk(name),
            "recommended_owner": _group_owner(name),
            "recommended_tests": _group_tests(name),
        }
        for name, items in buckets.items()
        if items
    ]


def _path_prefixes(*prefixes: str) -> Callable[[str], bool]:
    return lambda path: any(path == prefix or path.startswith(prefix) for prefix in prefixes)


def _group_risk(name: str) -> str:
    return {
        "mcp_runtime": "high",
        "organ_forge_lifecycle": "high",
        "command_surfaces": "medium",
        "organ_memory": "medium",
        "repo_topology_axioms": "medium",
        "runtime_infra": "high",
        "application_product_drift": "unknown",
    }.get(name, "unknown")


def _group_owner(name: str) -> str:
    return {
        "mcp_runtime": "organ-forge",
        "organ_forge_lifecycle": "organ-forge",
        "command_surfaces": "organ-supervisor",
        "organ_memory": "organ-forge",
        "repo_topology_axioms": "organ-supervisor",
        "runtime_infra": "organ-lifecycle",
        "application_product_drift": "domain owner review",
    }.get(name, "domain owner review")


def _group_tests(name: str) -> list[str]:
    common = ["make lint-repo-topology-contract"]
    return {
        "mcp_runtime": ["CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py", "make lint-agent-exposure-contract"],
        "organ_forge_lifecycle": ["python3 -m unittest organs.forge.src.tests.test_lifecycle_agent organs.forge.src.tests.test_lifecycle_intent_executor"],
        "command_surfaces": ["make lint-command-surfaces-contract"],
        "organ_memory": ["make lint-organ-memory-contract", "python3 -m unittest organs.forge.src.tests.test_organ_memory"],
        "repo_topology_axioms": common,
        "runtime_infra": ["python3 organs/forge/src/entrypoints/cli.py lifecycle-status"],
        "application_product_drift": ["owner-specific tests required before proposal"],
    }.get(name, common)


def _forgejo_remotes(output: str) -> list[dict[str, str]]:
    remotes: list[dict[str, str]] = []
    for line in output.splitlines():
        parts = line.split()
        if len(parts) < 2:
            continue
        name, url = parts[0], parts[1]
        if "forgejo" in url.lower() or "10.20.0.10" in url or "10.10.10.101" in url:
            remotes.append({"name": name, "url": url})
    return remotes
