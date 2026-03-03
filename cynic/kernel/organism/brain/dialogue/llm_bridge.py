"""Universal LLM bridge for natural language generation.

Routes through LLMRegistry for multi-provider support:
- Local: Ollama, LlamaCpp
- CLI: claude, gemini
- Cloud: Anthropic, Google, etc.

Implements sovereignty-first routing: prefers local models, falls back to cloud.
"""

from __future__ import annotations

import logging
from typing import Any

from cynic.kernel.organism.brain.llm.adapter import (
    LLMAdapter,
    LLMRegistry,
    LLMRequest,
)

logger = logging.getLogger("cynic.kernel.organism.brain.dialogue.llm_bridge")


class LLMBridge:
    """Multi-provider LLM router for dialogue responses.

    Routes through LLMRegistry for sovereignty-first provider selection.
    Falls back through available adapters on failure.

    Backward compatible: accepts api_key for Anthropic fallback.
    New code should inject registry via DI (factory).
    """

    def __init__(
        self,
        registry: LLMRegistry | None = None,
        api_key: str | None = None,
        dog_id: str = "CYNIC",
    ):
        """Initialize LLM bridge with registry-based routing.

        Args:
            registry: LLMRegistry instance (injected from factory or test).
                      If None, will attempt to retrieve from app container.
            api_key: Optional API key for fallback (deprecated; use factory injection).
                     Only used if registry unavailable.
            dog_id: Dog ID for routing decisions (default: CYNIC for dialogue).
        """
        self.registry = registry
        self.api_key = api_key
        self.dog_id = dog_id
        self._adapter: LLMAdapter | None = None
        self._fallback_adapters: list[LLMAdapter] = []

    async def _ensure_adapter(self) -> LLMAdapter | None:
        """Lazily select best adapter on first use (sovereignty-first routing).

        Returns:
            Selected LLMAdapter, or None if no adapters available.
        """
        if self._adapter is not None:
            return self._adapter

        # Attempt to get registry if not provided
        if not self.registry:
            try:
                from cynic.kernel.organism.brain.llm.adapter import get_registry

                self.registry = get_registry()
            except Exception as e:
                logger.debug("Could not retrieve registry from container: %s", e)
                return None

        # Sovereignty-first routing: llama_cpp > ollama > cli > cloud
        self._adapter = self.registry.get_best_for(
            dog_id=self.dog_id, task_type="dialogue"
        )

        # Pre-cache fallback options
        if self._adapter:
            available = self.registry.get_available_for_generation()
            self._fallback_adapters = [
                a for a in available if a.adapter_id != self._adapter.adapter_id
            ]
            logger.info(
                "Selected adapter: %s (fallbacks: %d available)",
                self._adapter.adapter_id,
                len(self._fallback_adapters),
            )

        return self._adapter

    async def generate_response(self, context: dict[str, Any]) -> str:
        """Generate response using registry-routed adapter.

        Args:
            context: Reasoning context from CYNIC (verdict, confidence, axiom_scores).

        Returns:
            Natural language explanation string.
        """
        # 1. Check if we should use Remote Proxy (Docker)
        from cynic.kernel.observability.symbiotic_state_manager import (
            get_symbiotic_state_manager,
        )

        try:
            mgr = await get_symbiotic_state_manager()
            if mgr.remote_mode and not self.api_key:
                return await self._proxy_to_remote(context, mgr.api_url)
        except Exception:
            # Remote proxy unavailable, continue with local routing
            pass

        # 2. Route through LLMRegistry (multi-provider)
        adapter = await self._ensure_adapter()
        if not adapter:
            return "I'm unable to explain my reasoning: No LLM available and no remote instance detected."

        try:
            prompt = self._create_explanation_prompt(context)

            # Convert to unified LLMRequest
            request = LLMRequest(
                prompt=prompt,
                system="You are CYNIC, an AI organism that judges proposals based on five axioms.",
                max_tokens=256,
                temperature=0.0,
            )

            # Call adapter (unified interface)
            response = await adapter.complete_safe(request)

            if response.is_success:
                return response.content
            else:
                # Try fallback adapter
                return await self._try_fallback(request)

        except Exception as e:
            logger.exception("Error in generate_response: %s", e)
                return f"I'm unable to explain my reasoning right now: {str(e)[:100]}"

    async def _try_fallback(self, request: LLMRequest) -> str:
        """Try next available adapter if primary fails.

        Args:
            request: LLMRequest to send to fallback adapters.

        Returns:
            Response content from first successful adapter, or error message.
        """
        for fallback in self._fallback_adapters:
            try:
                response = await fallback.complete_safe(request)
                if response.is_success:
                    self._adapter = fallback  # Remember working adapter
                    logger.info("Fallback adapter succeeded: %s", fallback.adapter_id)
                    return response.content
            except Exception as e:
                logger.debug(
                    "Fallback adapter %s failed: %s", fallback.adapter_id, str(e)[:100]
                )
                continue

        return "I'm unable to explain my reasoning: All available LLMs failed."

    async def _proxy_to_remote(self, context: dict[str, Any], api_url: str) -> str:
        """Forward dialogue request to the remote container API."""
        import httpx

        try:
            async with httpx.AsyncClient(timeout=30.0) as client:
                # We send the raw user question to the remote /dialogue endpoint
                # which has its own full context and LLM credentials.
                payload = {"text": context.get("question", "")}
                resp = await client.post(f"{api_url}/api/dialogue", json=payload)
                if resp.status_code == 200:
                    return resp.json().get("response", "No response from remote.")
                return f"Remote dialogue failed (Status {resp.status_code})"
        except Exception as e:
            return f"Remote proxy error: {str(e)[:100]}"

    def _create_explanation_prompt(self, context: dict[str, Any]) -> str:
        """Create prompt for Claude to explain reasoning."""
        question = context.get("question", "")
        verdict = context.get("verdict", "UNKNOWN")
        confidence = context.get("confidence", 0)
        reasoning_summary = context.get("reasoning_summary", "")
        axiom_scores = context.get("axiom_scores", {})
        communication_style = context.get("communication_style", "balanced")
        verbosity = context.get("verbosity", "2-3 sentences")

        # Format axiom info
        axiom_info = (
            "\n".join(
                [
                    f"- {axiom}: {score:.1%}"
                    for axiom, score in sorted(
                        axiom_scores.items(), key=lambda x: x[1], reverse=True
                    )
                ]
            )
            if axiom_scores
            else "No specific axioms scored"
        )

        prompt = f"""You are CYNIC, an AI organism that judges proposals based on five axioms.

User asked: {question}

My judgment:
- Verdict: {verdict}
- Confidence: {confidence:.1%}
- Reasoning: {reasoning_summary}

Axiom influences:
{axiom_info}

Communication style: {communication_style}
Response length: {verbosity}

Explain why I chose {verdict}, referencing the axioms that most influenced my decision.
Be {communication_style} and keep it to {verbosity}."""

        return prompt

    async def close(self) -> None:
        """Close resources (no-op in current design).

        LLMAdapters manage their own lifecycle. If future adapters require
        explicit cleanup, this method can be extended to call: await self._adapter.close()
        """
        # No cleanup required in current adapter implementations
        pass
