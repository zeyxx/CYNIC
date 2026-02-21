"""
CYNIC act router — act/execute · act/telemetry
"""
from __future__ import annotations

import logging
from typing import Any


from fastapi import APIRouter, Depends, HTTPException, Query

from cynic.core.formulas import CONFIDENCE_ENRICHMENT_MIN_THRESHOLD
from cynic.metabolism.telemetry import classify_task, compute_reward
from cynic.api.state import get_app_container, AppContainer

logger = logging.getLogger("cynic.api.server")

router_act = APIRouter(tags=["act"])


# ════════════════════════════════════════════════════════════════════════════
# CYNIC → Claude context injection (L2 bidirectional loop)
# ════════════════════════════════════════════════════════════════════════════

def _enrich_prompt(prompt: str, state) -> str:
    """
    Inject CYNIC context into Claude's prompt (CYNIC→Claude direction of L2).

    Prepends a compact block with:
    - Compressed session history (≤200 tokens from ContextCompressor)
    - Best learned action from QTable for this task type
    - QTable confidence level

    Returns enriched prompt when useful context exists, raw prompt otherwise.
    Skips enrichment if compressor is empty and QTable has no data (early sessions).
    """
    task_type = classify_task(prompt)
    state_key = f"SDK:default:{task_type}:medium"

    best_action = state.qtable.exploit(state_key)
    confidence = state.qtable.confidence(state_key)

    try:
        context_summary = state.context_compressor.get_compressed_context(budget=200)
    except httpx.RequestError:
        context_summary = ""

    # Skip enrichment if nothing useful yet (cold start)
    if not context_summary and confidence < CONFIDENCE_ENRICHMENT_MIN_THRESHOLD:
        return prompt

    lines = ["# CYNIC Context (kernel guidance)"]
    if context_summary:
        lines.append(f"## Session history\n{context_summary}")
    if confidence >= CONFIDENCE_ENRICHMENT_MIN_THRESHOLD:
        lines.append(
            f"## Learned guidance\n"
            f"Task type: {task_type} | Suggested approach: {best_action} "
            f"(confidence: {confidence:.0%})"
        )
    lines.append("---\n")
    return "\n".join(lines) + prompt


# ════════════════════════════════════════════════════════════════════════════
# POST /act/execute  (CYNIC spawns Claude Code autonomously)
# ════════════════════════════════════════════════════════════════════════════

@router_act.post("/act/execute")
async def act_execute(
    body: dict[str, Any],
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    CYNIC executes a task by spawning Claude Code autonomously.

    Body:
        {"prompt": "...", "cwd": "/path/to/project", "model": "claude-haiku-4-5",
         "timeout": 300}

    CYNIC launches `claude --sdk-url ws://localhost:PORT/ws/sdk` as a subprocess.
    Every tool call Claude makes is intercepted and judged by GUARDIAN.
    The result is returned when Claude's result message arrives.

    This is the ACT phase of the PERCEIVE → JUDGE → DECIDE → ACT cycle.
    No human needed — CYNIC does it entirely.
    """
    state = container.organism

    if state.runner is None:
        raise HTTPException(
            status_code=503,
            detail="ClaudeCodeRunner not initialized — kernel not started via lifespan",
        )

    prompt = body.get("prompt", "")
    if not prompt:
        raise HTTPException(status_code=400, detail="prompt is required")

    cwd = body.get("cwd")
    model = body.get("model")
    timeout = float(body.get("timeout", 300.0))

    logger.info("*ears perk* ACT requested: %s...", prompt[:80])

    enriched = _enrich_prompt(prompt, state)
    result = await state.runner.execute(enriched, cwd=cwd, model=model, timeout=timeout)

    if not result.get("success"):
        # Log failure but return structured response (not HTTP error)
        logger.warning("*GROWL* ACT failed: %s", result.get("error"))

    return {
        "success": result.get("success", False),
        "session_id": result.get("session_id"),
        "cost_usd": result.get("cost_usd", 0.0),
        "total_cost_usd": result.get("total_cost_usd", 0.0),
        "exec_id": result.get("exec_id"),
        "error": result.get("error"),
        "message": (
            f"*tail wag* Task executed (cost=${result.get('cost_usd', 0.0):.4f})"
            if result.get("success")
            else f"*GROWL* Task failed: {result.get('error')}"
        ),
    }


# ════════════════════════════════════════════════════════════════════════════
# GET /act/telemetry  (session telemetry — learning measurement layer)
# ════════════════════════════════════════════════════════════════════════════

@router_act.get("/act/telemetry")
async def act_telemetry(
    n: int = Query(default=10, ge=1, le=100),
    export: bool = Query(default=False),
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Session telemetry — CYNIC's learning measurement layer.

    Returns aggregate stats + recent sessions for H1-H5 hypothesis testing.

    Query params:
      n=10       → return last N sessions (max 100)
      export=true → return all sessions (full JSONL export)

    Stats include:
      - count, error_rate, mean_cost, mean_reward
      - verdicts (BARK/GROWL/WAG/HOWL distribution)
      - task_types (debug/refactor/test/review/write/explain/general)
      - complexities (trivial/simple/medium/complex)

    Research use: GET /act/telemetry?export=true → download for H1-H5 analysis
    """
    state = container.organism
    store = state.telemetry_store

    result = {
        "stats": store.stats(),
        "sessions": store.export() if export else store.recent(n),
        "message": f"*sniff* {len(store)} sessions measured — φ sees all.",
    }
    return result
