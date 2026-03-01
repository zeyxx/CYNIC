"""
Metabolism — Resource Accounting, Scheduling and Guardrails

Tracks costs, budgets, and executes autonomous tasks.

Components:
- immune/: Safety guardrails (Axioms, PowerLimiter)
- claude_sdk.py: Autonomous Claude Code runner
- scheduler.py: ConsciousnessRhythm scheduling
- llm_router.py: LLM routing based on metabolic cost
- telemetry.py: Resource usage tracking
"""
from .scheduler import ConsciousnessRhythm
from .llm_router import LLMRouter
from .claude_sdk import ClaudeCodeRunner
from .telemetry import SessionTelemetry, TelemetryStore

__all__ = [
    "ConsciousnessRhythm",
    "LLMRouter",
    "ClaudeCodeRunner",
    "SessionTelemetry",
    "TelemetryStore",
]
