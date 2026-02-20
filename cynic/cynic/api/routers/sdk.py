"""
CYNIC SDK router — ws/sdk · sdk/sessions · sdk/routing · sdk/last-session · sdk/task
"""
from __future__ import annotations

import asyncio
import json
import logging
import os
import pathlib as _pathlib
import time
import uuid
from dataclasses import dataclass, field
from typing import Any, List, Optional


from fastapi import APIRouter, HTTPException, WebSocket, WebSocketDisconnect

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import (
    DecisionMadePayload as _DecisionMadePayload,
    SdkResultReceivedPayload,
    SdkSessionStartedPayload,
    SdkToolJudgedPayload,
)
from cynic.core.phi import MAX_CONFIDENCE
from cynic.metabolism.telemetry import (
    SessionTelemetry as SDKTelemetry,
    classify_task,
    compute_reward,
    estimate_complexity,
)
from cynic.api.state import get_state

logger = logging.getLogger("cynic.api.server")

router_sdk = APIRouter(tags=["sdk"])

# Path for JSONL session persistence (survives restarts)
_SDK_SESSIONS_JSONL = os.path.join(os.path.expanduser("~"), ".cynic", "sdk_sessions.jsonl")


def _append_sdk_session_jsonl(record: SDKTelemetry) -> None:
    """Append one completed SDK session to JSONL file (fire-and-forget)."""
    try:
        import dataclasses as _dc
        os.makedirs(os.path.dirname(_SDK_SESSIONS_JSONL), exist_ok=True)
        with open(_SDK_SESSIONS_JSONL, "a", encoding="utf-8") as fh:
            fh.write(json.dumps(_dc.asdict(record)) + "\n")
    except Exception as exc:
        logger.debug("sdk_sessions.jsonl append skipped: %s", exc)


# ════════════════════════════════════════════════════════════════════════════
# SDK SESSION REGISTRY
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class SDKSession:
    """
    Tracks one active Claude Code --sdk-url WebSocket session.

    Each session is a running `claude --sdk-url ws://localhost:PORT/ws/sdk`
    process. CYNIC is the server; Claude Code is the client (the HANDS).

    Telemetry fields (prefixed _) are populated during the session and
    used to build a SessionTelemetry record when the result message arrives.
    """
    session_id: str
    ws: Any                                  # WebSocket — typed as Any to avoid circular import issues
    cwd: str = ""
    tools: list[str] = field(default_factory=list)
    model: str = "unknown"
    claude_code_version: str = ""
    cli_session_id: str = ""                 # Claude's internal session ID — used for --resume
    total_cost_usd: float = 0.0
    connected_at: float = field(default_factory=time.time)
    log: list[dict[str, Any]] = field(default_factory=list)

    # ── Telemetry (populated during session, consumed at result) ──────────
    _task_prompt: str = ""                           # last task sent to this session
    _tool_sequence: list[str] = field(default_factory=list)  # ordered tool names
    _result_text: str = ""                           # Claude's result description
    _input_tokens: int = 0                           # accumulated from assistant msgs
    _output_tokens: int = 0

    def record(self, msg_type: str, data: dict[str, Any]) -> None:
        self.log.append({"type": msg_type, "data": data, "ts": time.time()})

    def to_dict(self) -> dict[str, Any]:
        return {
            "session_id": self.session_id,
            "cwd": self.cwd,
            "model": self.model,
            "claude_code_version": self.claude_code_version,
            "cli_session_id": self.cli_session_id,
            "tools": self.tools,
            "total_cost_usd": round(self.total_cost_usd, 6),
            "connected_at": self.connected_at,
            "events": len(self.log),
            "tool_count": len(self._tool_sequence),
        }


# Process-level registry of active SDK sessions
_sdk_sessions: dict[str, SDKSession] = {}


# ════════════════════════════════════════════════════════════════════════════
# WS /ws/sdk  (Claude Code --sdk-url server — CYNIC is the BRAIN)
# ════════════════════════════════════════════════════════════════════════════

@router_sdk.websocket("/ws/sdk")
async def ws_sdk(websocket: WebSocket) -> None:
    """
    Claude Code SDK WebSocket server.

    Claude Code connects here as a HEADLESS CLIENT when launched with:
      claude --sdk-url ws://localhost:8765/ws/sdk \\
             --print --output-format stream-json --input-format stream-json

    CYNIC is the SERVER (the BRAIN). Claude Code is the CLIENT (the HANDS).
    CYNIC intercepts every tool use, judges it with GUARDIAN (REFLEX level),
    and learns from every result to build the Q-Table from real usage.

    Message flow (NDJSON — each line is one JSON object):

      CLI → CYNIC: system/init        → record session metadata
      CLI → CYNIC: can_use_tool       → CYNIC judges → control_response allow/deny
      CLI → CYNIC: assistant          → record to session log
      CLI → CYNIC: result             → record cost, emit SDK_RESULT_RECEIVED
      CLI → CYNIC: keep_alive         → respond keep_alive

      CYNIC → CLI: keep_alive         → heartbeat
      CYNIC → CLI: user               → send task (via POST /sdk/task)
      CYNIC → CLI: control_response   → approve/deny/modify tool use
      CYNIC → CLI: set_model          → switch Sonnet/Haiku mid-session

    Bootstrap loop:
      Phase 1: CYNIC intercepts all tool calls → builds Q-Table from real Claude sessions
      Phase 2: Q-Table confidence rises → CYNIC routes simple tasks to Ollama
      Phase 3: 80%+ tasks → Ollama ($0 cost). Claude only for novel tasks.
    """
    await websocket.accept()
    state = get_state()
    bus = get_core_bus()

    session_id = str(uuid.uuid4())
    session = SDKSession(session_id=session_id, ws=websocket)
    _sdk_sessions[session_id] = session

    logger.info("*ears perk* SDK session connected: %s", session_id)

    async def _send(msg: dict[str, Any]) -> None:
        """Send one NDJSON message to Claude Code."""
        await websocket.send_text(json.dumps(msg) + "\n")

    async def _judge_tool(tool_name: str, tool_input: dict[str, Any]) -> str:
        """
        Fast REFLEX judgment on a tool use request.
        Returns "BARK"/"GROWL"/"WAG"/"HOWL".
        """
        from cynic.core.judgment import Cell, infer_time_dim
        _tool_content = f"{tool_name}: {json.dumps(tool_input)[:400]}"
        _tool_ctx = f"SDK tool use — session {session_id[:8]}"
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            time_dim=infer_time_dim(_tool_content, _tool_ctx, "JUDGE"),
            content=_tool_content,
            context=_tool_ctx,
            lod=0,
            budget_usd=0.0005,
        )
        try:
            judgment = await state.orchestrator.run(
                cell, level=ConsciousnessLevel.REFLEX
            )
            return judgment.verdict
        except Exception as exc:
            logger.warning("SDK tool judgment error: %s", exc)
            return "WAG"  # Safe default: allow on error

    try:
        while True:
            raw = await websocket.receive_text()

            # NDJSON: each frame may contain one or more \n-terminated objects
            for line in raw.splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    msg = json.loads(line)
                except json.JSONDecodeError:
                    logger.debug("SDK invalid JSON: %r", line[:100])
                    continue

                msg_type = msg.get("type", "")
                msg_subtype = msg.get("subtype", "")

                # ── system/init ──────────────────────────────────────────────
                if msg_type == "system" and msg_subtype == "init":
                    session.cwd = msg.get("cwd", "")
                    session.tools = msg.get("tools", [])
                    session.model = msg.get("model", "unknown")
                    session.claude_code_version = msg.get("claude_code_version", "")
                    # Claude's internal session ID — persisted for --resume on restart
                    session.cli_session_id = msg.get("session_id", "")
                    session.record("init", {
                        "cwd": session.cwd,
                        "model": session.model,
                        "tools_count": len(session.tools),
                    })

                    logger.info(
                        "*sniff* SDK init: model=%s tools=%d cwd=%s",
                        session.model, len(session.tools), session.cwd,
                    )

                    await bus.emit(Event.typed(
                        CoreEvent.SDK_SESSION_STARTED,
                        SdkSessionStartedPayload(
                            session_id=session_id,
                            cli_session_id=session.cli_session_id,
                            model=session.model,
                            cwd=session.cwd,
                            tools=session.tools,
                        ),
                        source="ws_sdk",
                    ))

                    # Respond with keep_alive — server is ready
                    await _send({"type": "keep_alive"})

                # ── can_use_tool (tool permission request) ───────────────────
                elif msg_type == "control_request" and msg_subtype == "can_use_tool":
                    request_id = msg.get("request_id", str(uuid.uuid4()))
                    request = msg.get("request", {})
                    tool_name = request.get("tool_name", "unknown")
                    tool_input = request.get("input", {})

                    verdict = await _judge_tool(tool_name, tool_input)

                    if verdict == "BARK":
                        # GUARDIAN blocks: deny tool use
                        deny_msg = f"*GROWL* CYNIC GUARDIAN blocked: {tool_name}"
                        response = {
                            "type": "control_response",
                            "response": {
                                "subtype": "success",
                                "request_id": request_id,
                                "response": {
                                    "behavior": "deny",
                                    "message": deny_msg,
                                },
                            },
                        }
                        logger.warning("*GROWL* SDK BLOCKED: %s", tool_name)
                    else:
                        # WAG / GROWL / HOWL → allow (GROWL logs warning)
                        if verdict == "GROWL":
                            logger.warning("*sniff* SDK WARNED: %s (Q low)", tool_name)
                        response = {
                            "type": "control_response",
                            "response": {
                                "subtype": "success",
                                "request_id": request_id,
                                "response": {
                                    "behavior": "allow",
                                    "updatedInput": tool_input,
                                },
                            },
                        }

                    await _send(response)

                    behavior = "deny" if verdict == "BARK" else "allow"
                    session._tool_sequence.append(tool_name)  # telemetry: ordered sequence
                    session.record("tool_judged", {
                        "tool": tool_name,
                        "verdict": verdict,
                        "behavior": behavior,
                    })

                    await bus.emit(Event.typed(
                        CoreEvent.SDK_TOOL_JUDGED,
                        SdkToolJudgedPayload(
                            session_id=session_id,
                            tool=tool_name,
                            verdict=verdict,
                        ),
                        source="ws_sdk",
                    ))

                # ── assistant (Claude's response — streaming) ────────────────
                elif msg_type == "assistant":
                    message = msg.get("message", {})
                    usage = message.get("usage", {})
                    content = message.get("content", [])
                    text_blocks = sum(1 for b in content if b.get("type") == "text")
                    tool_blocks = sum(1 for b in content if b.get("type") == "tool_use")
                    # Accumulate tokens across all assistant messages for telemetry
                    session._input_tokens += usage.get("input_tokens", 0)
                    session._output_tokens += usage.get("output_tokens", 0)
                    session.record("assistant", {
                        "text_blocks": text_blocks,
                        "tool_blocks": tool_blocks,
                        "input_tokens": usage.get("input_tokens", 0),
                        "output_tokens": usage.get("output_tokens", 0),
                    })

                # ── result (task complete) ───────────────────────────────────
                elif msg_type == "result":
                    is_error = msg.get("is_error", False)
                    cost = float(msg.get("total_cost_usd") or 0.0)
                    duration_ms = float(msg.get("duration_ms") or 0.0)
                    result_text = msg.get("result", "")
                    result_subtype = msg.get("subtype", "unknown")
                    result_usage = msg.get("usage", {})

                    session.total_cost_usd += cost
                    session._result_text = result_text
                    # Accumulate final usage
                    session._input_tokens += result_usage.get("input_tokens", 0)
                    session._output_tokens += result_usage.get("output_tokens", 0)

                    session.record("result", {
                        "subtype": result_subtype,
                        "is_error": is_error,
                        "cost_usd": cost,
                        "result_text": result_text[:200],
                    })

                    # ── Rich Q-Learning signal (28 states vs 1 before) ───────
                    task_type = classify_task(session._task_prompt)
                    complexity = estimate_complexity(session._tool_sequence)
                    reward = compute_reward(is_error, len(session._tool_sequence), cost)
                    rich_state_key = f"SDK:{session.model}:{task_type}:{complexity}"

                    from cynic.learning.qlearning import LearningSignal as _LS
                    state.qtable.update(_LS(
                        state_key=rich_state_key,
                        action="BARK" if is_error else "HOWL",
                        reward=reward,
                        judgment_id=session_id,
                        loop_name="SDK_RESULT",
                    ))

                    # ── Quality judgment of Claude's output (REFLEX) ─────────
                    judgment_content = (
                        f"Task: {session._task_prompt[:200]}\n"
                        f"Result: {result_text[:300]}\n"
                        f"Tools: {', '.join(session._tool_sequence[:10])}\n"
                        f"Cost: ${cost:.4f} | Error: {is_error} | Type: {task_type}"
                    )
                    try:
                        from cynic.core.judgment import Cell as _Cell, infer_time_dim as _itd
                        quality_cell = _Cell(
                            reality="CODE", analysis="JUDGE",
                            time_dim=_itd(judgment_content, "", "JUDGE"),
                            content=judgment_content,
                            context=f"SDK quality — session {session_id[:8]}",
                            lod=0, budget_usd=0.001,
                        )
                        qj = await state.orchestrator.run(
                            quality_cell, level=ConsciousnessLevel.REFLEX
                        )
                        q_score = round(qj.q_score, 3)
                        verdict = qj.verdict
                        confidence = round(min(qj.confidence, MAX_CONFIDENCE), 3)
                    except Exception as _exc:
                        logger.debug("Quality judgment skipped: %s", _exc)
                        q_score, verdict, confidence = 30.0, "GROWL", 0.382

                    # ── Build and store SessionTelemetry ─────────────────────
                    tool_judgments = [e for e in session.log if e["type"] == "tool_judged"]
                    allowed = sum(1 for e in tool_judgments if e["data"]["behavior"] == "allow")
                    denied = sum(1 for e in tool_judgments if e["data"]["behavior"] == "deny")

                    telemetry_record = SDKTelemetry(
                        session_id=session_id,
                        task=session._task_prompt[:500],
                        task_type=task_type,
                        complexity=complexity,
                        model=session.model,
                        tools_sequence=session._tool_sequence.copy(),
                        tools_allowed=allowed,
                        tools_denied=denied,
                        tool_allow_rate=round(allowed / max(len(tool_judgments), 1), 3),
                        input_tokens=session._input_tokens,
                        output_tokens=session._output_tokens,
                        total_cost_usd=round(session.total_cost_usd, 6),
                        duration_s=round(duration_ms / 1000, 2),
                        is_error=is_error,
                        result_text=result_text[:500],
                        output_q_score=q_score,
                        output_verdict=verdict,
                        output_confidence=confidence,
                        state_key=rich_state_key,
                        reward=reward,
                        cli_session_id=session.cli_session_id,
                    )
                    state.telemetry_store.add(telemetry_record)

                    # ── JSONL persistence (survives restarts) ─────────────────
                    _append_sdk_session_jsonl(telemetry_record)

                    # ── L2→L1 cross-feed: BARK/error → ActionProposer ─────────
                    # Links L2 (SDK result) → L1 (action queue) automatically.
                    if is_error or verdict == "BARK":
                        await bus.emit(Event.typed(
                            CoreEvent.DECISION_MADE,
                            _DecisionMadePayload(
                                recommended_action="BARK",
                                verdict="BARK",
                                reality="CYNIC",
                                state_key=rich_state_key,
                                q_value=reward,
                                action_prompt=(
                                    f"SDK session {session_id[:8]} failed ({task_type}). "
                                    f"Review: {result_text[:200]}"
                                ),
                                judgment_id=session_id,
                                content_preview=(session._task_prompt or "")[:60],
                            ),
                            source="sdk_result",
                        ))

                    # Persist to DB (fire-and-forget, best-effort)
                    if state._pool is not None:
                        import dataclasses as _dc
                        _rec_dict = _dc.asdict(telemetry_record)
                        async def _persist_sdk_session(d=_rec_dict):
                            try:
                                from cynic.core.storage.postgres import SDKSessionRepository as _SDKSessionRepo
                                await _SDKSessionRepo().save(d)
                            except Exception as _e:
                                logger.debug("SDK session persist skipped: %s", _e)
                        asyncio.create_task(_persist_sdk_session())

                    await bus.emit(Event.typed(
                        CoreEvent.SDK_RESULT_RECEIVED,
                        SdkResultReceivedPayload(
                            session_id=session_id,
                            is_error=is_error,
                            cost_usd=cost,
                            output_q_score=q_score,
                            total_cost_usd=round(session.total_cost_usd, 6),
                            reward=reward,
                            task_type=task_type,
                            complexity=complexity,
                            output_verdict=verdict,
                        ),
                        source="ws_sdk",
                    ))

                    # ── Ring 4 LLM routing suggestion ─────────────────────────
                    # After recording the result, ask the router whether the NEXT
                    # task of this type should use Haiku instead of Sonnet.
                    # route_to_local=True only once Q-Table is warm enough (PHI_INV
                    # confidence + 3 visits). Cold-start: always stays on Sonnet.
                    if state.llm_router is not None:
                        routing = state.llm_router.route(
                            rich_state_key, state.qtable, task_type, complexity
                        )
                        if routing.route_to_local:
                            logger.info(
                                "LLM_ROUTER: %s → %s (%s)",
                                rich_state_key, routing.recommended_model, routing.reason,
                            )

                    logger.info(
                        "*%s* SDK result: %s task=%s complexity=%s verdict=%s Q=%.1f cost=$%.4f",
                        "tail wag" if not is_error else "GROWL",
                        result_subtype, task_type, complexity, verdict, q_score, cost,
                    )

                # ── keep_alive ───────────────────────────────────────────────
                elif msg_type == "keep_alive":
                    await _send({"type": "keep_alive"})

                # ── everything else: log and ignore ──────────────────────────
                else:
                    logger.debug("SDK unhandled: type=%s subtype=%s", msg_type, msg_subtype)

    except WebSocketDisconnect:
        pass
    except Exception as exc:
        logger.error("SDK session error: %s", exc, exc_info=True)
    finally:
        _sdk_sessions.pop(session_id, None)
        logger.info(
            "*yawn* SDK session ended: %s — %d events, cost=$%.4f",
            session_id, len(session.log), session.total_cost_usd,
        )


# ════════════════════════════════════════════════════════════════════════════
# GET /sdk/sessions  (list active Claude Code sessions)
# ════════════════════════════════════════════════════════════════════════════

@router_sdk.get("/sdk/sessions")
async def sdk_sessions() -> dict[str, Any]:
    """List all active Claude Code --sdk-url sessions."""
    return {
        "active": len(_sdk_sessions),
        "sessions": [s.to_dict() for s in _sdk_sessions.values()],
    }


@router_sdk.get("/sdk/routing")
async def sdk_routing() -> dict[str, Any]:
    """
    LLM routing stats — Ring 4 Q-Table driven model selection.

    Shows how often CYNIC routes SDK tasks from Sonnet → Haiku based on
    accumulated Q-Table confidence. local_rate rises as Q-Table warms up.
    """
    state = get_state()
    if state.llm_router is None:
        return {"available": False}
    return {"available": True, **state.llm_router.stats()}


@router_sdk.get("/sdk/last-session")
async def sdk_last_session(cwd: str = "") -> dict[str, Any]:
    """
    Return the last known cli_session_id for --resume.

    Lookup order:
      1. In-memory active sessions (current process)
      2. JSONL file (~/.cynic/sdk_sessions.jsonl) — survives restarts

    Query param: cwd (optional) — filter by working directory.
    """
    # 1. In-memory active sessions
    candidates = list(_sdk_sessions.values())
    if cwd:
        candidates = [s for s in candidates if s.cwd == cwd]
    if candidates:
        latest = max(candidates, key=lambda s: s.connected_at)
        if latest.cli_session_id:
            return {"cli_session_id": latest.cli_session_id, "found": True, "source": "memory"}

    # 2. JSONL file fallback
    try:
        jsonl_path = _pathlib.Path(_SDK_SESSIONS_JSONL)
        if jsonl_path.exists():
            last_sid = ""
            with jsonl_path.open("r", encoding="utf-8") as fh:
                for line in fh:
                    try:
                        rec = json.loads(line.strip())
                        sid = rec.get("cli_session_id", "")
                        if sid and (not cwd or rec.get("cwd", "") == cwd):
                            last_sid = sid
                    except Exception:
                        pass
            if last_sid:
                return {"cli_session_id": last_sid, "found": True, "source": "jsonl"}
    except Exception:
        pass

    return {"cli_session_id": "", "found": False, "source": "none"}


# ════════════════════════════════════════════════════════════════════════════
# POST /sdk/task  (send a task to a connected Claude Code session)
# ════════════════════════════════════════════════════════════════════════════

@router_sdk.post("/sdk/task")
async def sdk_task(body: dict[str, Any]) -> dict[str, Any]:
    """
    Send a task (user message) to a connected Claude Code session.

    Body: {"session_id": "...", "prompt": "...", "model": "claude-haiku-4-5"}

    If session_id is omitted, uses the most recently connected session.
    If model is provided, sends set_model before the task (model routing).
    """
    session_id = body.get("session_id")
    prompt = body.get("prompt", "")
    model_override = body.get("model")

    if not prompt:
        raise HTTPException(status_code=400, detail="prompt is required")

    # Resolve session
    session: SDKSession | None = None
    if session_id:
        session = _sdk_sessions.get(session_id)
    elif _sdk_sessions:
        # Most recent session
        session = max(_sdk_sessions.values(), key=lambda s: s.connected_at)

    if session is None:
        raise HTTPException(
            status_code=404,
            detail=f"No active SDK session. Run: claude --sdk-url ws://HOST:PORT/ws/sdk --print --output-format stream-json --input-format stream-json",
        )

    # Optional model routing (e.g. switch to Haiku for cheap tasks)
    if model_override and model_override != session.model:
        await session.ws.send_text(json.dumps({
            "type": "control_response",
            "response": {
                "subtype": "success",
                "request_id": str(uuid.uuid4()),
                "response": {"subtype": "set_model", "model": model_override},
            },
        }) + "\n")

    # Capture task prompt for telemetry (last task wins — typical single-task sessions)
    session._task_prompt = prompt

    # Send user message
    msg = {
        "type": "user",
        "message": {"role": "user", "content": prompt},
        "parent_tool_use_id": None,
        "session_id": session.session_id,
    }
    await session.ws.send_text(json.dumps(msg) + "\n")
    session.record("task_sent", {"prompt": prompt[:200], "model_override": model_override})

    logger.info("*tail wag* SDK task sent to session %s: %s...", session.session_id[:8], prompt[:80])

    return {
        "sent": True,
        "session_id": session.session_id,
        "model": model_override or session.model,
        "prompt_preview": prompt[:100],
    }
