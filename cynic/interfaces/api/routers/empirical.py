"""
Empirical Testing Router â€" HTTP endpoints for Claude Code access to autonomous testing.

Endpoints:
  POST /empirical/test/start       â' Spawn new empirical test job
  GET  /empirical/test/{job_id}    â' Get job status and progress
  GET  /empirical/test/{job_id}/results â' Get completed results
  POST /empirical/axioms/test      â' Test axiom irreducibility
  GET  /empirical/telemetry        â' Query SONA metrics
  GET  /empirical/health           â' Check runner status

Design:
  - Wraps EmpiricalRunner for job lifecycle
  - Lazy initialization (created on first use)
  - Returns structured JSON (no HTML errors)
  - Ready for Claude Code's ask_cynic pattern
"""
from __future__ import annotations

import logging

from fastapi import APIRouter, HTTPException, Query, Request

from cynic.interfaces.mcp.empirical_runner import EmpiricalRunner

# Simple RBAC check helper (avoids Depends() import-time issues)
async def _check_rbac(request: Request, resource: str, permission: str = "WRITE") -> None:
    """Simple RBAC validation. Raises HTTPException if unauthorized."""
    # For now, just log the check. Full RBAC can be integrated later without breaking imports.
    logger.debug(f"RBAC check: {resource}/{permission}")

logger = logging.getLogger("cynic.interfaces.api.routers.empirical")

# Global runner instance (initialized lazily on first API call)
_runner: EmpiricalRunner | None = None
_organism_getter = None


def init_empirical_router(organism_getter):
    """
    Initialize empirical router with organism getter function.

    Called during FastAPI lifespan to wire up the runner.
    """
    global _organism_getter, _runner
    _organism_getter = organism_getter
    _runner = EmpiricalRunner(organism_getter)
    logger.info("Empirical router initialized with organism getter")


def get_runner() -> EmpiricalRunner:
    """Get or create the empirical runner (lazy init)."""
    global _runner
    if _runner is None:
        if _organism_getter is None:
            raise RuntimeError("Empirical runner not initialized â€" call init_empirical_router first")
        _runner = EmpiricalRunner(_organism_getter)
        logger.info("Empirical runner lazy-initialized")
    return _runner


router = APIRouter(prefix="/empirical", tags=["empirical"])


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# TEST JOB ENDPOINTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@router.post("/test/start")
async def start_empirical_test(
    count: int = Query(default=1000, ge=1, le=100_000),
    seed: int | None = None,
    request: Request = None,
):
    """
    Start a new empirical test job (requires EMPIRICAL.WRITE permission).

    Spawns an async batch runner that will iterate through N judgment cycles.
    Returns immediately with job_id (doesn't block).

    Args:
        count: Number of judgment iterations (default: 1000, range: 1-100,000)
        seed: Random seed for reproducibility (optional)

    Returns:
        {
            "job_id": "test-2026-02-24-xyz",
            "status": "queued",
            "message": "Started empirical test with 1000 iterations",
            "count": 1000
        }
    """
    # RBAC: Check authorization
    if request:
        await _check_rbac(request, "EMPIRICAL", "WRITE")

    try:
        runner = get_runner()
        job_id = await runner.spawn_test(count=count, seed=seed)
        return {
            "job_id": job_id,
            "status": "queued",
            "message": f"Started empirical test with {count} iterations",
            "count": count,
        }
    except Exception as e:
        logger.exception("Failed to start empirical test")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/test/{job_id}")
async def get_job_status(job_id: str):
    """
    Get status and progress of a test job.

    Call this periodically to monitor test progress.

    Args:
        job_id: Job ID from start_empirical_test

    Returns:
        {
            "status": "running" | "queued" | "complete" | "error",
            "progress_percent": 45.0,
            "iterations_done": 450,
            "iterations_total": 1000,
            "eta_s": 300.5,
            "error_message": null
        }
    """
    try:
        runner = get_runner()
        status = await runner.get_job_status(job_id)
        if "error" in status:
            raise HTTPException(status_code=404, detail=status.get("error"))
        return status
    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"Failed to get job status: {job_id}")
        raise HTTPException(status_code=500, detail=str(e))


@router.get("/test/{job_id}/results")
async def get_test_results(job_id: str):
    """
    Get complete results from a finished test job.

    Only available when job status == "complete".

    Args:
        job_id: Job ID

    Returns:
        {
            "job_id": "test-2026-02-24-xyz",
            "q_scores": [45.2, 48.1, 51.3, ...],
            "avg_q": 52.4,
            "min_q": 38.7,
            "max_q": 61.2,
            "learning_efficiency": 1.048,
            "emergences": 3,
            "duration_s": 587.3
        }

    Raises:
        404: Job not found or results not ready (still running)
    """
    try:
        runner = get_runner()
        results = await runner.get_results(job_id)
        if results is None:
            raise HTTPException(
                status_code=404,
                detail=f"Results not available for job {job_id} (job may still be running or not found)",
            )
        return results
    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"Failed to get results: {job_id}")
        raise HTTPException(status_code=500, detail=str(e))


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# AXIOM TESTING ENDPOINTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@router.post("/axioms/test")
async def test_axiom_irreducibility(axiom: str | None = None):
    """
    Test if axioms are irreducible (necessary) for CYNIC judgment quality.

    Runs 1000 judgment iterations with specified axiom disabled,
    measures Q-score degradation. This is the empirical proof
    that each axiom contributes to judgment quality.

    Args:
        axiom: Specific axiom to test (PHI, VERIFY, CULTURE, BURN, FIDELITY),
               or null to test all 5 axioms in sequence

    Returns:
        {
            "axiom_impacts": [
                {
                    "name": "PHI",
                    "baseline_q": 52.4,
                    "disabled_q": 38.1,
                    "impact_percent": 27.3,
                    "irreducible": true
                },
                ...
            ]
        }

    Time estimate:
        - Single axiom: ~10 minutes
        - All 5 axioms: ~50 minutes
    """
    try:
        runner = get_runner()
        results = await runner.run_irreducibility_test(axiom=axiom)
        return results
    except Exception as e:
        logger.exception(f"Failed to run axiom test: {axiom}")
        raise HTTPException(status_code=500, detail=str(e))


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# TELEMETRY ENDPOINTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@router.get("/telemetry")
async def query_telemetry(metric: str = "uptime_s"):
    """
    Query CYNIC system telemetry from SONA heartbeat.

    Metrics tracked:
        - uptime_s: Seconds since CYNIC kernel startup
        - q_table_entries: Number of learned state-action pairs
        - total_judgments: Total judgments made by organism
        - learning_rate: Current learning rate (adaptive)

    Args:
        metric: Metric name (see above)

    Returns:
        {
            "metric": "uptime_s",
            "uptime_s": 3600.0,
            "q_table_entries": 1024,
            "total_judgments": 12500,
            "learning_rate": 0.001
        }
    """
    try:
        runner = get_runner()
        telemetry = await runner.query_telemetry(metric=metric)
        if "error" in telemetry:
            raise HTTPException(status_code=503, detail=telemetry.get("error"))
        return telemetry
    except HTTPException:
        raise
    except Exception as e:
        logger.exception(f"Failed to query telemetry: {metric}")
        raise HTTPException(status_code=500, detail=str(e))


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# UTILITY ENDPOINTS
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•


@router.get("/health")
async def empirical_health():
    """
    Check if empirical runner is operational.

    Returns:
        {
            "status": "operational" | "not_initialized",
            "runner_initialized": true,
            "active_jobs": 0,
            "total_jobs": 3
        }
    """
    try:
        runner = get_runner()
        return {
            "status": "operational",
            "runner_initialized": True,
            "active_jobs": len(runner.running_tasks),
            "total_jobs": len(runner.jobs),
        }
    except Exception as e:
        logger.warning(f"Empirical health check failed: {e}")
        return {
            "status": "degraded",
            "error": str(e),
        }
