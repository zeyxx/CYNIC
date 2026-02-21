"""Tests for MCPBridge base service.

TDD Step 1: Write tests FIRST, then implement.

MCPBridge responsibilities:
- Lifecycle management (startup/shutdown)
- Tool registration
- Event emission to organism event bus
- Translates MCP tool calls into organism events
"""

from __future__ import annotations

import asyncio

import pytest
import pytest_asyncio

from cynic.core.event_bus import Event, EventBus, reset_all_buses


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

@pytest.fixture(autouse=True)
def _clean_buses():
    """Reset global bus singletons between tests."""
    reset_all_buses()
    yield
    reset_all_buses()


@pytest.fixture
def bus() -> EventBus:
    """Isolated event bus for testing."""
    return EventBus(bus_id="TEST_MCP")


# ════════════════════════════════════════════════════════════════════════════
# INITIALIZATION
# ════════════════════════════════════════════════════════════════════════════

class TestMCPBridgeInitialization:
    """MCPBridge must initialize with correct defaults."""

    def test_default_bus_name(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        assert bridge.bus_name == "CORE"

    def test_custom_bus_name(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge(bus_name="AUTOMATION")
        assert bridge.bus_name == "AUTOMATION"

    def test_not_running_initially(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        assert bridge.is_running is False

    def test_no_tools_initially(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        assert bridge.tools == {}


# ════════════════════════════════════════════════════════════════════════════
# LIFECYCLE
# ════════════════════════════════════════════════════════════════════════════

class TestMCPBridgeLifecycle:
    """MCPBridge startup/shutdown must toggle is_running."""

    @pytest.mark.asyncio
    async def test_startup_sets_running(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        await bridge.startup()
        assert bridge.is_running is True

    @pytest.mark.asyncio
    async def test_shutdown_clears_running(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        await bridge.startup()
        await bridge.shutdown()
        assert bridge.is_running is False

    @pytest.mark.asyncio
    async def test_double_startup_is_safe(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        await bridge.startup()
        await bridge.startup()  # Should not raise
        assert bridge.is_running is True

    @pytest.mark.asyncio
    async def test_shutdown_without_startup_is_safe(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        await bridge.shutdown()  # Should not raise
        assert bridge.is_running is False


# ════════════════════════════════════════════════════════════════════════════
# TOOL REGISTRATION
# ════════════════════════════════════════════════════════════════════════════

class TestMCPBridgeToolRegistration:
    """Tools must be registerable and retrievable by name."""

    def test_register_tool(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        tool = MCPTool(
            name="judge",
            description="Request a CYNIC judgment",
            input_schema={"type": "object", "properties": {"prompt": {"type": "string"}}},
        )
        bridge.register_tool(tool)
        assert "judge" in bridge.tools
        assert bridge.tools["judge"] is tool

    def test_register_multiple_tools(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        bridge.register_tool(MCPTool(name="judge", description="Judge", input_schema={}))
        bridge.register_tool(MCPTool(name="perceive", description="Perceive", input_schema={}))
        assert len(bridge.tools) == 2
        assert "judge" in bridge.tools
        assert "perceive" in bridge.tools

    def test_register_overwrites_existing(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        bridge.register_tool(MCPTool(name="judge", description="v1", input_schema={}))
        bridge.register_tool(MCPTool(name="judge", description="v2", input_schema={}))
        assert bridge.tools["judge"].description == "v2"


# ════════════════════════════════════════════════════════════════════════════
# EVENT EMISSION (handle_call)
# ════════════════════════════════════════════════════════════════════════════

class TestMCPBridgeHandleCall:
    """handle_call must emit events to the organism bus."""

    @pytest.mark.asyncio
    async def test_handle_call_emits_event(self, bus: EventBus) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        captured: list[Event] = []

        async def capture(event: Event) -> None:
            captured.append(event)

        bus.on("mcp.tool_called", capture)

        bridge = MCPBridge(bus_name="CORE")
        bridge._bus = bus  # Inject test bus
        await bridge.startup()

        bridge.register_tool(MCPTool(name="judge", description="Judge", input_schema={}))

        result = await bridge.handle_call("judge", {"prompt": "test"})

        # Let the event propagate
        await asyncio.sleep(0.05)

        assert len(captured) == 1
        assert captured[0].type == "mcp.tool_called"
        assert captured[0].payload["tool_name"] == "judge"
        assert captured[0].payload["arguments"] == {"prompt": "test"}

    @pytest.mark.asyncio
    async def test_handle_call_unknown_tool_raises(self) -> None:
        from cynic.mcp.service import MCPBridge

        bridge = MCPBridge()
        await bridge.startup()

        with pytest.raises(KeyError, match="not_registered"):
            await bridge.handle_call("not_registered", {})

    @pytest.mark.asyncio
    async def test_handle_call_when_stopped_raises(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        bridge.register_tool(MCPTool(name="judge", description="Judge", input_schema={}))
        # Not started — should raise

        with pytest.raises(RuntimeError, match="not running"):
            await bridge.handle_call("judge", {})

    @pytest.mark.asyncio
    async def test_handle_call_returns_ack(self) -> None:
        from cynic.mcp.service import MCPBridge, MCPTool

        bridge = MCPBridge()
        await bridge.startup()
        bridge.register_tool(MCPTool(name="judge", description="Judge", input_schema={}))

        result = await bridge.handle_call("judge", {"prompt": "test"})

        assert result["status"] == "emitted"
        assert result["tool_name"] == "judge"
        assert "event_id" in result


# ════════════════════════════════════════════════════════════════════════════
# MCPRouter — PROTOCOL TRANSLATION
# ════════════════════════════════════════════════════════════════════════════

class TestMCPRouterInitialization:
    """MCPRouter must set up bridge with default tools."""

    def test_router_creates_bridge(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        assert router.bridge is not None

    def test_router_registers_default_tools(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        assert "ask_cynic" in router.bridge.tools
        assert "observe_cynic" in router.bridge.tools

    def test_default_tool_schemas_have_required_fields(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        for tool in router.bridge.tools.values():
            assert tool.name
            assert tool.description
            assert isinstance(tool.input_schema, dict)


class TestMCPRouterToolsList:
    """tools/list must return registered tools in JSON-RPC format."""

    def test_tools_list_returns_jsonrpc_envelope(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        message = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
        }
        response = router.handle_message(message)
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 1
        assert "result" in response
        assert "error" not in response

    def test_tools_list_contains_default_tools(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        message = {"jsonrpc": "2.0", "id": 42, "method": "tools/list"}
        response = router.handle_message(message)
        tools = response["result"]["tools"]
        tool_names = [t["name"] for t in tools]
        assert "ask_cynic" in tool_names
        assert "observe_cynic" in tool_names

    def test_tools_list_items_have_schema(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        message = {"jsonrpc": "2.0", "id": 1, "method": "tools/list"}
        response = router.handle_message(message)
        for tool in response["result"]["tools"]:
            assert "name" in tool
            assert "description" in tool
            assert "inputSchema" in tool


class TestMCPRouterToolsCall:
    """tools/call must route to bridge.handle_call and wrap result."""

    @pytest.mark.asyncio
    async def test_tools_call_returns_jsonrpc_result(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        await router.bridge.startup()

        message = {
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {"prompt": "hello"},
            },
        }
        response = await router.handle_message_async(message)
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 7
        assert "result" in response
        assert response["result"]["status"] == "emitted"

    @pytest.mark.asyncio
    async def test_tools_call_unknown_tool_returns_error(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        await router.bridge.startup()

        message = {
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "nonexistent_tool",
                "arguments": {},
            },
        }
        response = await router.handle_message_async(message)
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 9
        assert "error" in response
        assert response["error"]["code"] == -32603

    @pytest.mark.asyncio
    async def test_tools_call_when_bridge_stopped_returns_error(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        # Bridge NOT started

        message = {
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "ask_cynic",
                "arguments": {"prompt": "test"},
            },
        }
        response = await router.handle_message_async(message)
        assert "error" in response
        assert response["error"]["code"] == -32603


class TestMCPRouterUnknownMethod:
    """Unknown methods must return JSON-RPC method-not-found error."""

    def test_unknown_method_returns_error(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        message = {
            "jsonrpc": "2.0",
            "id": 99,
            "method": "unknown/method",
        }
        response = router.handle_message(message)
        assert response["jsonrpc"] == "2.0"
        assert response["id"] == 99
        assert "error" in response
        assert response["error"]["code"] == -32601

    def test_missing_method_returns_error(self) -> None:
        from cynic.mcp.router import MCPRouter

        router = MCPRouter()
        message = {"jsonrpc": "2.0", "id": 5}
        response = router.handle_message(message)
        assert "error" in response
