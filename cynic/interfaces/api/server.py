"""
CYNIC Kernel API - FastAPI Gateway.

Unified entry point for all CYNIC interactions.
No logic resides here; it only exposes the Organism via HTTP.
"""

from __future__ import annotations

import logging
import uuid
from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from cynic.interfaces.api.error_handler import setup_error_handlers
from cynic.interfaces.api.metrics import router_metrics
from cynic.interfaces.api.routers.act import router_act
from cynic.interfaces.api.routers.actions import router as router_actions
from cynic.interfaces.api.routers.chat import router as router_chat
from cynic.interfaces.api.routers.core import router_core
from cynic.interfaces.api.routers.health import router_health
from cynic.interfaces.api.routers.introspection import router_introspection
from cynic.interfaces.api.routers.mcp import router as router_mcp
from cynic.interfaces.api.routers.nervous import router as router_nervous
from cynic.interfaces.api.routers.organism import router as router_organism
from cynic.interfaces.api.routers.sovereignty import router as router_sovereignty
from cynic.interfaces.api.state import (
    AppContainer,
    get_app_container,
    set_app_container,
)
from cynic.kernel.organism.factory import awaken

logger = logging.getLogger("cynic.interfaces.api.server")


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncGenerator[None, None]:
    """Manages the lifecycle of the CYNIC Organism within FastAPI."""
    instance_id = f"API-{uuid.uuid4().hex[:8]}"

    # Initialize the Organism (Industrial Awakening)
    try:
        logger.info("CYNIC Awakening (instance=%s)...", instance_id)
        organism = await awaken()
        await organism.start()

        # Set the global app container
        container = AppContainer(organism=organism)
        set_app_container(container)

        logger.info("CYNIC Organism is ALIVE and serving API.")
        yield
    finally:
        logger.info("CYNIC Hibernation (instance=%s)...", instance_id)
        container = get_app_container()
        if container and container.organism:
            await container.organism.stop()
        logger.info("CYNIC Organism is DORMANT.")


def create_app() -> FastAPI:
    """Creates and configures the FastAPI application."""
    app = FastAPI(
        title="CYNIC Kernel API",
        version="3.0.0",
        lifespan=lifespan,
    )

    # CORS configuration
    app.add_middleware(
        CORSMiddleware,
        allow_origins=["*"],
        allow_credentials=True,
        allow_methods=["*"],
        allow_headers=["*"],
    )

    # Error Handlers
    setup_error_handlers(app)

    # Routers
    app.include_router(router_core)
    app.include_router(router_health)
    app.include_router(router_organism)
    app.include_router(router_introspection)
    app.include_router(router_nervous)
    app.include_router(router_act)
    app.include_router(router_chat)
    app.include_router(router_mcp)
    app.include_router(router_actions)
    app.include_router(router_sovereignty)
    app.include_router(router_metrics)

    return app


app = create_app()
