"""
LLM Router — Cognitive Inventory Control.

Exposes the LLM manifest and allows dynamic rescanning of the 
Organism's muscles.
"""
from __future__ import annotations
from typing import Any, Dict, List, Optional
from fastapi import APIRouter, Depends, HTTPException

from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.brain.llm.adapter import get_registry

router = APIRouter(prefix="/brain/llm", tags=["llm"])

@router.get("/manifest")
async def get_llm_manifest(container: AppContainer = Depends(get_app_container)):
    """Get the current discovery manifest (Hardware status + model fit)."""
    registry = get_registry()
    if not registry._manifest:
        # If not discovered yet, trigger one
        from cynic.kernel.core.config import CynicConfig
        config = CynicConfig.from_env()
        await registry.discover(
            ollama_url=config.ollama_url,
            claude_api_key=config.anthropic_api_key,
            models_dir=config.models_dir
        )
    return registry._manifest

@router.post("/rescan")
async def rescan_muscles(container: AppContainer = Depends(get_app_container)):
    """Force a new hardware-aware discovery cycle."""
    from cynic.kernel.core.config import CynicConfig
    config = CynicConfig.from_env()
    registry = get_registry()
    
    manifest = await registry.discover(
        ollama_url=config.ollama_url,
        claude_api_key=config.anthropic_api_key,
        models_dir=config.models_dir
    )
    return {"status": "SUCCESS", "manifest": manifest}

@router.get("/best/{dog_id}/{task_type}")
async def get_routing_decision(dog_id: str, task_type: str):
    """Peek into the routing logic: which muscle would be used?"""
    registry = get_registry()
    adapter = registry.get_best_for(dog_id, task_type)
    if not adapter:
        raise HTTPException(status_code=404, detail="No suitable muscle found")
    return {
        "dog_id": dog_id,
        "task_type": task_type,
        "selected_adapter": adapter.adapter_id,
        "provider": adapter.provider,
        "model": adapter.model
    }
