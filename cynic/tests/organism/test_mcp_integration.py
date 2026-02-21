"""
Integration Tests: MCPBridge wired into Organism SensoryCore.

Verifies that:
1. Organism.senses.mcp_bridge exists after awaken()
2. organism.mcp_bridge property delegates to senses.mcp_bridge
3. MCPBridge is created but NOT started (startup is lifespan's job)
4. MCPBridge instance is shared (single instance per organism)

LAW: NO MOCKS. Uses real awaken() path.
"""
from __future__ import annotations

import pytest

from cynic.mcp.service import MCPBridge
from cynic.organism.organism import awaken, Organism, SensoryCore


# ────────────────────────────────────────────────────────────────────────────
# TEST 1: Organism has MCPBridge in SensoryCore
# ────────────────────────────────────────────────────────────────────────────


def test_sensory_core_has_mcp_bridge_field():
    """SensoryCore dataclass should have an mcp_bridge field."""
    core = SensoryCore()
    assert hasattr(core, "mcp_bridge")
    assert isinstance(core.mcp_bridge, MCPBridge)


def test_sensory_core_mcp_bridge_default_not_running():
    """Default MCPBridge in SensoryCore should not be running."""
    core = SensoryCore()
    assert core.mcp_bridge.is_running is False


# ────────────────────────────────────────────────────────────────────────────
# TEST 2: Full organism wiring via awaken()
# ────────────────────────────────────────────────────────────────────────────


def test_organism_initializes_mcp_bridge():
    """Organism should have MCPBridge as part of SensoryCore after awaken()."""
    organism = awaken(db_pool=None, registry=None)

    assert hasattr(organism.senses, "mcp_bridge")
    assert organism.senses.mcp_bridge is not None
    assert isinstance(organism.senses.mcp_bridge, MCPBridge)
    assert organism.senses.mcp_bridge.is_running is False


def test_organism_mcp_bridge_property():
    """Organism should provide direct access to MCPBridge via property."""
    organism = awaken(db_pool=None, registry=None)

    bridge = organism.mcp_bridge
    assert bridge is not None
    assert isinstance(bridge, MCPBridge)
    assert bridge is organism.senses.mcp_bridge


def test_organism_mcp_bridge_uses_core_bus():
    """MCPBridge should be configured for the CORE bus."""
    organism = awaken(db_pool=None, registry=None)

    assert organism.mcp_bridge.bus_name == "CORE"


# ────────────────────────────────────────────────────────────────────────────
# TEST 3: MCPBridge lifecycle (not started at creation)
# ────────────────────────────────────────────────────────────────────────────


@pytest.mark.asyncio
async def test_organism_mcp_bridge_can_start_and_stop():
    """MCPBridge should be startable and stoppable after organism creation."""
    organism = awaken(db_pool=None, registry=None)
    bridge = organism.mcp_bridge

    assert bridge.is_running is False

    await bridge.startup()
    assert bridge.is_running is True

    await bridge.shutdown()
    assert bridge.is_running is False
