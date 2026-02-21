"""
MCP Resources Router

Exposes CYNIC's knowledge to Claude Code via MCP protocol.

GET /mcp/resource — Fetch specific MCP resource by URI
  - /mcp/judgments/similar — Find similar past judgments
  - /mcp/judgments/{id}/reasoning — Get judgment reasoning + DAG
  - /mcp/loops/status — Check if CYNIC is stuck
  - /mcp/learning/patterns — What has CYNIC learned?
  - /mcp/events/recent — Live event stream

This bridges L2 feedback loop: Claude Code ↔ CYNIC observation
"""
from __future__ import annotations

import logging
from fastapi import APIRouter, HTTPException, Depends, Query

from cynic.api.state import CynicOrganism, get_app_container, AppContainer
from cynic.mcp.resources import create_mcp_resources

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api/mcp", tags=["mcp"])


@router.get("/resource")
async def get_mcp_resource(
    uri: str = Query(..., description="MCP resource URI"),
    container: AppContainer = Depends(get_app_container),
) -> dict:
    """
    Get MCP resource by URI.

    Supported URIs:
      - /mcp/judgments/similar (find similar past judgments)
      - /mcp/judgments/{judgment_id}/reasoning (full judgment DAG + dog votes)
      - /mcp/loops/status (feedback loop health check)
      - /mcp/learning/patterns (learned verdict/dog patterns)
      - /mcp/events/recent (live event stream)
    """
    try:
        if not uri.startswith("/mcp/"):
            raise HTTPException(
                status_code=400,
                detail="URI must start with /mcp/",
            )

        manager = create_mcp_resources(container.organism)
        result = await manager.get_resource(uri)

        if "error" in result:
            raise HTTPException(status_code=404, detail=result["error"])

        return result

    except HTTPException:
        raise
    except httpx.RequestError as e:
        logger.error(f"Error fetching MCP resource {uri}: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/similar-judgments")
async def get_similar_judgments(
    q_score: float = Query(75.0, description="Target Q-Score"),
    verdict: str = Query("WAG", description="Target verdict (BARK/GROWL/WAG/HOWL)"),
    limit: int = Query(10, ge=1, le=50, description="Max results"),
    container: AppContainer = Depends(get_app_container),
) -> dict:
    """Find judgments similar to a given pattern."""
    try:
        manager = create_mcp_resources(container.organism)
        return await manager.get_similar_judgments(q_score, verdict, limit)
    except httpx.RequestError as e:
        logger.error(f"Error fetching similar judgments: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/judgment/{judgment_id}/reasoning")
async def get_judgment_reasoning(
    judgment_id: str,
    container: AppContainer = Depends(get_app_container),
) -> dict:
    """Get full reasoning for a judgment (DAG + dog votes + reasoning)."""
    try:
        manager = create_mcp_resources(container.organism)
        result = await manager.get_judgment_reasoning(judgment_id)

        if "error" in result:
            raise HTTPException(status_code=404, detail=result["error"])

        return result
    except HTTPException:
        raise
    except httpx.RequestError as e:
        logger.error(f"Error fetching judgment reasoning: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/loops/status")
async def get_loop_status(
    container: AppContainer = Depends(get_app_container),
) -> dict:
    """Check if CYNIC's feedback loops are healthy (is it stuck?)."""
    try:
        manager = create_mcp_resources(container.organism)
        return await manager.get_loop_status()
    except ValidationError as e:
        logger.error(f"Error checking loop status: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/learning/patterns")
async def get_learned_patterns(
    limit: int = Query(20, ge=1, le=100, description="Analysis window size"),
    container: AppContainer = Depends(get_app_container),
) -> dict:
    """Get patterns CYNIC has learned (verdict distribution, dog performance, etc)."""
    try:
        manager = create_mcp_resources(container.organism)
        return await manager.get_learned_patterns(limit)
    except httpx.RequestError as e:
        logger.error(f"Error fetching learned patterns: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/events/recent")
async def get_recent_events(
    since_ms: float = Query(None, description="Only events since this timestamp"),
    limit: int = Query(50, ge=1, le=500, description="Max events to return"),
    container: AppContainer = Depends(get_app_container),
) -> dict:
    """Get recent event stream for live monitoring."""
    try:
        manager = create_mcp_resources(container.organism)
        return await manager.get_event_stream(since_ms, limit)
    except httpx.RequestError as e:
        logger.error(f"Error fetching recent events: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/hypergraph/recent")
async def get_hypergraph_edges(
    limit: int = Query(50, ge=1, le=500, description="Max hyper-edges to return"),
    container: AppContainer = Depends(get_app_container),
) -> dict:
    """Get recent 7-dimensional hyper-edges linking perception->cognition->action."""
    try:
        manager = create_mcp_resources(container.organism)
        return await manager.get_hypergraph_edges(limit)
    except httpx.RequestError as e:
        logger.error(f"Error fetching hypergraph edges: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/health")
async def mcp_health(
    container: AppContainer = Depends(get_app_container),
) -> dict:
    """Check MCP bridge health."""
    try:
        manager = create_mcp_resources(container.organism)
        loop_status = await manager.get_loop_status()
        return {
            "status": "healthy",
            "mcp_bridge": "operational",
            "loop_health": loop_status.get("health", {}),
        }
    except ValidationError as e:
        logger.error(f"MCP health check failed: {e}")
        return {
            "status": "unhealthy",
            "mcp_bridge": "degraded",
            "error": str(e),
        }
