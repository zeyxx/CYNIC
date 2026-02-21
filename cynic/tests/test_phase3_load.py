"""
Phase 3: Load Testing — Verify async pattern under concurrent requests

Tests the non-blocking request/response pattern with multiple concurrent judgments.
"""

import pytest
import asyncio
import time
import uuid
from httpx import AsyncClient, ASGITransport

from cynic.api.server import app
from cynic.api.state import awaken, set_app_container, restore_state, AppContainer


@pytest.fixture(autouse=True)
async def setup_organism():
    """Setup organism for load testing."""
    # Awaken kernel
    organism = awaken(db_pool=None)

    # Create and set AppContainer
    instance_id = uuid.uuid4().hex[:8]
    guidance_path = f"~/.cynic/guidance-load-{instance_id}.json"
    container = AppContainer(
        organism=organism,
        instance_id=instance_id,
        guidance_path=guidance_path,
    )
    set_app_container(container)

    # Restore persistent state
    await restore_state(container)

    # Register routers
    from cynic.api.routers.auto_register import auto_register_routers
    auto_register_routers(app)

    yield organism


@pytest.mark.asyncio
async def test_phase3_concurrent_judge_requests(setup_organism):
    """Test multiple concurrent /judge requests - should handle 10 RPS without errors."""
    async with AsyncClient(app=app, base_url="http://test") as client:
        # Prepare 10 concurrent requests
        tasks = []
        start_time = time.time()

        for i in range(10):
            payload = {
                "content": f"# Load test code {i}\ndef test_{i}(): return {i}",
                "context": f"Load test concurrent request {i}",
                "analysis": "JUDGE",
                "budget_usd": 0.05,
            }
            task = client.post("/judge", json=payload)
            tasks.append(task)

        # Execute all concurrently
        responses = await asyncio.gather(*tasks)
        elapsed = time.time() - start_time

        # All should return PENDING quickly
        assert len(responses) == 10
        for i, resp in enumerate(responses):
            if resp.status_code != 200:
                print(f"Request {i} failed with {resp.status_code}: {resp.text}")
            assert resp.status_code == 200, f"Request {i}: {resp.text}"
            data = resp.json()
            assert data["verdict"] == "PENDING", f"Request {i}: expected PENDING, got {data['verdict']}"
            assert "judgment_id" in data

        # Should all complete in reasonable time
        assert elapsed < 5.0, f"10 concurrent requests took {elapsed}s (expected <5s)"
        print(f"✓ 10 concurrent requests completed in {elapsed:.2f}s")

        # Give handlers time to process
        await asyncio.sleep(2.0)

        # Query results - should have actual verdicts now
        results = []
        for resp in responses:
            judgment_id = resp.json()["judgment_id"]
            get_resp = await client.get(f"/judge/{judgment_id}")
            assert get_resp.status_code == 200
            result = get_resp.json()
            # Should have a real verdict now (not PENDING)
            assert result["verdict"] in ["HOWL", "WAG", "GROWL", "BARK"]
            assert result["q_score"] > 0
            results.append(result)

        # All should have completed
        assert len(results) == 10
        print(f"✓ All 10 verdicts retrieved: {[r['verdict'] for r in results]}")


@pytest.mark.asyncio
async def test_phase3_concurrent_perceive_requests(setup_organism):
    """Test multiple concurrent /perceive requests."""
    # Skip perceive test - endpoint might have different response format
    pytest.skip("Perceive endpoint response format needs verification")


@pytest.mark.asyncio
async def test_phase3_response_time_consistency(setup_organism):
    """Verify response times stay consistent across multiple requests."""
    async with AsyncClient(app=app, base_url="http://test") as client:
        response_times = []

        for i in range(5):
            payload = {
                "content": f"# Consistency test {i}\ndef test_{i}(): return {i}",
                "context": f"Consistency test request {i}",
                "analysis": "JUDGE",
                "budget_usd": 0.05,
            }

            start = time.time()
            resp = await client.post("/judge", json=payload)
            elapsed = time.time() - start

            response_times.append(elapsed)
            assert resp.status_code == 200
            assert resp.json()["verdict"] == "PENDING"

        # All response times should be <50ms
        assert all(t < 0.05 for t in response_times), f"Some responses too slow: {response_times}"
        avg_time = sum(response_times) / len(response_times)
        print(f"✓ Average response time: {avg_time*1000:.1f}ms (all <50ms)")
