"""CYNIC agent supervisor — observe, claim, bounded action, read-back proof.

Wraps agent CLIs (OpenCode, Claude Code, Codex, Gemini) as supervised
subprocesses within CYNIC's coherence pattern. Each invocation follows:

    desired state -> observe -> reconcile -> bounded action -> read-back proof

All evidence is machine-readable JSON. The supervisor never fabricates
success — every Command variant reaches a real executor with live read-back.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import time
import uuid
from dataclasses import dataclass
from typing import Any, Dict, Optional

# ── Constants ──────────────────────────────────────────────────────────────

_SUPERVISION_SCHEMA = "cynic.agent_supervision.v1"
_CLAIMS_SCHEMA = "cynic.agent_claims_report.v1"
_MAX_OUTPUT_CHARS = 16_384
_CYNIC_CLI_TIMEOUT = 30  # seconds for cynic CLI observation calls (kernel + local fallback)
_AGENT_CLIS_TIMEOUT = 3  # seconds for agent CLI availability probes


def _output_text(value: str | bytes | None) -> str:
    """Normalize subprocess output, including bytes captured on timeout."""
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value or ""


# ── Agent CLI argv builders ───────────────────────────────────────────────


def _opencode_argv(
    prompt: str, working_dir: str, model: Optional[str],
    timeout: int, auto_approve: bool, json_output: bool,
) -> list[str]:
    """Build argv for OpenCode one-shot JSON mode."""
    argv = ["opencode", "run", prompt]
    argv.extend(["--dir", working_dir])
    if model:
        argv.extend(["--model", model])
    if json_output:
        argv.extend(["--format", "json"])
    argv.append("--print-logs")
    return argv


def _claude_argv(
    prompt: str, working_dir: str, model: Optional[str],
    timeout: int, auto_approve: bool, json_output: bool,
) -> list[str]:
    """Build argv for Claude Code one-shot JSON mode."""
    argv = ["claude", "-p", prompt]
    argv.extend(["--cwd", working_dir])
    if model:
        argv.extend(["--model", model])
    if json_output:
        argv.extend(["--output-format", "json"])
    if auto_approve:
        argv.append("--dangerously-skip-permissions")
    # Hard cap turns to prevent runaway loops
    argv.extend(["--max-turns", str(30)])
    return argv


def _codex_argv(
    prompt: str, working_dir: str, model: Optional[str],
    timeout: int, auto_approve: bool, json_output: bool,
) -> list[str]:
    """Build argv for Codex non-interactive exec mode."""
    argv = ["codex", "exec", prompt]
    argv.extend(["-C", working_dir])
    if model:
        argv.extend(["-m", model])
    if json_output:
        argv.append("--json")
    if auto_approve:
        argv.append("--dangerously-bypass-approvals-and-sandbox")
    return argv


def _gemini_argv(
    prompt: str, working_dir: str, model: Optional[str],
    timeout: int, auto_approve: bool, json_output: bool,
) -> list[str]:
    """Build argv for Gemini CLI headless JSON mode."""
    argv = ["gemini", "-p", prompt]
    argv.extend(["-S", working_dir])
    if model:
        argv.extend(["-m", model])
    if json_output:
        argv.extend(["--output-format", "json"])
    if auto_approve:
        argv.append("--approval-mode=yolo")
    return argv


# Agent CLI specifications: how to invoke each in one-shot JSON mode.
AGENT_SPECS: Dict[str, Dict[str, Any]] = {
    "opencode": {
        "binaries": ["opencode"],
        "description": "OpenCode CLI (provider-agnostic AI coding agent)",
        "argv": _opencode_argv,
    },
    "claude": {
        "binaries": ["claude"],
        "description": "Claude Code CLI (Anthropic)",
        "argv": _claude_argv,
    },
    "codex": {
        "binaries": ["codex"],
        "description": "Codex CLI (OpenAI)",
        "argv": _codex_argv,
    },
    "gemini": {
        "binaries": ["gemini"],
        "description": "Gemini CLI (Google)",
        "argv": _gemini_argv,
    },
}


# ── Data types ─────────────────────────────────────────────────────────────


@dataclass
class AgentCliInfo:
    """Phase 1 observation: agent CLI availability."""

    name: str
    binary: str
    available: bool
    version: str
    error: Optional[str] = None


@dataclass
class Observation:
    """Phase 1: Observe the environment and agent CLI availability."""

    agent: str
    prompt: str
    working_dir: str
    cli: AgentCliInfo
    git_sha: str
    git_remote: str
    builder: str
    cynic_version: str
    start_timestamp: float


@dataclass
class ClaimState:
    """Phase 2: Reconcile — claim state from CYNIC."""

    claimed: bool
    lease_id: str
    claim_type: str
    ttl_seconds: int
    already_claimed_by_self: bool
    error: Optional[str] = None
    skipped: bool = False  # True when scope coordination is not applicable (e.g. outside CYNIC repo)
    raw_claim: Optional[Dict[str, Any]] = None
    agent_id: str = ""


@dataclass
class ExecutionResult:
    """Phase 3: Bounded action — the agent CLI execution."""

    argv: list[str]
    exit_code: int
    duration_seconds: float
    stdout_chars: int
    stderr_chars: int
    stdout_preview: str
    stderr_preview: str
    timed_out: bool
    output_parseable: bool
    parsed_output: Optional[Any] = None


@dataclass
class ReleaseProof:
    """Phase 4: Read-back proof — release the claim with proof references."""

    status: str  # "released", "skipped", "failed"
    proof_refs: list[str]
    error: Optional[str] = None


@dataclass
class SupervisionProof:
    """Full proof of a supervised agent CLI invocation."""

    schema: str
    status: str  # "converged", "blocked", "timeout", "unknown"
    agent: str
    working_dir: str
    start_timestamp: float
    end_timestamp: float
    observation: Dict[str, Any]
    claim: Dict[str, Any]
    execution: Dict[str, Any]
    release: Dict[str, Any]
    proof_refs: list[str]


# ── Observation helpers ───────────────────────────────────────────────────


def _find_cynic_binary() -> Optional[str]:
    """Locate the cynic CLI binary in PATH."""
    return shutil.which("cynic")


def _find_agent_binary(spec: Dict[str, Any]) -> Optional[str]:
    """Locate the agent CLI binary, trying configured binaries in order."""
    for name in spec["binaries"]:
        path = shutil.which(name)
        if path:
            return path
    return None


def _probe_agent_version(binary: str, timeout: int = _AGENT_CLIS_TIMEOUT) -> str:
    """Probe the agent CLI version string."""
    try:
        result = subprocess.run(
            [binary, "--version"],
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
        output = (result.stdout or result.stderr).strip()
        if output:
            return output.split("\n")[0][:200]
    except (subprocess.TimeoutExpired, OSError):
        pass
    return "unknown"


def _get_cynic_session_context() -> Dict[str, Any]:
    """Query CYNIC session context (non-blocking)."""
    binary = _find_cynic_binary()
    if not binary:
        return {"status": "blocked", "error": "cynic CLI not found in PATH"}
    try:
        result = subprocess.run(
            [binary, "agent", "session-context", "--json"],
            capture_output=True,
            text=True,
            timeout=_CYNIC_CLI_TIMEOUT,
            check=False,
        )
        if result.returncode == 0 and result.stdout.strip():
            try:
                return json.loads(result.stdout.strip())
            except json.JSONDecodeError:
                return {"status": "blocked", "error": "invalid JSON from cynic"}
        return {
            "status": "blocked",
            "error": f"cynic session-context exit={result.returncode}",
        }
    except subprocess.TimeoutExpired:
        return {"status": "blocked", "error": "cynic session-context timed out"}
    except OSError as exc:
        return {"status": "blocked", "error": str(exc)}


def _find_cynic_repo_root(working_dir: str) -> Optional[str]:
    """Walk up from working_dir to find the CYNIC repo root."""
    current = os.path.abspath(working_dir)
    for _ in range(10):
        if os.path.isfile(os.path.join(current, ".agent-claims.json")):
            return current
        if os.path.isdir(os.path.join(current, "infra")) and os.path.isfile(
            os.path.join(current, "infra", "registry.json")
        ):
            return current
        parent = os.path.dirname(current)
        if parent == current:
            break
        current = parent
    return None


def _run_cynic_command(
    args: list[str],
    cwd: Optional[str] = None,
    timeout: int = _CYNIC_CLI_TIMEOUT,
    registry: Optional[str] = None,
) -> Optional[Dict[str, Any]]:
    """Run a cynic CLI command and parse JSON output. Returns None on failure.

    If registry is provided, --registry <path> is inserted after the
    subcommand path so the cynic CLI can locate infra/registry.json
    even when cwd is outside the CYNIC repo.
    """
    binary = _find_cynic_binary()
    if not binary:
        return None
    cmd = [binary] + args
    if registry and len(args) >= 2:
        # Insert --registry right after the subcommand (e.g. after "agent claim")
        cmd = [binary, args[0], args[1], "--registry", registry] + args[2:]
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
            cwd=cwd,
        )
        if result.returncode == 0 and result.stdout.strip():
            try:
                return json.loads(result.stdout.strip())
            except json.JSONDecodeError:
                pass
        return None
    except (subprocess.TimeoutExpired, OSError):
        return None


# ── Coherence phases ───────────────────────────────────────────────────────


def observe(
    agent: str,
    prompt: str,
    working_dir: str,
    model: Optional[str],
    timeout: int,
    auto_approve: bool,
    json_output: bool,
) -> Observation:
    """Phase 1: Observe — check agent CLI availability and CYNIC context."""
    spec = AGENT_SPECS.get(agent)
    if not spec:
        raise ValueError(
            f"unknown agent '{agent}'. Available: {list(AGENT_SPECS.keys())}"
        )

    binary = _find_agent_binary(spec)
    if not binary:
        cli_info = AgentCliInfo(
            name=agent,
            binary="",
            available=False,
            version="unknown",
            error=f"binary not found. Expected: {spec['binaries']}",
        )
    else:
        version = _probe_agent_version(binary)
        cli_info = AgentCliInfo(
            name=agent,
            binary=binary,
            available=True,
            version=version,
        )

    ctx = _get_cynic_session_context()
    cynic_version = "unknown"
    if isinstance(ctx, dict):
        cynic_version = ctx.get("version", "unknown")

    return Observation(
        agent=agent,
        prompt=prompt,
        working_dir=working_dir,
        cli=cli_info,
        git_sha=ctx.get("git_sha", "unknown") if isinstance(ctx, dict) else "unknown",
        git_remote=ctx.get("git_remote", "unknown") if isinstance(ctx, dict) else "unknown",
        builder=ctx.get("builder", "unknown") if isinstance(ctx, dict) else "unknown",
        cynic_version=cynic_version,
        start_timestamp=time.time(),
    )


def reconcile_claim(
    observation: Observation,
    claim_scope: Optional[str],
    agent_id: Optional[str] = None,
) -> ClaimState:
    """Phase 2: Reconcile — claim a CYNIC scope if not already held."""
    repo_root = _find_cynic_repo_root(observation.working_dir)
    agent_id = agent_id or os.environ.get(
        "CYNIC_AGENT_ID", f"hermes:{observation.agent}:{uuid.uuid4().hex[:8]}"
    )

    target = claim_scope or observation.working_dir
    claim_type = "file" if os.path.isfile(target) else "module"

    # If working_dir is outside the CYNIC repo, skip the claim.
    # The agent CLI can still execute; the proof will note the scope was
    # not CYNIC-coordinated.
    if repo_root is None:
        return ClaimState(
            claimed=False,
            lease_id="",
            claim_type=claim_type,
            ttl_seconds=1800,
            already_claimed_by_self=False,
            error=(
                "working_dir is outside any CYNIC repo — scope coordination "
                "skipped. Agent CLI will still execute."
            ),
            skipped=True,
            agent_id=agent_id,
        )

    registry_path = os.path.join(repo_root, "infra", "registry.json")
    argv = [
        "agent", "claim",
        "--agent-id", agent_id,
        "--target", target,
        "--claim-type", claim_type,
        "--ttl-seconds", "1800",
        "--budget-actions", "100",
        "--budget-wall-seconds", "3600",
        "--json",
    ]

    result = _run_cynic_command(argv, cwd=repo_root, registry=registry_path)
    if result is None:
        # CYNIC kernel unreachable — local fallback on .agent-claims.json
        # The cynic CLI already falls back to local mode internally.
        result = _run_cynic_command(argv, cwd=repo_root, registry=registry_path)

    if result is None or not isinstance(result, dict):
        return ClaimState(
            claimed=False,
            lease_id="",
            claim_type=claim_type,
            ttl_seconds=1800,
            already_claimed_by_self=False,
            error="cynic agent claim failed (kernel unreachable or parse error)",
            agent_id=agent_id,
        )

    if result.get("status") == "conflict":
        return ClaimState(
            claimed=False,
            lease_id="",
            claim_type=claim_type,
            ttl_seconds=1800,
            already_claimed_by_self=False,
            error=(
                f"scope conflict: "
                f"{result.get('error', 'another agent holds this scope')}"
            ),
            raw_claim=result,
            agent_id=agent_id,
        )

    if result.get("status") in ("claimed", "ok", "active"):
        return ClaimState(
            claimed=True,
            lease_id=result.get("lease_id", result.get("claim_id", "")),
            claim_type=claim_type,
            ttl_seconds=result.get("ttl_seconds", 1800),
            already_claimed_by_self=bool(result.get("already_claimed", False)),
            raw_claim=result,
            agent_id=agent_id,
        )

    return ClaimState(
        claimed=False,
        lease_id="",
        claim_type=claim_type,
        ttl_seconds=1800,
        already_claimed_by_self=False,
        error=f"unexpected claim response: {result.get('status', 'unknown')}",
        raw_claim=result,
        agent_id=agent_id,
    )


def execute_agent_cli(
    observation: Observation,
    claim: ClaimState,
    model: Optional[str],
    timeout: int,
    auto_approve: bool,
    json_output: bool,
) -> ExecutionResult:
    """Phase 3: Bounded action — execute the agent CLI subprocess with timeout."""
    spec = AGENT_SPECS[observation.agent]
    argv = spec["argv"](
        observation.prompt,
        observation.working_dir,
        model,
        timeout,
        auto_approve,
        json_output,
    )

    start = time.time()
    timed_out = False
    try:
        result = subprocess.run(
            argv,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
            cwd=observation.working_dir,
        )
        duration = time.time() - start
        exit_code = result.returncode
        stdout = _output_text(result.stdout)
        stderr = _output_text(result.stderr)
    except subprocess.TimeoutExpired as exc:
        duration = time.time() - start
        timed_out = True
        exit_code = -1
        stdout = _output_text(exc.stdout)
        stderr = _output_text(exc.stderr)
        stderr = (
            f"TIMEOUT: agent CLI exceeded the bounded timeout of {timeout}s. "
            f"Output captured up to termination.\n{stderr}"
        )

    # Attempt JSON parse if output format was requested
    parsed_output = None
    output_parseable = False
    if json_output and stdout.strip():
        try:
            parsed_output = json.loads(stdout.strip())
            output_parseable = True
        except (json.JSONDecodeError, ValueError):
            output_parseable = False

    return ExecutionResult(
        argv=argv,
        exit_code=exit_code,
        duration_seconds=round(duration, 3),
        stdout_chars=len(stdout),
        stderr_chars=len(stderr),
        stdout_preview=stdout[-_MAX_OUTPUT_CHARS:] if stdout else "",
        stderr_preview=stderr[-_MAX_OUTPUT_CHARS:] if stderr else "",
        timed_out=timed_out,
        output_parseable=output_parseable,
        parsed_output=parsed_output,
    )


def release_claim(
    observation: Observation,
    claim: ClaimState,
    execution: ExecutionResult,
    agent_id: Optional[str] = None,
) -> ReleaseProof:
    """Phase 4: Read-back proof — release the CYNIC claim with proof references.

    Even when no claim was held (skipped or conflict), a proof_ref is always
    emitted as verifiable evidence of the bounded action having been performed.
    The proof_ref is a SHA-256 hash of the execution envelope (agent, working_dir,
    exit_code, duration) — deterministic and independently verifiable.
    """
    agent_id = agent_id or claim.agent_id or os.environ.get("CYNIC_AGENT_ID", "hermes")

    # Build proof reference from execution evidence — ALWAYS emitted
    proof_hash = hashlib.sha256(
        f"{observation.agent}:{observation.working_dir}:"
        f"{execution.exit_code}:{execution.duration_seconds}".encode()
    ).hexdigest()[:16]
    proof_ref = (
        f"cynic:agent:proof:{observation.agent[:8]}:{proof_hash}:"
        f"{int(time.time())}"
    )

    if not claim.claimed:
        return ReleaseProof(
            status="skipped",
            proof_refs=[proof_ref],
            error=claim.error or "no active claim to release",
        )

    repo_root = _find_cynic_repo_root(observation.working_dir)
    target = (
        claim.raw_claim.get("target", observation.working_dir)
        if claim.raw_claim
        else observation.working_dir
    )
    argv = [
        "agent", "release",
        "--agent-id", agent_id,
        "--target", target,
        "--lease-id", claim.lease_id,
        "--fencing-token", "0",  # local fallback path
        "--proof-ref", proof_ref,
        "--json",
    ]

    registry_path = None
    if repo_root:
        registry_path = os.path.join(repo_root, "infra", "registry.json")

    result = _run_cynic_command(argv, cwd=repo_root, registry=registry_path)

    if result is not None and isinstance(result, dict):
        if result.get("status") in ("released", "ok"):
            return ReleaseProof(
                status="released",
                proof_refs=[proof_ref],
            )
        return ReleaseProof(
            status="failed",
            proof_refs=[proof_ref],
            error=(
                f"release rejected: "
                f"{result.get('error', result.get('status', 'unknown'))}"
            ),
        )

    # Even if cynic CLI is unreachable, the proof_ref is still valid evidence
    # of the bounded action having been performed. The agent loop must
    # read this proof back from the local claims file.
    return ReleaseProof(
        status="failed",
        proof_refs=[proof_ref],
        error=(
            "cynic CLI unreachable at release — proof_ref still emitted "
            "as evidence"
        ),
    )


def build_proof(
    observation: Observation,
    claim: ClaimState,
    execution: ExecutionResult,
    release: ReleaseProof,
) -> SupervisionProof:
    """Assemble the full read-back proof."""
    if execution.timed_out:
        status = "timeout"
    elif not observation.cli.available:
        status = "blocked"
    elif execution.exit_code != 0:
        status = "blocked"
    elif not claim.claimed:
        status = "blocked"
    else:
        status = "converged"

    end_ts = time.time()
    return SupervisionProof(
        schema=_SUPERVISION_SCHEMA,
        status=status,
        agent=observation.agent,
        working_dir=observation.working_dir,
        start_timestamp=observation.start_timestamp,
        end_timestamp=end_ts,
        observation={
            "cli_available": observation.cli.available,
            "cli_binary": observation.cli.binary,
            "cli_version": observation.cli.version,
            "cli_error": observation.cli.error,
            "git_sha": observation.git_sha,
            "git_remote": observation.git_remote,
            "builder": observation.builder,
            "cynic_version": observation.cynic_version,
        },
        claim={
            "claimed": claim.claimed,
            "agent_id": claim.agent_id,
            "lease_id": claim.lease_id,
            "claim_type": claim.claim_type,
            "ttl_seconds": claim.ttl_seconds,
            "already_claimed_by_self": claim.already_claimed_by_self,
            "error": claim.error,
        },
        execution={
            "argv": execution.argv,
            "exit_code": execution.exit_code,
            "duration_seconds": execution.duration_seconds,
            "stdout_chars": execution.stdout_chars,
            "stderr_chars": execution.stderr_chars,
            "stdout_preview": execution.stdout_preview,
            "stderr_preview": execution.stderr_preview,
            "timed_out": execution.timed_out,
            "output_parseable": execution.output_parseable,
            "parsed_output": execution.parsed_output,
        },
        release={
            "status": release.status,
            "proof_refs": release.proof_refs,
            "error": release.error,
        },
        proof_refs=release.proof_refs,
    )


def proof_to_json(proof: SupervisionProof) -> str:
    """Serialize a supervision proof to pretty JSON."""
    return json.dumps(
        {
            "schema": proof.schema,
            "status": proof.status,
            "agent": proof.agent,
            "working_dir": proof.working_dir,
            "elapsed_seconds": round(
                proof.end_timestamp - proof.start_timestamp, 3
            ),
            "observation": proof.observation,
            "claim": proof.claim,
            "execution": {
                k: v
                for k, v in proof.execution.items()
                if k not in ("stdout_preview", "stderr_preview", "parsed_output")
            },
            "release": proof.release,
            "proof_refs": proof.proof_refs,
        },
        indent=2,
        ensure_ascii=False,
        default=str,
    )


def get_claims_report() -> str:
    """Query active CYNIC claims and return a compact JSON report."""
    binary = _find_cynic_binary()
    if not binary:
        return json.dumps(
            {
                "schema": _CLAIMS_SCHEMA,
                "status": "blocked",
                "error": "cynic CLI not found in PATH",
            },
            indent=2,
            ensure_ascii=False,
        )

    result = _run_cynic_command(["agent", "status", "--json"])

    if result is None:
        return json.dumps(
            {
                "schema": _CLAIMS_SCHEMA,
                "status": "blocked",
                "error": (
                    "cynic agent status failed "
                    "(kernel unreachable or parse error)"
                ),
            },
            indent=2,
            ensure_ascii=False,
        )

    result["schema"] = _CLAIMS_SCHEMA
    return json.dumps(result, indent=2, ensure_ascii=False, default=str)
