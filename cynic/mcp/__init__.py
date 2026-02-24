"""
CYNIC MCP — Model Context Protocol bootstrap bridge.

Two implementations:
1. HTTP MCP Server (aiohttp):
   - /observe  — Get CYNIC state (Component 1 ServiceStateRegistry)
   - /act      — Execute Claude Code actions
   - /learn    — Human feedback for Q-Table learning

2. Stdio MCP Server (for Cline integration):
   - cynic_run_empirical_test() — Spawn judgment batch tests
   - cynic_get_job_status() — Poll test progress
   - cynic_get_results() — Fetch test results
   - cynic_run_irreducibility_test() — Test axiom necessity
   - cynic_query_telemetry() — SONA heartbeat metrics

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
from cynic.mcp.empirical_runner import EmpiricalRunner, JobResult

# Optional: stdio_server requires 'mcp' package (pip install mcp)
try:
    from cynic.mcp.stdio_server import CynicMCPServer, start_mcp_server
    _HAS_MCP = True
except ImportError:
    CynicMCPServer = None  # type: ignore
    start_mcp_server = None  # type: ignore
    _HAS_MCP = False

__all__ = [
    # HTTP MCP
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
    # Stdio MCP (optional)
    "EmpiricalRunner",
    "JobResult",
    "CynicMCPServer",
    "start_mcp_server",
]
