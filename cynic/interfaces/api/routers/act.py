"""
CYNIC act router — act/execute · act/telemetry
"""
from __future__ import annotations

import logging
from typing import Any

from fastapi import APIRouter, Depends, HTTPException, Query

from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.kernel.core.formulas import CONFIDENCE_ENRICHMENT_MIN_THRESHOLD
from cynic.kernel.organism.metabolism.telemetry import classify_task

logger = logging.getLogger("cynic.interfaces.api.server")

router_act = APIRouter(tags=["act"])


def _enrich_prompt(prompt: str, state) -> str:
    """
    Inject CYNIC context into Claude's prompt (CYNIC→Claude direction of L2).
    """
    task_type = classify_task(prompt)
    state_key = f"SDK:default:{task_type}:medium"

    best_action = state.qtable.exploit(state_key)
    confidence = state.qtable.confidence(state_key)

    try:
        context_summary = state.context_compressor.get_compressed_context(budget=200)
    except Exception:
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


@router_act.post("/act/execute")
async def act_execute(
    body: dict[str, Any],
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    CYNIC executes a task by spawning Claude Code autonomously.
    """
    state = container.organism

    if state.metabolism.runner is None:
        raise HTTPException(
            status_code=503,
            detail="ClaudeCodeRunner not initialized",
        )

    prompt = body.get("prompt", "")
    if not prompt:
        raise HTTPException(status_code=400, detail="prompt is required")

    cwd = body.get("cwd")
    model = body.get("model")
    timeout = float(body.get("timeout", 300.0))

    logger.info("*ears perk* ACT requested: %s...", prompt[:80])

    enriched = _enrich_prompt(prompt, state)
    result = await state.metabolism.runner.execute(enriched, cwd=cwd, model=model, timeout=timeout)

    return {
        "success": result.get("success", False),
        "session_id": result.get("session_id"),
        "cost_usd": result.get("cost_usd", 0.0),
        "error": result.get("error"),
    }


@router_act.get("/act/telemetry")
async def act_telemetry(
    n: int = Query(default=10, ge=1, le=100),
    export: bool = Query(default=False),
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Session telemetry — CYNIC's learning measurement layer.
    """
    state = container.organism
    store = state.metabolism.telemetry_store

    if store is None:
        return {"stats": {}, "sessions": [], "message": "Telemetry store not initialized"}

    result = {
        "stats": store.stats(),
        "sessions": store.export() if export else store.recent(n),
        "message": f"*sniff* {len(store)} sessions measured",
    }
    return result
