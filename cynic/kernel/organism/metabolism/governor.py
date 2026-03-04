"""
CYNIC Metabolic Governor - Resource Appropriation & Body Budget.
Ensures the organism doesn't burn out by monitoring and managing hardware state.
Vascularized implementation using the central connection pool.
"""

from __future__ import annotations

import logging
from abc import ABC, abstractmethod
from typing import Any, Dict, List

import psutil

from cynic.kernel.core.vascular import VascularSystem

logger = logging.getLogger("cynic.organism.metabolism")


class ResourceProvider(ABC):
    @abstractmethod
    async def get_loaded_models(self) -> List[str]: ...

    @abstractmethod
    async def unload_model(self, model_id: str) -> bool: ...


class OllamaProvider(ResourceProvider):
    """Implementation for Ollama-based infrastructure using VascularSystem."""

    def __init__(
        self, vascular: VascularSystem, base_url: str = "http://localhost:11434"
    ):
        self.vascular = vascular
        self.base_url = base_url

    async def get_loaded_models(self) -> List[str]:
        try:
            client = await self.vascular.get_client()
            resp = await client.get(f"{self.base_url}/api/ps")
            if resp.status_code == 200:
                models = resp.json().get("models", [])
                return [str(m["name"]) for m in models]
        except Exception as e:
            logger.error(f"OllamaProvider: Failed to list models: {e}")
        return []

    async def unload_model(self, model_id: str) -> bool:
        try:
            client = await self.vascular.get_client()
            # Unload by setting keep_alive to 0
            resp = await client.post(
                f"{self.base_url}/api/generate",
                json={"model": model_id, "keep_alive": 0},
            )
            return resp.status_code == 200
        except Exception as e:
            logger.error(f"OllamaProvider: Failed to unload {model_id}: {e}")
        return False


class MetabolicGovernor:
    """
    Manages the 'Body Budget' of the agent.
    Prevents OOM and ensures hardware headroom.
    """

    def __init__(self, provider: ResourceProvider, vram_threshold: float = 85.0):
        self.provider = provider
        self.vram_threshold = vram_threshold  # Percent

    async def check_health(self) -> Dict[str, Any]:
        """Returns the current metabolic state."""
        mem = psutil.virtual_memory()
        return {
            "ram_percent": mem.percent,
            "ram_available_gb": mem.available / (1024**3),
            "is_stressed": bool(mem.percent > self.vram_threshold),
        }

    async def ensure_headroom(self, required_model: str) -> None:
        """If stressed, unloads idle models to make room for the new one."""
        state = await self.check_health()
        if state["is_stressed"]:
            logger.warning(
                f"Metabolic Stress detected ({state['ram_percent']}%). Evicting idle models..."
            )
            loaded = await self.provider.get_loaded_models()
            for model in loaded:
                if model != required_model:
                    logger.info(
                        f"Evicting {model} to free resources for {required_model}"
                    )
                    await self.provider.unload_model(model)

    async def allocate(self, model_id: str) -> None:
        """Requests resource allocation for a model."""
        await self.ensure_headroom(model_id)
        logger.info(f"Metabolic Allocation granted for {model_id}")
