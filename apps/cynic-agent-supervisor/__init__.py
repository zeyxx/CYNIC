"""CYNIC agent supervisor — Hermes plugin.

Supervises agent CLIs (OpenCode, Claude Code, Codex, Gemini) as bounded
subprocesses within CYNIC's coherence pattern:

    desired state -> observe -> reconcile -> bounded action -> read-back proof

This plugin does NOT bypass any provider restriction. It invokes the agent
CLI as a real subprocess (using the agent's own authentication) and wraps
the invocation in CYNIC's claim/release coordination protocol. The proof
envelope (cynic.agent_supervision.v1) is machine-readable evidence that the
bounded action was performed, with exit code, duration, and read-back.

Hooks provided:
  - system_prompt_section (cynic_agent_supervision) — non-blocking read-only
    evidence injected into the agent session prompt.

Tools provided:
  - cynic_agent_supervise — run an agent CLI under CYNIC supervision
  - cynic_agent_claims     — query active CYNIC coordination claims
  - cynic_agent_list     — list available agent CLIs and versions
"""

from __future__ import annotations

import hashlib
import json
import logging
import os
import shutil
import subprocess
import threading
import time
from typing import Any, Mapping

from .supervisor import (
    AGENT_SPECS,
    _find_agent_binary,
    _find_cynic_repo_root,
    _get_cynic_session_context,
    _probe_agent_version,
    _run_cynic_command,
    get_claims_report,
)
from .session_lifecycle import (
    DEFAULT_CLAIM_TTL_SECONDS,
    DEFAULT_HEARTBEAT_INTERVAL_SECONDS,
    SessionLeaseLifecycle,
    agent_id_for_session,
)
from .tools import (
    CYNIC_AGENT_CLAIMS_SCHEMA,
    CYNIC_AGENT_LIST_SCHEMA,
    CYNIC_AGENT_SUPERVISE_SCHEMA,
    handle_cynic_agent_claims,
    handle_cynic_agent_list,
    handle_cynic_agent_supervise,
)

logger = logging.getLogger(__name__)

_MAX_CONTEXT_CHARS = 4_000
_CLI_TIMEOUT_SECONDS = 3
_SESSION_REPO_ROOTS: dict[str, str] = {}
_SESSION_REPO_ROOTS_LOCK = threading.Lock()


def _truncate(text: str, max_chars: int = _MAX_CONTEXT_CHARS) -> str:
    if len(text) <= max_chars:
        return text
    return text[:max_chars] + "...[truncated]"


def _cynic_supervisor_context(_session: Mapping[str, Any]) -> str:
    """Inject non-blocking CYNIC coordination context into the session prompt.

    This is read-only evidence only. It never blocks session startup, never
    performs mutations, and degrades gracefully when the cynic CLI is absent.
    """
    session_id = _session.get("session_id")
    session_agent_id = None
    session_identity = ""
    if isinstance(session_id, str) and session_id:
        session_agent_id = agent_id_for_session(session_id)
        session_identity = (
            "Session agent_id: "
            f"{session_agent_id}. Use this exact --agent-id and "
            f"--ttl-seconds {DEFAULT_CLAIM_TTL_SECONDS} when creating claims. "
            f"Heartbeats run every {DEFAULT_HEARTBEAT_INTERVAL_SECONDS} seconds "
            "while the session is active; release claims only on finalization. "
            "Without a heartbeat, the bounded claim TTL is the fallback."
        )

    cynic_bin = shutil.which("cynic")
    if not cynic_bin:
        return "CYNIC agent supervisor: cynic CLI not found in PATH (unknown).\n" + session_identity

    parts: list[str] = [session_identity] if session_identity else []

    # Session context
    try:
        result = subprocess.run(
            [cynic_bin, "agent", "session", "context", "--json"],
            capture_output=True, text=True,
            timeout=_CLI_TIMEOUT_SECONDS, check=False,
        )
        if result.returncode == 0 and result.stdout.strip():
            ctx = json.loads(result.stdout.strip())
            parts.append(
                f"builder={ctx.get('builder', 'unknown')} "
                f"git_sha={ctx.get('git_sha', 'unknown')[:12]} "
                f"cynic={ctx.get('version', 'unknown')}"
            )
        else:
            parts.append("session-context: unavailable")
    except Exception as exc:
        parts.append(f"session-context: {exc!s}")

    # Active claims
    try:
        claims_command = [cynic_bin, "agent", "status"]
        if session_agent_id:
            claims_command.extend(["--agent-id", session_agent_id])
        claims_command.append("--json")
        claims_result = subprocess.run(
            claims_command,
            capture_output=True, text=True,
            timeout=_CLI_TIMEOUT_SECONDS, check=False,
        )
        if claims_result.returncode == 0 and claims_result.stdout.strip():
            claims = json.loads(claims_result.stdout.strip())
            active = claims.get("active_claims", [])
            total = claims.get("total_claims", 0)
            parts.append(f"claims: {total} active, {len(active)} holders")
            for c in active[:3]:
                parts.append(
                    f"  - {c.get('target', '?')} [{c.get('claim_type', '?')}] "
                    f"expires_in={c.get('remaining_seconds', '?')}s"
                )
        else:
            parts.append("claims: unavailable")
    except Exception as exc:
        parts.append(f"claims: {exc!s}")

    # Agent CLI availability
    agent_lines: list[str] = []
    for name, spec in AGENT_SPECS.items():
        binary = _find_agent_binary(spec)
        if binary:
            version = _probe_agent_version(binary)
            agent_lines.append(f"  {name}: {version[:60]}")
        else:
            agent_lines.append(f"  {name}: not found")
    parts.append("agent CLIs:\n" + "\n".join(agent_lines))

    return (
        "CYNIC agent supervisor (session-scoped coordination):\n"
        + "\n".join(parts)
    )


def _run_session_agent_command(args: list[str], repo_root: str) -> dict[str, Any]:
    registry = os.path.join(repo_root, "infra", "registry.json")
    result = _run_cynic_command(
        args,
        cwd=repo_root,
        timeout=_CLI_TIMEOUT_SECONDS,
        registry=registry,
    )
    if result is None:
        raise RuntimeError(f"CYNIC command failed: {' '.join(args[:2])}")
    return result


def _run_session_coordination_command(args: list[str], repo_root: str) -> dict[str, Any]:
    result = _run_cynic_command(args, cwd=repo_root, timeout=_CLI_TIMEOUT_SECONDS)
    if result is None:
        raise RuntimeError(f"CYNIC command failed: {' '.join(args[:2])}")
    return result


def _session_claim_status(agent_id: str, repo_root: str) -> dict[str, Any]:
    report = _run_session_agent_command(
        ["agent", "status", "--agent-id", agent_id, "--json"], repo_root
    )
    claims = report.get("active_claims", [])
    if not isinstance(claims, list):
        raise RuntimeError("CYNIC claim status returned an invalid active_claims value")
    return report


def _session_repo_root(agent_id: str) -> str | None:
    with _SESSION_REPO_ROOTS_LOCK:
        repo_root = _SESSION_REPO_ROOTS.get(agent_id)
    return repo_root or _find_cynic_repo_root(os.getcwd())


def _record_session_start(agent_id: str) -> str | None:
    """Persist and read back this session's CYNIC identity/worktree binding."""
    with _SESSION_REPO_ROOTS_LOCK:
        existing_root = _SESSION_REPO_ROOTS.get(agent_id)
    if existing_root:
        return existing_root

    repo_root = _find_cynic_repo_root(os.getcwd())
    if not repo_root:
        return None

    log = _run_session_coordination_command(
        ["coordination", "read", "--kind", "session_open", "--limit", "1000", "--json"],
        repo_root,
    )
    for entry in log.get("entries", []):
        if not isinstance(entry, dict) or entry.get("agent") != agent_id:
            continue
        if entry.get("kind") != "session_open" or not isinstance(entry.get("body"), str):
            continue
        try:
            prior_body = json.loads(entry["body"])
        except json.JSONDecodeError:
            continue
        bound_root = prior_body.get("repo_root") if prior_body.get("session_id") == agent_id else None
        if isinstance(bound_root, str) and bound_root:
            with _SESSION_REPO_ROOTS_LOCK:
                _SESSION_REPO_ROOTS[agent_id] = bound_root
            return bound_root

    body = json.dumps(
        {
            "session_id": agent_id,
            "agent_id": agent_id,
            "repo_root": repo_root,
            "claim_ttl_seconds": DEFAULT_CLAIM_TTL_SECONDS,
            "heartbeat_interval_seconds": DEFAULT_HEARTBEAT_INTERVAL_SECONDS,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
    _run_session_coordination_command(
        [
            "coordination",
            "append",
            "--agent",
            agent_id,
            "--kind",
            "session_open",
            "--body",
            body,
            "--json",
        ],
        repo_root,
    )
    readback = _run_session_coordination_command(
        ["coordination", "read", "--kind", "session_open", "--limit", "1000", "--json"],
        repo_root,
    )
    if not any(
        isinstance(entry, dict)
        and entry.get("agent") == agent_id
        and entry.get("kind") == "session_open"
        and entry.get("body") == body
        for entry in readback.get("entries", [])
    ):
        raise RuntimeError("session-open identity mapping was not present in the coordination read-back")
    with _SESSION_REPO_ROOTS_LOCK:
        _SESSION_REPO_ROOTS[agent_id] = repo_root
    return repo_root


def _heartbeat_session_claims(agent_id: str) -> None:
    repo_root = _session_repo_root(agent_id)
    if not repo_root:
        return
    report = _session_claim_status(agent_id, repo_root)
    for claim in report.get("active_claims", []):
        if not isinstance(claim, dict):
            raise RuntimeError("CYNIC claim status returned an invalid claim")
        target = claim.get("target")
        lease_id = claim.get("claim_id")
        if not isinstance(target, str) or not isinstance(lease_id, str):
            raise RuntimeError("CYNIC claim status omitted target or claim_id")
        _run_session_agent_command(
            [
                "agent",
                "heartbeat",
                "--agent-id",
                agent_id,
                "--target",
                target,
                "--lease-id",
                lease_id,
                "--json",
            ],
            repo_root,
        )


def _release_session_claims(agent_id: str) -> None:
    repo_root = _session_repo_root(agent_id)
    if not repo_root:
        return

    initial_status = _session_claim_status(agent_id, repo_root)
    initial_claims = initial_status.get("active_claims", [])
    released_claim_ids: list[str] = []
    release_errors: list[str] = []
    release_reports: list[dict[str, Any]] = []

    for claim in initial_claims:
        if not isinstance(claim, dict):
            release_errors.append("invalid claim summary")
            continue
        target = claim.get("target")
        lease_id = claim.get("claim_id")
        if not isinstance(target, str) or not isinstance(lease_id, str):
            release_errors.append("claim summary omitted target or claim_id")
            continue
        result = _run_cynic_command(
            [
                "agent",
                "release",
                "--agent-id",
                agent_id,
                "--target",
                target,
                "--lease-id",
                lease_id,
                "--proof-ref",
                f"hermes-session-close:{agent_id}:{lease_id}",
                "--json",
            ],
            cwd=repo_root,
            timeout=_CLI_TIMEOUT_SECONDS,
            registry=os.path.join(repo_root, "infra", "registry.json"),
        )
        if result is None:
            release_errors.append(f"release command returned no report for {target}")
            continue
        release_reports.append(result)
        released_claim_ids.append(lease_id)

    final_status = _session_claim_status(agent_id, repo_root)
    remaining = final_status.get("active_claims", [])
    if remaining:
        raise RuntimeError(f"session close left {len(remaining)} active CYNIC claim(s)")

    evidence = {
        "agent_id": agent_id,
        "initial_status": initial_status,
        "released_claim_ids": released_claim_ids,
        "release_reports": release_reports,
        "release_errors": release_errors,
        "final_status": final_status,
    }
    proof_ref = "sha256:" + hashlib.sha256(
        json.dumps(evidence, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    body = json.dumps(
        {
            "session_id": agent_id,
            "repo_root": repo_root,
            "claims_released": True,
            "released_claim_ids": released_claim_ids,
            "release_errors": release_errors,
            "worktree_fate": "retained",
            "proof_ref": proof_ref,
            "readback": final_status,
        },
        sort_keys=True,
        separators=(",", ":"),
    )
    _run_session_coordination_command(
        [
            "coordination",
            "append",
            "--agent",
            agent_id,
            "--kind",
            "session_close",
            "--body",
            body,
            "--ref",
            proof_ref,
            "--json",
        ],
        repo_root,
    )
    receipt_log = _run_session_coordination_command(
        ["coordination", "read", "--kind", "session_close", "--limit", "1000", "--json"],
        repo_root,
    )
    entries = receipt_log.get("entries", [])
    if not any(
        isinstance(entry, dict)
        and entry.get("agent") == agent_id
        and entry.get("kind") == "session_close"
        and entry.get("body") == body
        for entry in entries
    ):
        raise RuntimeError("session-close receipt was not present in the coordination read-back")
    with _SESSION_REPO_ROOTS_LOCK:
        _SESSION_REPO_ROOTS.pop(agent_id, None)


_SESSION_LEASE_LIFECYCLE = SessionLeaseLifecycle(
    heartbeat=_heartbeat_session_claims,
    release=_release_session_claims,
    interval_seconds=DEFAULT_HEARTBEAT_INTERVAL_SECONDS,
    on_error=lambda agent_id, phase, error: logger.warning(
        "CYNIC session claim %s failed for %s: %s", phase, agent_id, error
    ),
)


def _on_session_start(**kwargs: Any) -> None:
    """Start session heartbeats and log the CYNIC context snapshot."""
    session_id = kwargs.get("session_id")
    if isinstance(session_id, str) and session_id:
        try:
            _record_session_start(agent_id_for_session(session_id))
        except Exception as exc:
            logger.warning("CYNIC session identity receipt was not recorded: %s", exc)
        _SESSION_LEASE_LIFECYCLE.start_session(session_id)
    try:
        ctx = _get_cynic_session_context()
        logger.debug("CYNIC session start — context: %s",
                     json.dumps(ctx, ensure_ascii=False)[:500])
    except Exception as exc:
        logger.debug("CYNIC session start — context unavailable: %s", exc)


def _on_pre_llm_call(**kwargs: Any) -> None:
    """Ensure a resumed session has a claim heartbeat."""
    session_id = kwargs.get("session_id")
    if isinstance(session_id, str) and session_id:
        try:
            _record_session_start(agent_id_for_session(session_id))
        except Exception as exc:
            logger.warning("CYNIC session identity receipt was not recorded: %s", exc)
        _SESSION_LEASE_LIFECYCLE.start_session(session_id)


def _on_session_finalize(**kwargs: Any) -> None:
    """Release this session's claims and verify its close receipt."""
    session_id = kwargs.get("session_id")
    if isinstance(session_id, str) and session_id:
        if not _SESSION_LEASE_LIFECYCLE.finalize_session(session_id):
            logger.warning("CYNIC session claims were not fully finalized")


def _on_session_reset(**kwargs: Any) -> None:
    """Finalize only an explicitly identified old session on reset."""
    old_session_id = kwargs.get("old_session_id")
    if isinstance(old_session_id, str) and old_session_id:
        if not _SESSION_LEASE_LIFECYCLE.reset_session(old_session_id):
            logger.warning("CYNIC claims for the previous session were not fully finalized")


def register(ctx) -> None:
    """Register the CYNIC agent supervisor plugin with Hermes.

    Called once by the Hermes plugin loader. Registers:
    - A system_prompt_section with non-blocking CYNIC context
    - Three tools: cynic_agent_supervise, cynic_agent_claims, cynic_agent_list
    - Session lifecycle hooks (heartbeat, normal close, bounded TTL fallback)
    """
    # System prompt section: inject CYNIC context as read-only evidence
    ctx.register_system_prompt_section(
        "cynic_agent_supervisor",
        _cynic_supervisor_context,
        position="after_memory",
        max_chars=_MAX_CONTEXT_CHARS,
    )

    # Tools: supervised agent CLI execution within CYNIC coherence pattern
    ctx.register_tool(
        name="cynic_agent_supervise",
        toolset="cynic_agent_supervisor",
        schema=CYNIC_AGENT_SUPERVISE_SCHEMA,
        handler=handle_cynic_agent_supervise,
        description="Supervise an agent CLI (OpenCode/Claude/Codex/Gemini) "
                    "within CYNIC coherence pattern: observe, claim, bounded "
                    "action, read-back proof.",
        emoji="🛡️",
    )

    ctx.register_tool(
        name="cynic_agent_claims",
        toolset="cynic_agent_supervisor",
        schema=CYNIC_AGENT_CLAIMS_SCHEMA,
        handler=handle_cynic_agent_claims,
        description="Query active CYNIC agent claims (claim/status coordination).",
        emoji="📊",
    )

    ctx.register_tool(
        name="cynic_agent_list",
        toolset="cynic_agent_supervisor",
        schema=CYNIC_AGENT_LIST_SCHEMA,
        handler=handle_cynic_agent_list,
        description="List available agent CLIs (OpenCode, Claude, Codex, Gemini) "
                    "with versions and availability.",
        emoji="📋",
    )

    # Heartbeats persist across turns; only finalization releases session claims.
    ctx.register_hook("on_session_start", _on_session_start)
    ctx.register_hook("pre_llm_call", _on_pre_llm_call)
    ctx.register_hook("on_session_finalize", _on_session_finalize)
    ctx.register_hook("on_session_reset", _on_session_reset)

    logger.debug(
        "cynic-agent-supervisor plugin registered: "
        "3 tools, 1 system_prompt_section, 4 hooks"
    )
