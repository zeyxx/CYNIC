"""
CYNIC MCP — Model Context Protocol bootstrap bridge.

Connects Claude Code ↔ CYNIC organism via three endpoints:
- /observe  — Get CYNIC state (Component 1 ServiceStateRegistry)
- /act      — Execute Claude Code actions
- /learn    — Human feedback for Q-Table learning

Pure async, Pydantic v2, OpenTelemetry-ready.
"""
from cynic.mcp.server import MCPServer, run_mcp_server
from cynic.mcp.models import (
    ObserveRequest,
    ObserveResponse,
    ActRequest,
    ActResponse,
    LearnRequest,
    LearnResponse,
    ErrorResponse,
    ComponentHealthSnapshot,
    RegistrySnapshot,
    ActionProposal,
    FeedbackSignal,
)
from cynic.mcp.utils import setup_logging

__all__ = [
    "MCPServer",
    "run_mcp_server",
    "ObserveRequest",
    "ObserveResponse",
    "ActRequest",
    "ActResponse",
    "LearnRequest",
    "LearnResponse",
    "ErrorResponse",
    "ComponentHealthSnapshot",
    "RegistrySnapshot",
    "ActionProposal",
    "FeedbackSignal",
    "setup_logging",
]
