"""Bridge to Claude API for natural language generation."""

from __future__ import annotations

import os
from typing import Any

from anthropic import AsyncAnthropic


class LLMBridge:
    """Interface to Claude API for dialogue responses."""

    def __init__(self, api_key: str | None = None):
        self.api_key = api_key or os.getenv("ANTHROPIC_API_KEY")
        self.client = AsyncAnthropic(api_key=self.api_key) if self.api_key else None
        self.model = "claude- opus-4-6"  # Use latest available model

    async def generate_response(self, context: dict[str, Any]) -> str:
        """Generate natural language response from reasoning context."""
        # 1. Check if we should use Remote Proxy (Docker)
        from cynic.kernel.observability.symbiotic_state_manager import get_symbiotic_state_manager
        mgr = await get_symbiotic_state_manager()
        
        if mgr.remote_mode and not self.api_key:
            return await self._proxy_to_remote(context, mgr.api_url)

        # 2. Local execution
        if not self.client:
            return "I'm unable to explain my reasoning: No API key found and no remote instance detected."

        try:
            prompt = self._create_explanation_prompt(context)

            message = await self.client.messages.create(
                model=self.model,
                max_tokens=256,
                messages=[
                    {
                        "role": "user",
                        "content": prompt
                    }
                ]
            )

            return message.content[0].text

        except Exception as e:
            # Graceful degradation: return structured response
            return f"I'm unable to explain my reasoning right now: {str(e)[:100]}"

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
        axiom_info = "\n".join([
            f"- {axiom}: {score:.1%}"
            for axiom, score in sorted(axiom_scores.items(),
                                      key=lambda x: x[1],
                                      reverse=True)
        ]) if axiom_scores else "No specific axioms scored"

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
        """Close API client."""
        await self.client.close()
