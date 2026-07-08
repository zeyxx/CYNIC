"""Runtime DNA checkout diagnostics for organ-forge."""

from __future__ import annotations

import os
import subprocess
from collections.abc import Callable
from pathlib import Path


def build_runtime_dna_report(
    repo_path: str | Path,
    *,
    runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
) -> dict:
    repo = Path(repo_path)
    repo_probe = _probe_repo_layout(repo)
    commands = {
        "head": ["git", "rev-parse", "HEAD"],
        "branch": ["git", "branch", "--show-current"],
        "upstream": ["git", "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
        "remote": ["git", "remote", "-v"],
        "porcelain": ["git", "status", "--porcelain"],
    }
    results = {name: _run_git(repo, command, runner) for name, command in commands.items()}
    if any(result["returncode"] not in (0, None) for result in results.values()):
        status = "blocked"
        summary = _blocked_summary(repo_probe, results)
    else:
        dirty_lines = [
            line for line in results["porcelain"]["stdout"].splitlines() if line.strip()
        ]
        status = "ok" if not dirty_lines else "degraded"
        summary = (
            "runtime checkout is clean"
            if status == "ok"
            else "runtime checkout has uncommitted drift"
        )
    sync_policy = _sync_policy(status)
    return {
        "schema": "cynic.runtime_dna_report.v1",
        "status": status,
        "summary": summary,
        "repo_path": str(repo),
        "head": _single_line(results["head"]["stdout"]),
        "branch": _single_line(results["branch"]["stdout"]),
        "upstream": _single_line(results["upstream"]["stdout"]),
        "dirty_count": len(
            [line for line in results["porcelain"]["stdout"].splitlines() if line.strip()]
        ),
        "dirty_sample": results["porcelain"]["stdout"].splitlines()[:20],
        "remotes": results["remote"]["stdout"].splitlines(),
        "repo_probe": repo_probe,
        "sync_policy": sync_policy | {"blocked_reason": repo_probe.get("blocked_reason")},
        "git": results,
    }


def _run_git(
    repo: Path,
    command: list[str],
    runner: Callable[..., subprocess.CompletedProcess],
) -> dict:
    try:
        completed = runner(
            command,
            cwd=str(repo),
            capture_output=True,
            text=True,
            timeout=20,
            check=False,
        )
        return {
            "command": command,
            "returncode": completed.returncode,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
        }
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {
            "command": command,
            "returncode": None,
            "stdout": getattr(exc, "stdout", "") or "",
            "stderr": str(exc),
        }


def _single_line(value: str) -> str:
    return value.splitlines()[0].strip() if value.splitlines() else ""


def _probe_repo_layout(repo: Path) -> dict:
    git_dir = repo / ".git"
    probe = {
        "repo_exists": repo.exists(),
        "git_dir": str(git_dir),
        "git_dir_exists": git_dir.exists(),
        "git_head_exists": (git_dir / "HEAD").exists(),
        "git_config_exists": (git_dir / "config").exists(),
        "git_objects_exists": (git_dir / "objects").exists(),
        "git_refs_exists": (git_dir / "refs").exists(),
    }
    if not probe["repo_exists"]:
        probe["blocked_reason"] = "repo_path_missing"
    elif not probe["git_dir_exists"]:
        probe["blocked_reason"] = "git_dir_missing"
    elif not probe["git_objects_exists"]:
        probe["blocked_reason"] = "git_objects_missing"
    elif not probe["git_head_exists"]:
        probe["blocked_reason"] = "git_head_missing"
    else:
        probe["blocked_reason"] = None
    return probe


def _blocked_summary(repo_probe: dict, git_results: dict[str, dict]) -> str:
    reason = repo_probe.get("blocked_reason")
    if reason == "git_objects_missing":
        return "runtime checkout has incomplete .git materialization"
    if reason == "git_dir_missing":
        return "runtime checkout is missing .git metadata"
    if reason == "repo_path_missing":
        return "runtime checkout path is missing"
    stderr = " ".join(
        result.get("stderr", "").strip()
        for result in git_results.values()
        if result.get("stderr")
    )
    if "not a git repository" in stderr:
        return "runtime checkout git metadata is unreadable from current execution context"
    return "runtime checkout git metadata is not readable"


def _sync_policy(status: str) -> dict:
    transition_plan = [
        "diagnose_runtime_dna",
        "classify_dirty_runtime",
        "archive_or_promote_local_matter",
        "select_canonical_sync_source",
        "fast_forward_or_block",
        "build",
        "read_back",
    ]
    return {
        "destructive_reset_allowed": False,
        "allowed_sync_modes": ["fast_forward_only", "claimed_patch_artifact"],
        "canonical_sync_sources": [
            "registry-selected repo remote/ref",
            "signed lifecycle task payload",
            "coord-claimed cortex patch artifact",
        ],
        "required_preflight_order": transition_plan,
        "next_action": (
            "classify and archive dirty runtime matter before lifecycle execution"
            if status == "degraded"
            else (
                "restore readable git metadata before lifecycle execution"
                if status == "blocked"
                else "runtime checkout can enter fast-forward-only sync"
            )
        ),
    }
