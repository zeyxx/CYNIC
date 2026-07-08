"""Bootstrap source diagnostics for organ-forge."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from urllib.parse import urlsplit, urlunsplit

from ..adapters.registry_adapter import load_registry, required_lifecycle, required_node
from .lifecycle_bootstrap import build_runner_bootstrap_contract
from .project_root import resolve_project_root_from_script


def _redact_remote(remote: str) -> str:
    try:
        parsed = urlsplit(remote)
    except Exception:
        return remote
    if not parsed.scheme or "@" not in parsed.netloc:
        return remote
    hostname = parsed.hostname or parsed.netloc.split("@", 1)[-1]
    netloc = hostname
    if parsed.port:
        netloc = f"{hostname}:{parsed.port}"
    return urlunsplit((parsed.scheme, netloc, parsed.path, parsed.query, parsed.fragment))


def _git(
    *args: str,
    env: dict[str, str] | None = None,
    cwd: str | Path | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=str(cwd) if cwd is not None else None,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def _contract_probe(registry_path: Path) -> dict:
    registry = json.loads(registry_path.read_text())
    missing: list[str] = []
    lifecycle_root = registry.get("lifecycle")
    if not isinstance(lifecycle_root, dict):
        missing.append("lifecycle")
        lifecycle_root = {}
    lifecycle = lifecycle_root.get("kernel_release")
    if not isinstance(lifecycle, dict):
        missing.append("lifecycle.kernel_release")
        lifecycle = {}
    build_policy = lifecycle.get("build_policy")
    if not isinstance(build_policy, dict):
        missing.append("lifecycle.kernel_release.build_policy")
        build_policy = {}
    bootstrap = build_policy.get("bootstrap")
    if not isinstance(bootstrap, dict):
        missing.append("lifecycle.kernel_release.build_policy.bootstrap")
        bootstrap = {}
    if lifecycle.get("orchestrator_organ") != "organ-forge":
        missing.append("lifecycle.kernel_release.orchestrator_organ=organ-forge")
    if not bootstrap.get("project_slug"):
        missing.append("lifecycle.kernel_release.build_policy.bootstrap.project_slug")
    if not bootstrap.get("repo_materialization"):
        missing.append("lifecycle.kernel_release.build_policy.bootstrap.repo_materialization")
    return {
        "ok": not missing,
        "missing": missing,
    }


def _suggest_local_forgejo_remote() -> dict | None:
    repo_root = resolve_project_root_from_script(__file__)
    remotes = _git("remote", "-v", cwd=repo_root)
    if remotes.returncode != 0:
        return None
    candidates: list[tuple[str, str]] = []
    for line in remotes.stdout.splitlines():
        parts = line.strip().split()
        if len(parts) < 2:
            continue
        name, remote = parts[0], parts[1]
        lowered = remote.lower()
        if "forgejo" in lowered or "localhost:18300" in lowered or "10.20.0.10" in lowered or "10.10.10.101" in lowered:
            candidates.append((name, remote))
    if not candidates:
        return None
    name, remote = candidates[0]
    return {
        "source": f"git remote {name}",
        "remote": remote,
        "remote_redacted": _redact_remote(remote),
        "command": f"git remote get-url {name}",
    }


def probe_bootstrap_source(
    *,
    registry: dict | None = None,
    env: dict[str, str] | None = None,
) -> dict:
    registry = registry or load_registry()
    lifecycle = required_lifecycle(registry, "kernel_release")
    node = required_node(registry, lifecycle["build_runner_role"])
    contract = build_runner_bootstrap_contract(lifecycle, node)
    repo = contract["repo"]
    source = repo.get("source", {})
    base_env = dict(os.environ)
    if env:
        base_env.update(env)

    remote = source.get("remote")
    ref_env_name = source.get("ref_env", "CYNIC_FORGEJO_BOOTSTRAP_REF")
    fallback_remote_env_name = source.get(
        "fallback_remote_env",
        "CYNIC_FORGEJO_BOOTSTRAP_REMOTE",
    )
    env_contract = {
        "ref_env": ref_env_name,
        "ref_present": bool(base_env.get(ref_env_name)),
        "fallback_remote_env": fallback_remote_env_name,
        "fallback_remote_present": bool(base_env.get(fallback_remote_env_name)),
    }
    if not remote:
        return {
            "status": "ok",
            "reason": "bootstrap_source_not_required",
            "remote_attempts": [],
            "selected_ref": None,
            "env_contract": env_contract,
        }
    ref_default = source.get("ref", "main")
    ref = base_env.get(ref_env_name, ref_default)
    fallback_remote = base_env.get(fallback_remote_env_name, "")

    attempts = []
    selected_remote = None
    for candidate, label in ((remote, "primary"), (fallback_remote, "fallback")):
        if not candidate:
            continue
        result = _git("ls-remote", candidate, "HEAD", env=base_env)
        attempts.append(
            {
                "kind": label,
                "remote": _redact_remote(candidate),
                "ok": result.returncode == 0,
                "returncode": result.returncode,
                "stderr": result.stderr.strip(),
            }
        )
        if result.returncode == 0:
            selected_remote = candidate
            break

    if selected_remote is None:
        reason = "bootstrap_source_unreachable"
        operator_action = None
        fallback_remote_suggestion = _suggest_local_forgejo_remote()
        if attempts and not fallback_remote and fallback_remote_env_name:
            reason = "bootstrap_source_fallback_env_missing"
            if fallback_remote_suggestion:
                operator_action = (
                    "set "
                    + fallback_remote_env_name
                    + " from `"
                    + fallback_remote_suggestion["command"]
                    + "`, regenerate ~/.config/cynic/env, then restart organ-forge-hermes-agent.service"
                )
            else:
                operator_action = (
                    "set "
                    + fallback_remote_env_name
                    + " in ~/.cynic-env, regenerate ~/.config/cynic/env, then restart organ-forge-hermes-agent.service"
                )
        return {
            "status": "blocked",
            "reason": reason,
            "remote_attempts": attempts,
            "selected_ref": ref,
            "env_contract": env_contract,
            "runtime_env_files": ["~/.cynic-env", "~/.config/cynic/env"],
            "operator_action": operator_action,
            "fallback_remote_suggestion": (
                {
                    "source": fallback_remote_suggestion["source"],
                    "remote_redacted": fallback_remote_suggestion["remote_redacted"],
                    "command": fallback_remote_suggestion["command"],
                }
                if fallback_remote_suggestion
                else None
            ),
        }

    tmpdir = Path(tempfile.mkdtemp(prefix="forge-bootstrap-source-"))
    dest = tmpdir / repo.get("project_slug", "CYNIC")
    try:
        clone = _git(
            "clone",
            "--origin",
            "origin",
            "--branch",
            ref,
            "--single-branch",
            selected_remote,
            str(dest),
            env=base_env,
        )
        if clone.returncode != 0:
            return {
                "status": "blocked",
                "reason": "bootstrap_clone_failed",
                "remote_attempts": attempts,
                "selected_remote": _redact_remote(selected_remote),
                "selected_ref": ref,
                "env_contract": env_contract,
                "clone": {
                    "returncode": clone.returncode,
                    "stderr": clone.stderr.strip(),
                },
            }

        registry_path = dest / "infra" / "registry.json"
        sentinels = {
            "git_dir": (dest / ".git").exists(),
            "git_head": (dest / ".git" / "HEAD").exists(),
            "git_config": (dest / ".git" / "config").exists(),
            "git_objects": (dest / ".git" / "objects").exists(),
            "agents_md": (dest / "AGENTS.md").exists(),
            "registry_json": registry_path.exists(),
        }
        contract_probe = (
            _contract_probe(registry_path)
            if registry_path.exists()
            else {"ok": False, "missing": ["infra/registry.json"]}
        )
        head = _git("rev-parse", "HEAD", env=base_env, cwd=dest)
        branch = _git("rev-parse", "--abbrev-ref", "HEAD", env=base_env, cwd=dest)
        status = "ok" if all(sentinels.values()) and contract_probe["ok"] else "blocked"
        reason = "bootstrap_source_ready" if status == "ok" else "bootstrap_source_contract_stale"
        return {
            "status": status,
            "reason": reason,
            "remote_attempts": attempts,
            "selected_remote": _redact_remote(selected_remote),
            "selected_ref": ref,
            "env_contract": env_contract,
            "checkout": {
                "head": head.stdout.strip() if head.returncode == 0 else "",
                "branch": branch.stdout.strip() if branch.returncode == 0 else "",
                "sentinels": sentinels,
                "registry_contract": contract_probe,
            },
        }
    finally:
        shutil.rmtree(tmpdir, ignore_errors=True)
