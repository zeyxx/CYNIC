"""
Tests for MCP Empirical Runner — Job lifecycle and Cline integration.
"""
import asyncio

import pytest

from cynic.interfaces.mcp.empirical_runner import EmpiricalRunner


@pytest.fixture
def mock_organism():
    """Mock CynicOrganism for testing."""

    class MockOrchestrator:
        async def judge(self):
            """Return mock judgment result."""
            return {"q_score": 52.4, "verdict": "WAG"}

    class MockCortex:
        def __init__(self):
            self.orchestrator = MockOrchestrator()

    class MockOrganism:
        def __init__(self):
            self.cortex = MockCortex()

    return MockOrganism()


@pytest.fixture
def runner(mock_organism, tmp_path):
    """Create EmpiricalRunner with mock organism."""
    return EmpiricalRunner(
        organism_getter=lambda: mock_organism,
        results_dir=str(tmp_path / "results"),
    )


@pytest.mark.asyncio
async def test_spawn_test_returns_job_id(runner):
    """Test: spawn_test() returns a valid job_id."""
    job_id = await runner.spawn_test(count=10)
    assert job_id.startswith("test-")
    assert job_id in runner.jobs


@pytest.mark.asyncio
async def test_job_status_queued_initially(runner):
    """Test: job starts in QUEUED status."""
    job_id = await runner.spawn_test(count=10)
    status = await runner.get_job_status(job_id)
    assert status["status"] == "queued"


@pytest.mark.asyncio
async def test_job_runs_and_completes(runner):
    """Test: job transitions to RUNNING then COMPLETE."""
    job_id = await runner.spawn_test(count=5)

    # Wait a bit for job to start
    await asyncio.sleep(0.1)

    status = await runner.get_job_status(job_id)
    assert status["status"] in ["running", "complete"]

    # Wait for completion
    for _ in range(50):
        status = await runner.get_job_status(job_id)
        if status["status"] == "complete":
            break
        await asyncio.sleep(0.1)

    assert status["status"] == "complete"


@pytest.mark.asyncio
async def test_get_results_requires_completion(runner):
    """Test: get_results() returns None until job completes."""
    job_id = await runner.spawn_test(count=5)

    # Immediately after spawn, results should be None
    results = await runner.get_results(job_id)
    assert results is None

    # Wait for completion
    for _ in range(50):
        status = await runner.get_job_status(job_id)
        if status["status"] == "complete":
            break
        await asyncio.sleep(0.1)

    # After completion, results should be available
    results = await runner.get_results(job_id)
    assert results is not None
    assert "q_scores" in results
    assert "avg_q" in results
    assert "learning_efficiency" in results


@pytest.mark.asyncio
async def test_results_contain_metrics(runner):
    """Test: completed results contain all expected metrics."""
    job_id = await runner.spawn_test(count=10)

    # Wait for completion
    while True:
        status = await runner.get_job_status(job_id)
        if status["status"] == "complete":
            break
        await asyncio.sleep(0.1)

    results = await runner.get_results(job_id)

    assert len(results["q_scores"]) == 10
    assert results["avg_q"] > 0
    assert results["min_q"] > 0
    assert results["max_q"] > 0
    assert results["learning_efficiency"] > 0
    assert results["duration_s"] > 0


@pytest.mark.asyncio
async def test_irreducibility_test_returns_axiom_impacts(runner):
    """Test: irreducibility test returns axiom impact data."""
    results = await runner.run_irreducibility_test(axiom="PHI")
    assert "axiom_impacts" in results
    assert len(results["axiom_impacts"]) > 0

    impact = results["axiom_impacts"][0]
    assert "name" in impact
    assert "baseline_q" in impact
    assert "disabled_q" in impact
    assert "impact_percent" in impact
    assert "irreducible" in impact


@pytest.mark.asyncio
async def test_irreducibility_test_all_axioms(runner):
    """Test: irreducibility test with axiom=None tests all 5 axioms."""
    results = await runner.run_irreducibility_test(axiom=None)
    assert "axiom_impacts" in results
    assert len(results["axiom_impacts"]) == 5


@pytest.mark.asyncio
async def test_query_telemetry(runner):
    """Test: query_telemetry returns valid metrics."""
    telemetry = await runner.query_telemetry(metric="uptime_s")
    assert "metric" in telemetry
    assert "uptime_s" in telemetry
    assert telemetry["uptime_s"] >= 0


@pytest.mark.asyncio
async def test_multiple_jobs_run_in_parallel(runner):
    """Test: multiple jobs can run concurrently."""
    job1 = await runner.spawn_test(count=5)
    job2 = await runner.spawn_test(count=5)

    assert job1 != job2
    assert job1 in runner.jobs
    assert job2 in runner.jobs

    # Both should progress
    await asyncio.sleep(0.2)

    status1 = await runner.get_job_status(job1)
    status2 = await runner.get_job_status(job2)

    assert status1["status"] in ["running", "complete"]
    assert status2["status"] in ["running", "complete"]


@pytest.mark.asyncio
async def test_get_status_invalid_job(runner):
    """Test: get_status returns error for unknown job_id."""
    status = await runner.get_job_status("invalid-job-id")
    assert "error" in status


@pytest.mark.asyncio
async def test_get_results_invalid_job(runner):
    """Test: get_results returns None for unknown job_id."""
    results = await runner.get_results("invalid-job-id")
    assert results is None
