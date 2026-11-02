"""Tool schemas and handlers for the cynic-agent-supervisor plugin.

Provides three tools:
  - cynic_agent_supervise  — run an agent CLI under CYNIC supervision
  - cynic_agent_claims     — query active CYNIC coordination claims
  - cynic_agent_list     — list available agent CLIs with versions

All evidence is machine-readable JSON. The supervisor follows CYNIC's
coherence pattern: observe -> claim -> bounded action -> read-back proof.
"""

from __future__ import annotations

import json
import os
from typing import Any, Dict

from .supervisor import (
    AGENT_SPECS,
    Observation,
    ClaimState,
    ExecutionResult,
    ReleaseProof,
    SupervisionProof,
    build_proof,
    execute_agent_cli,
    get_claims_report,
    observe,
    proof_to_json,
    reconcile_claim,
    release_claim,
)
from .session_lifecycle import agent_id_for_session

# ── JSON Schema helpers ───────────────────────────────────────────────────


def _str(description: str, **extra: Any) -> Dict[str, Any]:
    schema: Dict[str, Any] = {"type": "string", "description": description}
    schema.update(extra)
    return schema


def _schema(name: str, description: str, properties: Dict[str, Any],
            required: list[str] | None = None) -> Dict[str, Any]:
    params: Dict[str, Any] = {"type": "object", "properties": properties}
    if required:
        params["required"] = required
    params["additionalProperties"] = False
    return {"name": name, "description": description, "parameters": params}


# ── Tool: cynic_agent_supervise ────────────────────────────────────────────

CYNIC_AGENT_SUPERVISE_SCHEMA = _schema(
    "cynic_agent_supervise",
    "Supervise an agent CLI (OpenCode, Claude Code, Codex, Gemini) execution "
    "within CYNIC coherence pattern: observe -> claim -> bounded action -> "
    "read-back proof. The agent CLI is invoked as a real subprocess with a "
    "hard timeout; CYNIC scope coordination is applied via the cynic CLI "
    "(claim/release). Returns a machine-readable JSON proof envelope "
    "(cynic.agent_supervision.v1).",
    {
        "agent": {
            "type": "string",
            "enum": ["opencode", "claude", "codex", "gemini"],
            "description": (
                "The agent CLI to supervise. "
                "opencode=OpenCode, claude=Claude Code, codex=Codex, gemini=Gemini CLI."
            ),
        },
        "prompt": _str(
            "The prompt/instruction to send to the agent CLI. Keep it bounded "
            "(max 16K chars). The prompt is passed as a single argument to the agent CLI."
        ),
        "working_dir": _str(
            "The working directory for the agent CLI. Must be an existing directory. "
            "CYNIC will attempt to claim this scope before execution."
        ),
        "timeout_seconds": {
            "type": "integer",
            "description": "Hard timeout for the agent CLI subprocess. Default 300, max 3600.",
            "default": 300,
            "minimum": 10,
            "maximum": 3600,
        },
        "model": _str(
            "Optional model to pass to the agent CLI (e.g. openrouter/anthropic/claude-sonnet-4). "
            "Omit to use the agent CLI's default."
        ),
        "auto_approve": {
            "type": "boolean",
            "description": "Auto-approve tool permissions in the agent CLI (dangerous). "
                           "Equivalent to --dangerously-skip-permissions (claude) or "
                           "--dangerously-bypass-approvals-and-sandbox (codex) or "
                           "--approval-mode=yolo (gemini). Default: false.",
            "default": False,
        },
        "json_output": {
            "type": "boolean",
            "description": "Request JSON-structured output from the agent CLI. "
                           "Default: true. Parsed JSON is included in the proof.",
            "default": True,
        },
        "claim_scope": _str(
            "Optional override for the CYNIC claim target. Defaults to working_dir. "
            "Use a file path for file-scope or a directory for module-scope claims."
        ),
    },
    required=["agent", "prompt", "working_dir"],
)


def handle_cynic_agent_supervise(
    args: Dict[str, Any],
    session_id: str | None = None,
    **_kw,
) -> str:
    """Run an agent CLI under CYNIC supervision."""
    agent = (args.get("agent") or "").strip()
    prompt = (args.get("prompt") or "").strip()
    working_dir = (args.get("working_dir") or "").strip()
    timeout = int(args.get("timeout_seconds", 300))
    model = args.get("model")
    auto_approve = bool(args.get("auto_approve", False))
    json_output = bool(args.get("json_output", True))
    claim_scope = args.get("claim_scope")
    session_agent_id = (
        agent_id_for_session(session_id)
        if isinstance(session_id, str) and session_id.strip()
        else None
    )

    # ── Validate inputs ───────────────────────────────────────────────
    if agent not in AGENT_SPECS:
        return _err(f"unknown agent '{agent}'. Available: {list(AGENT_SPECS.keys())}")
    if not prompt:
        return _err("prompt is required")
    if not working_dir:
        return _err("working_dir is required")
    if not os.path.isdir(working_dir):
        return _err(f"working_dir does not exist or is not a directory: {working_dir}")

    try:
        # Phase 1: Observe
        obs = observe(agent, prompt, working_dir, model, timeout, auto_approve, json_output)

        if not obs.cli.available:
            proof = build_proof(
                obs,
                ClaimState(claimed=False, lease_id="", claim_type="",
                           ttl_seconds=0, already_claimed_by_self=False,
                           error="agent CLI not available"),
                ExecutionResult(
                    argv=[], exit_code=-1, duration_seconds=0.0,
                    stdout_chars=0, stderr_chars=0, stdout_preview="",
                    stderr_preview="", timed_out=False, output_parseable=False,
                ),
                ReleaseProof(status="skipped", proof_refs=[],
                             error="agent CLI not available"),
            )
            return proof_to_json(proof)

        # Phase 2: Reconcile — claim CYNIC scope
        claim = reconcile_claim(obs, claim_scope, agent_id=session_agent_id)

        if not claim.claimed:
            # If the claim was intentionally skipped (e.g. working_dir outside
            # any CYNIC repo), the agent CLI can still execute — the proof will
            # note that scope coordination was not applicable.
            if claim.skipped:
                execution = execute_agent_cli(
                    obs, claim, model, timeout, auto_approve, json_output
                )
                release = release_claim(obs, claim, execution, agent_id=claim.agent_id)
                proof = build_proof(obs, claim, execution, release)
                return proof_to_json(proof)

            # Otherwise, claim failed (conflict or kernel unreachable) — block
            # execution. CYNIC coordination is fail-closed.
            execution = ExecutionResult(
                argv=[], exit_code=-1, duration_seconds=0.0,
                stdout_chars=0, stderr_chars=0, stdout_preview="",
                stderr_preview=claim.error or "claim failed",
                timed_out=False, output_parseable=False,
            )
            release = ReleaseProof(
                status="skipped", proof_refs=[],
                error=claim.error,
            )
            proof = build_proof(obs, claim, execution, release)
            return proof_to_json(proof)

        # Phase 3: Bounded action — execute the agent CLI
        execution = execute_agent_cli(obs, claim, model, timeout, auto_approve, json_output)

        # Phase 4: Read-back proof — release claim with proof refs
        release = release_claim(obs, claim, execution, agent_id=claim.agent_id)

        # Assemble final proof
        proof = build_proof(obs, claim, execution, release)
        return proof_to_json(proof)

    except Exception as exc:
        # Every error path must produce machine-readable evidence, never
        # a silent failure or a fake SUCCESS.
        return json.dumps({
            "schema": "cynic.agent_supervision.v1",
            "status": "blocked",
            "agent": agent,
            "working_dir": working_dir,
            "error": str(exc),
            "error_type": type(exc).__name__,
            "elapsed_seconds": 0,
        }, indent=2, ensure_ascii=False)


# ── Tool: cynic_agent_claims ───────────────────────────────────────────────

CYNIC_AGENT_CLAIMS_SCHEMA = _schema(
    "cynic_agent_claims",
    "Query active CYNIC agent claims (claim/status coordination state). "
    "Returns a machine-readable JSON report of all active claims, conflicts, "
    "and lease states. Read-only — does not mutate any claim.",
    {},
    required=None,
)


def handle_cynic_agent_claims(args: Dict[str, Any], **_kw) -> str:
    """Query active CYNIC claims."""
    return get_claims_report()


# ── Tool: cynic_agent_list ─────────────────────────────────────────────────

CYNIC_AGENT_LIST_SCHEMA = _schema(
    "cynic_agent_list",
    "List available agent CLIs detectable on this system with their availability "
    "and version. Read-only inspection — does not execute any agent.",
    {},
    required=None,
)

def handle_cynic_agent_list(args: Dict[str, Any], **_kw) -> str:
    """List available agent CLIs."""
    from .supervisor import _find_agent_binary, _probe_agent_version, _get_cynic_session_context

    results = []
    for name, spec in AGENT_SPECS.items():
        binary = _find_agent_binary(spec)
        if binary:
            version = _probe_agent_version(binary)
            results.append({
                "name": name,
                "available": True,
                "binary": binary,
                "version": version,
                "description": spec["description"],
            })
        else:
            results.append({
                "name": name,
                "available": False,
                "binary": None,
                "version": "unknown",
                "description": spec["description"],
                "error": f"binary not found. Expected: {spec['binaries']}",
            })

    ctx = _get_cynic_session_context()

    return json.dumps({
        "schema": "cynic.agent_list_report.v1",
        "status": "observed",
        "agents": results,
        "cynic_context": ctx,
    }, indent=2, ensure_ascii=False, default=str)


# ── Helpers ────────────────────────────────────────────────────────────────

def _json(obj: Any) -> str:
    return json.dumps(obj, ensure_ascii=False)


def _err(msg: str, **extra: Any) -> str:
    return _json({"success": False, "error": msg, **extra})
