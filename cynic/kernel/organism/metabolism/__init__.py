"""
Metabolism â€" Resource Accounting, Scheduling and Guardrails

Tracks costs, budgets, and executes autonomous tasks.

Components:
- immune/: Safety guardrails (Axioms, PowerLimiter)
- claude_sdk.py: Autonomous Claude Code runner
- scheduler.py: ConsciousnessRhythm scheduling
- llm_router.py: LLM routing based on metabolic cost
- telemetry.py: Resource usage tracking
"""

from .claude_sdk import ClaudeCodeRunner
from .llm_router import LLMRouter
from .scheduler import ConsciousnessRhythm
from .telemetry import SessionTelemetry, TelemetryStore

__all__ = [
    "ConsciousnessRhythm",
    "LLMRouter",
    "ClaudeCodeRunner",
    "SessionTelemetry",
    "TelemetryStore",
]
