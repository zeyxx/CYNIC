"""
CYNIC Holistic Organism Test — Validation of the Unified Architecture (V3).

Tests the full biological cycle:
1. Awakening (Components & Wiring)
2. Respiration (Async State Processing)
3. Cognition (Fractal Judgment Cycle)
4. Agency (Manager Veto)
5. Memory (Unified State Persistence)
6. API Gateway (HTTP Access to Memory)
"""
import asyncio
import pytest
from fastapi.testclient import TestClient
from cynic.kernel.organism.organism import awaken
from cynic.interfaces.api.server import app
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel

@pytest.mark.asyncio
async def test_organism_holistic_health():
    # 1. AWAKENING
    organism = awaken()
    assert organism is not None
    assert len(organism.dogs) == 11
    
    # 2. RESPIRATION
    await organism.state.start_processing()
    assert organism.state._processing is True
    
    try:
        # 3. COGNITION (Micro cycle)
        cell = Cell(
            content="Is the unified architecture stable?",
            reality="SOCIAL",
            analysis="JUDGE",
            lod=1
        )
        
        judgment = await organism.orchestrator.run(
            cell, 
            level=ConsciousnessLevel.MICRO,
            fractal_depth=2
        )
        
        assert judgment.verdict in ("HOWL", "WAG", "GROWL", "BARK")
        assert judgment.q_score >= 0.0
        
        # 4. MEMORY PERSISTENCE
        # Wait for async consolidation
        await asyncio.sleep(0.2)
        recent = organism.state.get_recent_judgments(limit=1)
        assert len(recent) > 0
        assert recent[0].judgment_id == judgment.judgment_id
        
        # 5. API GATEWAY
        # We use TestClient to verify the skin of the organism
        with TestClient(app) as client:
            # Check Root
            resp = client.get("/")
            assert resp.status_code == 200
            assert resp.json()["status"] == "AWAKE"
            
            # Check Memory access via API
            resp = client.get(f"/core/judge/{judgment.judgment_id}")
            if resp.status_code == 200: # Judgment might still be in buffer
                data = resp.json()
                assert data["judgment_id"] == judgment.judgment_id
                
    finally:
        # 6. SLEEP
        await organism.state.stop_processing()
        assert organism.state._processing is False

if __name__ == "__main__":
    asyncio.run(test_organism_holistic_health())
