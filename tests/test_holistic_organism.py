"""
CYNIC Holistic Organism Test â€” Validation of the Unified Architecture (V3).

Tests the full biological cycle:
1. Awakening (Components & Wiring)
2. Respiration (Async State Processing)
3. Cognition (Fractal Judgment Cycle)
4. Memory (Unified State Persistence)
"""

import asyncio
import pytest
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.judgment import Cell


@pytest.mark.asyncio
async def test_organism_holistic_health(organism):
    # 1. AWAKENING (Already done by fixture)
    assert organism is not None
    # All Dogs should be discovered
    assert len(organism.cognition.orchestrator.dogs) == 11

    # 2. RESPIRATION (Already started by factory in conftest)
    assert organism.state._processing is True

    # 3. COGNITION (Micro cycle)
    cell = Cell(
        content="Is the unified architecture stable?",
        reality="INTERNAL",
        analysis="JUDGE",
        lod=1,
    )

    judgment = await organism.cognition.orchestrator.run(
        cell, level=ConsciousnessLevel.MICRO, fractal_depth=1
    )

    assert judgment.verdict in ("HOWL", "WAG", "GROWL", "BARK")
    assert judgment.q_score >= 0.0

    # 4. MEMORY PERSISTENCE
    # Wait for async consolidation
    await asyncio.sleep(0.5)
    recent = organism.state.get_recent_judgments(limit=1)
    assert len(recent) > 0
    assert recent[0].judgment_id == judgment.judgment_id
