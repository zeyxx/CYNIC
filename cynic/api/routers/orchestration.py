"""
Orchestration API Routes

POST /orchestration/build      — Build Docker image
POST /orchestration/deploy     — Deploy services
POST /orchestration/health     — Health check
GET  /orchestration/status     — Status overview
POST /orchestration/stop       — Stop services
POST /orchestration/release    — Create release
"""
from __future__ import annotations

import logging

from fastapi import APIRouter, HTTPException, Depends
from pydantic import BaseModel

from cynic.orchestration import DockerManager, VersionManager, HealthMonitor
from typing import Optional

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api/orchestration", tags=["orchestration"])


# ════════════════════════════════════════════════════════════════════════════
# MODELS
# ════════════════════════════════════════════════════════════════════════════


class BuildRequest(BaseModel):
    """Build request."""
    version: str = "latest"
    services: Optional[list[str]] = None


class BuildResponse(BaseModel):
    """Build response."""
    success: bool
    image: str
    version: str
    timestamp: str
    output: str
    error: Optional[str] = None


class DeployRequest(BaseModel):
    """Deploy request."""
    environment: str = "dev"  # dev, staging, prod
    pull: bool = True


class DeployResponse(BaseModel):
    """Deploy response."""
    success: bool
    services: list[str]
    timestamp: str
    duration_seconds: float
    error: Optional[str] = None


class HealthCheckResponse(BaseModel):
    """Health check response."""
    service: str
    status: str  # healthy, unhealthy, starting
    timestamp: str
    latency_ms: Optional[float] = None
    error: Optional[str] = None


class OrchestratorStatus(BaseModel):
    """Overall orchestrator status."""
    kernel_running: bool
    postgres_running: bool
    ollama_running: bool
    last_build: Optional[str] = None
    last_deploy: Optional[str] = None
    current_version: str


class ReleaseRequest(BaseModel):
    """Release request."""
    notes: str
    bump_type: str = "patch"  # patch, minor, major
    docker_image: Optional[str] = None


class ReleaseResponse(BaseModel):
    """Release response."""
    version: str
    timestamp: str
    notes: str
    status: str


# ════════════════════════════════════════════════════════════════════════════
# DEPENDENCIES
# ════════════════════════════════════════════════════════════════════════════


def get_docker_manager() -> DockerManager:
    """Get or create docker manager."""
    return DockerManager()


def get_version_manager() -> VersionManager:
    """Get or create version manager."""
    return VersionManager()


def get_health_monitor() -> HealthMonitor:
    """Get or create health monitor."""
    return HealthMonitor()


# ════════════════════════════════════════════════════════════════════════════
# ROUTES
# ════════════════════════════════════════════════════════════════════════════


@router.post("/build", response_model=BuildResponse)
async def build_image(
    request: BuildRequest,
    docker: DockerManager = Depends(get_docker_manager),
) -> BuildResponse:
    """
    Build Docker image for CYNIC kernel.

    **CYNIC builds itself.**
    """
    logger.info(f"Build requested: version={request.version}")

    result = await docker.build(version=request.version, services=request.services)

    return BuildResponse(
        success=result.success,
        image=result.image,
        version=result.version,
        timestamp=result.timestamp.isoformat(),
        output=result.output[-500:] if result.output else "",  # Last 500 chars
        error=result.error,
    )


@router.post("/deploy", response_model=DeployResponse)
async def deploy_services(
    request: DeployRequest,
    docker: DockerManager = Depends(get_docker_manager),
) -> DeployResponse:
    """
    Deploy CYNIC services via docker-compose.

    **CYNIC deploys itself.**
    """
    if request.environment not in ["dev", "staging", "prod"]:
        raise HTTPException(status_code=400, detail="Invalid environment")

    logger.info(f"Deploy requested: environment={request.environment}")

    result = await docker.deploy(environment=request.environment, pull=request.pull)

    return DeployResponse(
        success=result.success,
        services=result.services,
        timestamp=result.timestamp.isoformat(),
        duration_seconds=result.duration_seconds,
        error=result.error,
    )


@router.post("/health")
async def health_check(
    services: Optional[list[str]] = None,
    docker: DockerManager = Depends(get_docker_manager),
) -> list[HealthCheckResponse]:
    """
    Check health of CYNIC services.

    **CYNIC monitors itself.**
    """
    logger.info("Health check requested")

    checks = await docker.health_check(services=services)

    return [
        HealthCheckResponse(
            service=check.service,
            status=check.status,
            timestamp=check.timestamp.isoformat(),
            latency_ms=check.latency_ms,
            error=check.error,
        )
        for check in checks
    ]


@router.get("/status", response_model=OrchestratorStatus)
async def get_status(
    docker: DockerManager = Depends(get_docker_manager),
    version: VersionManager = Depends(get_version_manager),
) -> OrchestratorStatus:
    """
    Get overall orchestration status.
    """
    last_build = docker.last_build()
    last_deploy = docker.last_deploy()

    return OrchestratorStatus(
        kernel_running=True,  # TODO: Check actual status
        postgres_running=True,
        ollama_running=True,
        last_build=last_build.version if last_build else None,
        last_deploy=last_deploy.timestamp.isoformat() if last_deploy else None,
        current_version=str(version.get_current()),
    )


@router.post("/stop")
async def stop_services(
    docker: DockerManager = Depends(get_docker_manager),
) -> dict:
    """
    Stop all CYNIC services.

    **CYNIC can shut itself down.**
    """
    logger.warning("Stop requested")

    success = await docker.stop()

    return {
        "success": success,
        "message": "Services stopped" if success else "Stop failed",
    }


@router.post("/release", response_model=ReleaseResponse)
async def create_release(
    request: ReleaseRequest,
    version: VersionManager = Depends(get_version_manager),
    docker: DockerManager = Depends(get_docker_manager),
) -> ReleaseResponse:
    """
    Create a release.

    **CYNIC releases itself.**
    """
    logger.info(f"Release requested: bump_type={request.bump_type}")

    # Bump version
    if request.bump_type == "major":
        new_version = await version.bump_major()
    elif request.bump_type == "minor":
        new_version = await version.bump_minor()
    else:
        new_version = await version.bump_patch()

    # Get docker image name
    docker_image = request.docker_image or f"cynic-kernel:{new_version}"

    # Create release record
    release = await version.create_release(
        notes=request.notes,
        docker_image=docker_image,
        db_migrations=[],
    )

    return ReleaseResponse(
        version=release.version,
        timestamp=release.timestamp.isoformat(),
        notes=release.notes,
        status=release.status,
    )
