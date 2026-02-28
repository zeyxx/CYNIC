"""
CYNIC Kernel API — FastAPI Gateway.

Unified entry point for all CYNIC interactions. 
No logic resides here; it only exposes the Organism via HTTP.
"""
from __future__ import annotations

import asyncio
import logging
import os
import sys
import time
from contextlib import asynccontextmanager
from typing import AsyncGenerator, Optional, Any

from fastapi import FastAPI, Depends
from fastapi.middleware.cors import CORSMiddleware

# Core imports
from cynic.kernel.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.kernel.core.phi import MAX_CONFIDENCE
from cynic.kernel.organism.organism import Organism, awaken
from cynic.interfaces.api.state import (
    AppContainer,
    set_app_container,
    get_app_container,
    set_instance_id,
)

logger = logging.getLogger("cynic.interfaces.api.server")

# ════════════════════════════════════════════════════════════════════════════
# LIFESPAN — The Organism's Biological Cycle
# ════════════════════════════════════════════════════════════════════════════

@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncGenerator[None, None]:
    """
    Manage the Organism's life cycle: Awaken -> Breathe -> Sleep.
    """
    t0 = time.perf_counter()
    instance_id = os.environ.get("CYNIC_INSTANCE_ID", os.urandom(4).hex())
    set_instance_id(instance_id)
    
    logger.info("🧬 CYNIC Awakening (instance=%s)...", instance_id)

    # 1. AWAKEN the Organism (Load components and wire nervous system)
    # Registry can be passed here if LLMs are enabled
    organism = awaken()
    
    # 2. BREATHE (Start async processing loops for state and sensors)
    # Note: In production, we would pass the actual DB pool here
    await organism.state.start_processing(db=None)
    
    # 3. CONTEXT (Create the API gateway)
    container = AppContainer(
        organism=organism,
        instance_id=instance_id,
        guidance_path=os.path.join(os.path.expanduser("~"), ".cynic", f"guidance-{instance_id}.json"),
    )
    set_app_container(container)
    
    # Inject legacy state for backward compatibility with old routes
    from cynic.interfaces.api.state import set_state
    set_state(organism)

    logger.info("✅ CYNIC is AWAKE and RESPIRING (ready in %.2fs)", time.perf_counter() - t0)

    yield

    # 4. SLEEP (Graceful shutdown)
    logger.info("💤 CYNIC falling asleep...")
    await organism.state.stop_processing()
    
    # Give event bus tasks a moment to clear their buffers
    await asyncio.sleep(0.5)
    logger.info("🛑 CYNIC is now dormant.")


# ════════════════════════════════════════════════════════════════════════════
# APP INITIALIZATION
# ════════════════════════════════════════════════════════════════════════════

app = FastAPI(
    title="CYNIC Kernel API",
    description="Python kernel — φ-bounded judgment + learning",
    version="3.0.0",  # Major version jump for unified architecture
    lifespan=lifespan,
)

# CORS Middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── Auto-register API Routers ────────────────────────────────────────────
# We import them here to ensure they are registered with the app
from cynic.interfaces.api.routers.core import router_core
from cynic.interfaces.api.routers.consciousness import router_consciousness
from cynic.interfaces.api.routers.health import router_health
from cynic.interfaces.api.routers.federation import router as router_federation
from cynic.interfaces.api.routers.sovereignty import router as router_sovereignty
from cynic.interfaces.api.routers.governance import router as router_governance
from cynic.interfaces.api.routers.dna import router as router_dna
from cynic.interfaces.api.routers.llm import router as router_llm

app.include_router(router_core)
app.include_router(router_consciousness)
app.include_router(router_health)
app.include_router(router_federation)
app.include_router(router_sovereignty)
app.include_router(router_governance)
app.include_router(router_dna)
app.include_router(router_llm)

@app.get("/")
async def root():
    """System heartbeat."""
    return {
        "status": "AWAKE",
        "phi": 1.618033988749895,
        "confidence_cap": 0.618,
        "timestamp": time.time()
    }

if __name__ == "__main__":
    import uvicorn
    port = int(os.environ.get("PORT", 8765))
    uvicorn.run(app, host="0.0.0.0", port=port)
