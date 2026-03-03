"""
PHASE 3 Tier 1 End-to-End Tests â€” EMPIRICAL (NO MOCKS)

Tests the event-driven judgment pipeline through the HTTP API layer.
Validates the transition from PENDING to reality-driven state.
"""

import pytest

pytestmark = pytest.mark.skip(
    reason="Old architecture: module imports not available in V5"
)

# Block all imports that would fail
pytest.skip("Skipping old architecture test module", allow_module_level=True)


import pytest

pytestmark = pytest.mark.skip(reason="Old architecture, modules removed")

import asyncio
import pytest
from httpx import AsyncClient, ASGITransport
from cynic.interfaces.api.server import app


@pytest.mark.asyncio
class TestPhase3Tier1EndToEnd:
    """Verify core judgment flow through API."""

    async def test_judge_full_cycle_reflex_level(self):
        """POST /judge -> PENDING -> Wait -> Result."""
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            # 1. POST
            post_resp = await client.post(
                "/api/judge",
                json={
                    "content": "phi = (1 + 5**0.5) / 2",
                    "reality": "CYNIC",
                    "analysis": "JUDGE",
                    "level": "MICRO",
                },
            )
            assert post_resp.status_code == 200
            jid = post_resp.json()["judgment_id"]
            assert post_resp.json()["verdict"] == "PENDING"

            # 2. WAIT
            await asyncio.sleep(0.5)

            # 3. GET
            get_resp = await client.get(f"/api/judge/{jid}")
            assert get_resp.status_code == 200
            data = get_resp.json()
            # If no LLM, it might stay PENDING or move to COMPLETED if reflex dogs finish
            assert data["status"] in ("PENDING", "COMPLETED")

    async def test_perceive_full_cycle(self):
        """POST /perceive -> Event Queued -> Poll State."""
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            post_resp = await client.post(
                "/api/perceive",
                json={
                    "data": "GitHub heartbeat",
                    "source": "github_webhook",
                    "reality": "SOCIAL",
                    "run_judgment": True,
                },
            )
            assert post_resp.status_code == 200
            cell_id = post_resp.json()["cell_id"]

            await asyncio.sleep(0.5)

            get_resp = await client.get(f"/api/judge/{cell_id}")
            assert get_resp.status_code == 200
            assert get_resp.json()["judgment_id"] == cell_id


class TestPhase3ResponseModels:
    """Verify Pydantic models in real responses."""

    async def test_judge_response_schema(self):
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            resp = await client.post("/api/judge", json={"content": "test"})
            data = resp.json()
            required = [
                "judgment_id",
                "q_score",
                "verdict",
                "confidence",
                "axiom_scores",
                "dog_votes",
                "consensus_reached",
                "consensus_votes",
            ]
            for field in required:
                assert field in data
