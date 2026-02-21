"""MCP architecture verification tests.

Validates structural properties of the MCP subsystem:
- Single responsibility (MCPBridge has < 10 public methods)
- File size constraints (routers under 150 lines)
- Internal integration (bridge lives in organism.senses)
- Observability interface (get_metrics, get_health exist)

These tests enforce design invariants, not runtime behavior.
"""
from __future__ import annotations

import inspect
from pathlib import Path

from cynic.mcp.service import MCPBridge
from cynic.organism.organism import SensoryCore


_ROUTERS_DIR = Path(__file__).resolve().parent.parent.parent / "cynic" / "api" / "routers"


class TestMCPBridgeSingleResponsibility:
    """MCPBridge stays focused: fewer than 10 public methods."""

    def test_public_method_count_under_limit(self) -> None:
        public_methods = [
            name for name, _ in inspect.getmembers(MCPBridge, predicate=inspect.isfunction)
            if not name.startswith("_")
        ]
        assert len(public_methods) < 10, (
            f"MCPBridge has {len(public_methods)} public methods: {public_methods}. "
            "Keep it under 10 for single responsibility."
        )


class TestMCPRouterSmallFile:
    """mcp_websocket.py stays under 150 lines."""

    def test_mcp_websocket_under_150_lines(self) -> None:
        path = _ROUTERS_DIR / "mcp_websocket.py"
        line_count = len(path.read_text(encoding="utf-8").splitlines())
        assert line_count < 150, (
            f"mcp_websocket.py is {line_count} lines. Keep under 150."
        )


class TestMCPMetricsSmallFile:
    """mcp_observability.py stays under 150 lines."""

    def test_mcp_observability_under_150_lines(self) -> None:
        path = _ROUTERS_DIR / "mcp_observability.py"
        line_count = len(path.read_text(encoding="utf-8").splitlines())
        assert line_count < 150, (
            f"mcp_observability.py is {line_count} lines. Keep under 150."
        )


class TestMCPIntegratedNotExternal:
    """MCPBridge lives inside organism.senses (internal, not external)."""

    def test_bridge_in_sensory_core(self) -> None:
        senses = SensoryCore()
        assert hasattr(senses, "mcp_bridge")
        assert isinstance(senses.mcp_bridge, MCPBridge)

    def test_bridge_field_on_dataclass(self) -> None:
        fields = {f.name for f in SensoryCore.__dataclass_fields__.values()}
        assert "mcp_bridge" in fields


class TestMCPObservable:
    """MCPBridge exposes get_metrics() and get_health()."""

    def test_has_get_metrics(self) -> None:
        bridge = MCPBridge()
        assert callable(getattr(bridge, "get_metrics", None))
        result = bridge.get_metrics()
        assert isinstance(result, dict)

    def test_has_get_health(self) -> None:
        bridge = MCPBridge()
        assert callable(getattr(bridge, "get_health", None))
        result = bridge.get_health()
        assert isinstance(result, dict)
        assert "status" in result
