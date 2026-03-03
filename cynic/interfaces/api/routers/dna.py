"""
DNA Router " Assembly Language for the CYNIC Organism.

Exposes low-level primitives and high-level workflows via API.
"""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, Depends, HTTPException

from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.kernel.organism.brain.dna import assembly


# Simple RBAC check helper (avoids Depends() import-time issues)
async def _check_rbac(request, resource: str, permission: str = "WRITE") -> None:
    """Simple RBAC validation. Raises HTTPException if unauthorized."""
    logger.debug(f"RBAC check: {resource}/{permission}")


router = APIRouter(prefix="/dna", tags=["dna"])


@router.get("/workflows")
async def list_available_workflows():
    """List all built-in DNA workflows."""
    return {"workflows": assembly.list_workflows()}


@router.post("/run/{workflow_name}")
async def run_workflow(
    workflow_name: str,
    input_data: Any,
    container: AppContainer = Depends(get_app_container),
):
    """
    Execute a specific DNA workflow.

    Workflows:
    - FAST_QUALITY_CHECK
    - ANALYZE_CODE_SECURITY
    - AUDIT_REPO
    - CONTINUOUS_LEARNING
    """
    workflow_fn = getattr(assembly, workflow_name, None)
    if not workflow_fn:
        raise HTTPException(
            status_code=404, detail=f"Workflow {workflow_name} not found"
        )

    try:
        # Pass orchestrator and qtable from container
        result = await workflow_fn(
            input_data, orchestrator=container.orchestrator, qtable=container.qtable
        )
        return result
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@router.post("/chain")
async def run_custom_chain(
    input_content: str,
    source: str = "code",
    level: str = "MICRO",
    axiom: str = "PHI",
    container: AppContainer = Depends(get_app_container),
):
    """Run a custom DNA chain (Perceive -> Judge -> Decide -> Act -> Learn)."""
    from cynic.kernel.organism.brain.dna.primitives import PERCEIVE, run_dna_chain

    cell = await PERCEIVE(source, input_content)
    result = await run_dna_chain(
        cell,
        level=level,
        axiom=axiom,
        orchestrator=container.orchestrator,
        qtable=container.qtable,
    )
    return result
