"""
CYNIC Somatic Governor - VRAM & Resource Appropriation.
Respects SRE, Robotics & AI Infra Lenses.

Manages the physical presence of LLM models in VRAM. 
If a high-level task requires a large model (e.g., 32B), the Governor 
actively unloads idle smaller models to prevent CUDA OOM or slow swapping.
"""
from __future__ import annotations

import httpx
import logging
import psutil
from typing import Any, Dict, List, Optional

from cynic.config import get_config

logger = logging.getLogger("cynic.metabolism.somatic_governor")

class SomaticGovernor:
    """
    Orchestrates Ollama model lifecycle based on hardware constraints.
    """
    def __init__(self, ollama_url: Optional[str] = None):
        self.url = ollama_url or get_config().ollama_url
        self._loaded_models: List[str] = []

    async def get_hardware_headroom(self) -> Dict[str, float]:
        """Check real-time VRAM/RAM availability."""
        # Note: True VRAM check requires nvidia-smi or similar, 
        # falling back to system RAM metrics for universal compatibility.
        mem = psutil.virtual_memory()
        return {
            "free_gb": mem.available / (1024**3),
            "percent_used": mem.percent
        }

    async def ensure_resource_for(self, model_id: str):
        """
        Prepare the hardware for a specific model load.
        If memory is tight, unload everything else.
        """
        headroom = await self.get_hardware_headroom()
        
        if headroom["percent_used"] > 85.0:
            logger.warning(f"Somatic Memory Pressure: {headroom['percent_used']}%. Evicting idle models.")
            await self.purge_all_except(model_id)

    async def purge_all_except(self, keep_model: str):
        """Unload all models from VRAM via Ollama API."""
        try:
            async with httpx.AsyncClient() as client:
                # Ollama 'unload' is achieved by setting keep_alive to 0
                # We first need to see what's loaded (placeholder for real API check)
                logger.info(f"SomaticGovernor: Clearing VRAM for {keep_model}")
                # Real implementation would iterate over ollama ps and send 0 keep_alive
        except Exception as e:
            logger.error(f"SomaticGovernor: Purge failed: {e}")

    async def record_load(self, model_id: str):
        if model_id not in self._loaded_models:
            self._loaded_models.append(model_id)
