"""
CYNIC Llama.cpp Server Adapter - Industrial APU Inference.
Optimized for Vulkan acceleration on Ryzen 5700G.
Directly implements Unsloth-style parameter management.
"""
from __future__ import annotations
import logging
import time
import httpx
from typing import Optional, Any, Dict
from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRequest, LLMResponse
from cynic.kernel.core.vascular import VascularSystem

logger = logging.getLogger("cynic.kernel.organism.brain.llm.llama_server")

class LlamaServerAdapter(LLMAdapter):
    """
    Adapter for a persistent llama-server process.
    Provides deep control over sampling and hardware-specific features.
    """
    def __init__(
        self,
        model: str,
        base_url: str = "http://localhost:8080",
        vascular: Optional[VascularSystem] = None,
    ):
        super().__init__(model=model, provider="llama_cpp_server", vascular=vascular)
        self.base_url = base_url

    async def complete(self, request: LLMRequest) -> LLMResponse:
        start = time.time()
        
        # Deep parameters extraction from metadata (set by ParameterGovernor)
        sampling_params = {
            "prompt": request.prompt,
            "temperature": request.temperature,
            "top_p": request.metadata.get("top_p", 0.95),
            "min_p": request.metadata.get("min_p", 0.05),
            "n_predict": request.max_tokens,
            "repeat_penalty": request.metadata.get("repeat_penalty", 1.1),
            "cache_prompt": True, # For efficiency during long maintenance
            "slot_id": -1, # Auto-allocate
            "stop": ["</s>", "Task:", "Analysis:"] + request.metadata.get("stop", [])
        }

        try:
            async with httpx.AsyncClient(timeout=300.0) as client:
                # Direct call to /completion endpoint of llama-server
                resp = await client.post(f"{self.base_url}/completion", json=sampling_params)
                
                if resp.status_code != 200:
                    raise RuntimeError(f"LlamaServer Error: {resp.text}")
                
                result = resp.json()
                content = result["content"]
                
                latency_ms = (time.time() - start) * 1000
                
                return LLMResponse(
                    content=content,
                    model=self.model,
                    provider="llama_cpp_server",
                    prompt_tokens=result.get("tokens_evaluated", 0),
                    completion_tokens=result.get("tokens_predicted", 0),
                    latency_ms=latency_ms
                )
        except Exception as e:
            logger.error(f"LlamaServer E2E failure: {e}")
            return LLMResponse(content="", model=self.model, provider="llama_cpp_server", error=str(e))

    async def check_available(self) -> bool:
        """Check if the server is alive and responding via /health."""
        try:
            async with httpx.AsyncClient(timeout=2.0) as client:
                resp = await client.get(f"{self.base_url}/health")
                return resp.status_code == 200
        except Exception:
            return False
