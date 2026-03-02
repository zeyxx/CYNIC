"""
CYNIC CLI Bridge â€” Universal binary adapter.

Routes prompts to official CLI tools (claude, gemini, etc).
Uses subprocess with the '-p' (print) flag for non-interactive reasoning.
"""

from __future__ import annotations

import asyncio
import logging
import time

from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRequest, LLMResponse

logger = logging.getLogger("cynic.kernel.organism.brain.llm.cli_bridge")


class CLIAdapter(LLMAdapter):
    def __init__(self, binary: str, model_alias: str):
        super().__init__(model=model_alias, provider=f"{binary}_cli")
        self._binary = binary

    async def complete(self, request: LLMRequest) -> LLMResponse:
        start = time.time()

        # Build command: binary -p "prompt"
        # Note: We combine system and user prompt for CLI tools
        full_prompt = f"{request.system}\n\n{request.prompt}" if request.system else request.prompt
        cmd = [self._binary, "-p", full_prompt]

        # Gemini specific adjustment
        if self._binary == "gemini":
            cmd.append("--yolo")

        try:
            process = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )

            stdout, stderr = await process.communicate()
            latency_ms = (time.time() - start) * 1000

            if process.returncode == 0:
                return LLMResponse(
                    content=stdout.decode().strip(),
                    model=self.model,
                    provider=self.provider,
                    latency_ms=latency_ms,
                )
            else:
                return LLMResponse(
                    content="",
                    model=self.model,
                    provider=self.provider,
                    error=stderr.decode().strip(),
                    latency_ms=latency_ms,
                )
        except Exception as e:
            return LLMResponse(content="", model=self.model, provider=self.provider, error=str(e))

    async def check_available(self) -> bool:
        try:
            proc = await asyncio.create_subprocess_exec(
                self._binary,
                "--version",
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await proc.communicate()
            return proc.returncode == 0
        except asyncio.CancelledError:
            raise
        except (OSError, FileNotFoundError):
            return False
