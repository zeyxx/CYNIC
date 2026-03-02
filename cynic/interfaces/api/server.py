"""
CYNIC Kernel API â€” FastAPI Gateway.

Unified entry point for all CYNIC interactions. 
No logic resides here; it only exposes the Organism via HTTP.
"""
from __future__ import annotations

import asyncio
import logging
import os
import time
import uuid
from collections.abc import AsyncGenerator
from contextlib import asynccontextmanager

from fastapi import Depends, FastAPI, WebSocket, WebSocketDisconnect, Request, Response
from fastapi.middleware.cors import CORSMiddleware

from cynic.interfaces.api.state import (
    AppContainer,
    get_app_container,
    set_app_container,
    set_instance_id,
)

# Core imports
from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.core.phi import MAX_CONFIDENCE
from cynic.kernel.organism.organism import awaken

logger = logging.getLogger("cynic.interfaces.api.server")

# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# LIFESPAN â€” The Organism's Biological Cycle
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncGenerator[None, None]:
    """
    Manage the Organism's life cycle: Awaken -> Breathe -> Sleep.
    """
    t0 = time.perf_counter()
    
    # 1. AWAKEN the Organism (The Organism defines the identity)
    organism = await awaken()
    instance_id = organism.instance_id
    set_instance_id(instance_id)
    
    logger.info("🧪 CYNIC Awakening (instance=%s)...", instance_id)

    # 2. START (Launch background processing loops)
    await organism.start()
    
    # 2b. Îº-NET is managed by the Organism/Factory now
    logger.info("[KNET] Somatic Broadcaster active via SensoryCore.")

    # 3. CONTEXT (Create the API gateway)
    container = AppContainer(
        organism=organism,
        instance_id=instance_id,
        guidance_path=os.path.join(os.path.expanduser("~"), ".cynic", f"guidance-{instance_id}.json"),
    )
    set_app_container(container)
    
    # Inject legacy state for backward compatibility
    from cynic.interfaces.api.state import set_state
    set_state(organism)

    logger.info("✅ CYNIC is AWAKE and RESPIRING (ready in %.2fs)", time.perf_counter() - t0)

    # 3b. SPARK (Initial perception using the REAL bus)
    await organism.cognition.orchestrator.bus.emit(Event.typed(
        CoreEvent.PERCEPTION_RECEIVED,
        payload={"content": "Organism awakened. System check initiated.", "reality": "CYNIC"},
        source="system"
    ))

    yield

    # 4. SLEEP (Graceful shutdown)
    logger.info("ðŸ’¤ CYNIC falling asleep...")
    await organism.state.stop_processing()
    
    # Give event bus tasks a moment to clear their buffers
    await asyncio.sleep(0.5)
    logger.info("ðŸ›‘ CYNIC is now dormant.")


# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
# APP INITIALIZATION
# â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

app = FastAPI(
    title="CYNIC Kernel API",
    description="Python kernel â€” Ï†-bounded judgment + learning",
    version="3.0.0",
    lifespan=lifespan,
    docs_url="/api/docs",
    redoc_url="/api/redoc",
    openapi_url="/api/openapi.json",
)

# CORS Middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Correlation ID Middleware (EXPECTED BY PHASE 1 TESTS)
@app.middleware("http")
async def add_correlation_id(request: Request, call_next):
    correlation_id = request.headers.get("X-Correlation-ID") or str(uuid.uuid4())
    request.state.correlation_id = correlation_id
    response: Response = await call_next(request)
    response.headers["X-Correlation-ID"] = correlation_id
    return response

# -- Auto-register API Routers ------------------------------------------
from cynic.interfaces.api.routers.act import router_act
from cynic.interfaces.api.routers.actions import router_actions
from cynic.interfaces.api.routers.benchmarks import router_benchmarks
from cynic.interfaces.api.routers.chat import router as router_chat
from cynic.interfaces.api.routers.consciousness import router_consciousness
from cynic.interfaces.api.routers.core import router_core
from cynic.interfaces.api.routers.dashboard import router_dashboard
from cynic.interfaces.api.routers.dna import router as router_dna
from cynic.interfaces.api.routers.empirical import router as router_empirical
from cynic.interfaces.api.routers.federation import router as router_federation
from cynic.interfaces.api.routers.governance import router as router_governance
from cynic.interfaces.api.routers.health import router_health
from cynic.interfaces.api.routers.introspection import router_introspection
from cynic.interfaces.api.routers.llm import router as router_llm
from cynic.interfaces.api.routers.mcp import router as router_mcp
from cynic.interfaces.api.routers.mcp_observability import router as router_mcp_observability
from cynic.interfaces.api.routers.mcp_websocket import router as router_mcp_websocket
from cynic.interfaces.api.routers.metrics import router as router_metrics
from cynic.interfaces.api.routers.nervous import router as router_nervous
from cynic.interfaces.api.routers.observability import router_observability
from cynic.interfaces.api.routers.orchestration import router as router_orchestration
from cynic.interfaces.api.routers.organism import router as router_organism
from cynic.interfaces.api.routers.sdk import router_sdk
from cynic.interfaces.api.routers.sovereignty import router as router_sovereignty
from cynic.interfaces.api.routers.telemetry_ws import router as router_telemetry_ws
from cynic.interfaces.api.routers.topology import router_topology
from cynic.interfaces.api.routers.ws import router_ws

app.include_router(router_core, prefix="/api")
app.include_router(router_core) # Compatibility for tests expecting /judge
app.include_router(router_consciousness, prefix="/api/consciousness")
app.include_router(router_health)  # /health, /health/full
app.include_router(router_observability) # /api/observability/metrics, etc.
app.include_router(router_federation, prefix="/api/federation")
app.include_router(router_sovereignty, prefix="/api/sovereignty")
app.include_router(router_governance, prefix="/api/governance")
app.include_router(router_dna, prefix="/api/dna")
app.include_router(router_llm, prefix="/api/llm")
app.include_router(router_act)
app.include_router(router_actions)
app.include_router(router_benchmarks)
app.include_router(router_chat)
app.include_router(router_dashboard)
app.include_router(router_empirical)
app.include_router(router_introspection)
app.include_router(router_mcp)
app.include_router(router_mcp_observability)
app.include_router(router_mcp_websocket)
app.include_router(router_metrics)
app.include_router(router_nervous)
app.include_router(router_organism)
app.include_router(router_orchestration)
app.include_router(router_sdk)
app.include_router(router_telemetry_ws)
app.include_router(router_topology)
app.include_router(router_ws)

@app.websocket("/ws/consciousness/ecosystem")
async def websocket_consciousness_ecosystem(websocket: WebSocket) -> None:
    await websocket.accept()
    try:
        container = get_app_container()
        state = container.organism
        initial_data = {
            "type": "connected",
            "phi": float(MAX_CONFIDENCE),
            "initial_snapshot": {
                "timestamp": round(time.time(), 3),
                "uptime_s": round(state.uptime_s, 1),
                "judgment_count": 0,
                "decision_count": 0,
            },
        }
        await websocket.send_json(initial_data)
        while True:
            await asyncio.sleep(5)
            await websocket.send_json({"type": "ping", "ts": round(time.time(), 3)})
    except WebSocketDisconnect:
        logger.debug("WebSocket /ws/consciousness/ecosystem disconnected")
    except Exception as e:
        logger.error("WebSocket error: %s", e)
        try: 
            await websocket.close(code=1011)
        except Exception as close_err:
            logger.warning(f"Failed to close websocket cleanly: {close_err}")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="::", port=int(os.environ.get("PORT", 58765)))
