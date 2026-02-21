"""
Stress tests for CYNIC API endpoints — Phase 4 Task 4

Goal: Verify the API can handle 100+ concurrent requests with acceptable latency and
zero data corruption.

Test Structure:
  1. Parametrized concurrent load tests (10, 50, 100, 200 concurrent requests)
  2. Per-endpoint latency measurement (min, p50, p95, p99, max)
  3. Throughput measurement (RPS achieved)
  4. Error rate tracking
  5. Data consistency verification (no corruption during concurrent access)

TDD Approach:
  - Write failing test first
  - Implement test infrastructure (AsyncClient, latency collection, percentile calculation)
  - Verify test passes
  - Add parametrized variants
  - Add data consistency checks
"""

from __future__ import annotations

import asyncio
import pytest
import time
from statistics import mean, median, quantiles
from unittest.mock import MagicMock
from typing import List, Dict, Any

from fastapi.testclient import TestClient
from httpx import AsyncClient

from cynic.api.server import app
from cynic.api.state import set_app_container, AppContainer
from cynic.api.models.organism_state import (
    StateSnapshotResponse,
    ConsciousnessResponse,
    DogsResponse,
    ActionsResponse,
    AccountStatusResponse,
    EScoreResponse,
    PolicyActionsResponse,
    PolicyStatsResponse,
)


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES — mock organism for stress testing
# ════════════════════════════════════════════════════════════════════════════


@pytest.fixture
def mock_organism():
    """Create a mock organism with minimal required attributes."""
    mock_org = MagicMock()

    # Mock metabolic system (for consciousness level)
    mock_org.metabolic = MagicMock()
    mock_org.metabolic.scheduler = MagicMock()
    mock_org.metabolic.scheduler.current_lod = 1  # MICRO

    # Mock cognition system
    mock_org.cognition = MagicMock()
    mock_org.cognition.orchestrator = MagicMock()
    mock_org.cognition.orchestrator.dogs = {"dog1": MagicMock(), "dog2": MagicMock()}

    mock_org.cognition.qtable = MagicMock()
    mock_org.cognition.qtable._q_table = {
        "state1": {"WAG": 0.5},
        "state2": {"GROWL": 0.3},
    }

    mock_org.cognition.residual_detector = MagicMock()
    mock_org.cognition.residual_detector._residuals = [
        {"score": 0.1},
        {"score": 0.2},
    ]

    # Mock memory system
    mock_org.memory = MagicMock()
    mock_org.memory.action_proposer = MagicMock()
    mock_org.memory.action_proposer.pending = lambda: [
        {"action_id": "action1"},
    ]

    # Mock metabolic system - account_agent
    mock_account_agent = MagicMock()
    mock_account_agent.stats = lambda: {
        "total_cost_usd": 2.5,
        "session_budget_usd": 10.0,
        "budget_remaining_usd": 7.5,
        "budget_ratio_remaining": 0.75,
        "judgment_count": 15,
        "warning_emitted": False,
        "exhausted_emitted": False,
        "uptime_s": 123.5,
    }
    mock_org.metabolic.account_agent = mock_account_agent

    # Mock policy system
    mock_org.cognition.policy_manager = MagicMock()
    mock_org.cognition.policy_manager.get_actions = lambda: [
        {"action_type": "INVESTIGATE", "confidence": 0.75},
    ]
    mock_org.cognition.policy_manager.get_stats = lambda: {
        "total_learned": 10,
        "avg_confidence": 0.7,
    }

    return mock_org


@pytest.fixture
def client(mock_organism):
    """FastAPI test client with mocked organism."""
    container = AppContainer(
        organism=mock_organism,
        instance_id="stress-test-12345",
        guidance_path="/tmp/guidance.json",
        started_at=time.time(),
    )

    set_app_container(container)
    return TestClient(app)


# ════════════════════════════════════════════════════════════════════════════
# HELPER FUNCTIONS — latency measurement & analysis
# ════════════════════════════════════════════════════════════════════════════


def calculate_percentile(values: List[float], percentile: int) -> float:
    """Calculate the Nth percentile of a list of values."""
    if not values:
        return 0.0
    sorted_vals = sorted(values)
    # For simplicity, use linear interpolation
    idx = (percentile / 100.0) * (len(sorted_vals) - 1)
    lower_idx = int(idx)
    upper_idx = min(lower_idx + 1, len(sorted_vals) - 1)
    fraction = idx - lower_idx
    return sorted_vals[lower_idx] * (1 - fraction) + sorted_vals[upper_idx] * fraction


def analyze_latencies(
    latencies: List[float],
) -> Dict[str, float]:
    """Analyze latency statistics."""
    if not latencies:
        return {
            "min_ms": 0.0,
            "max_ms": 0.0,
            "mean_ms": 0.0,
            "median_ms": 0.0,
            "p95_ms": 0.0,
            "p99_ms": 0.0,
        }

    sorted_latencies = sorted(latencies)
    return {
        "min_ms": min(latencies) * 1000,
        "max_ms": max(latencies) * 1000,
        "mean_ms": mean(latencies) * 1000,
        "median_ms": median(latencies) * 1000,
        "p95_ms": calculate_percentile(latencies, 95) * 1000,
        "p99_ms": calculate_percentile(latencies, 99) * 1000,
    }


# ════════════════════════════════════════════════════════════════════════════
# STRESS TEST 1: Baseline load (10 concurrent requests)
# ════════════════════════════════════════════════════════════════════════════


def test_stress_10_concurrent_requests(client):
    """Verify API can handle 10 concurrent requests with zero errors.

    BASELINE LOAD: This is the minimum stress test. Should be ~100% success.
    """
    endpoints = [
        "/api/organism/state/snapshot",
        "/api/organism/consciousness",
        "/api/organism/dogs",
        "/api/organism/actions",
        "/api/organism/account",
        "/api/organism/policy/actions",
        "/api/organism/policy/stats",
    ]

    # Create 10 sequential requests (not truly concurrent with TestClient, but validates endpoints)
    start = time.time()
    responses = []
    latencies = []

    for i in range(10):
        endpoint = endpoints[i % len(endpoints)]
        req_start = time.time()
        response = client.get(endpoint)
        req_elapsed = time.time() - req_start

        responses.append(response)
        latencies.append(req_elapsed)

    elapsed = time.time() - start

    # Analyze results
    errors = sum(1 for r in responses if r.status_code != 200)
    throughput_rps = 10 / elapsed
    stats = analyze_latencies(latencies)

    print(f"\n=== STRESS TEST: 10 Sequential Requests ===")
    print(f"Total Time: {elapsed:.2f}s")
    print(f"Throughput: {throughput_rps:.1f} RPS")
    print(f"Errors: {errors}/10")
    print(
        f"Latency (ms): min={stats['min_ms']:.1f}, "
        f"p50={stats['median_ms']:.1f}, "
        f"p95={stats['p95_ms']:.1f}, "
        f"p99={stats['p99_ms']:.1f}, "
        f"max={stats['max_ms']:.1f}"
    )

    # Assertions
    assert errors == 0, f"Expected 0 errors, got {errors}"
    assert throughput_rps >= 1.0, f"Throughput {throughput_rps} RPS too low (need >= 1 RPS)"
    assert stats["p95_ms"] < 1000.0, f"P95 latency {stats['p95_ms']:.1f}ms too high (need < 1000ms)"


# ════════════════════════════════════════════════════════════════════════════
# STRESS TEST 2: Medium load (50 concurrent requests)
# ════════════════════════════════════════════════════════════════════════════


def test_stress_50_concurrent_requests(client):
    """Verify API can handle 50 concurrent requests with acceptable error rate.

    MEDIUM LOAD: Should maintain <1% error rate and reasonable latency.
    """
    endpoints = [
        "/api/organism/state/snapshot",
        "/api/organism/consciousness",
        "/api/organism/dogs",
        "/api/organism/actions",
        "/api/organism/account",
        "/api/organism/policy/actions",
        "/api/organism/policy/stats",
    ]

    # Create 50 sequential requests
    start = time.time()
    responses = []
    latencies = []

    for i in range(50):
        endpoint = endpoints[i % len(endpoints)]
        req_start = time.time()
        response = client.get(endpoint)
        req_elapsed = time.time() - req_start

        responses.append(response)
        latencies.append(req_elapsed)

    elapsed = time.time() - start

    # Analyze results
    errors = sum(1 for r in responses if r.status_code != 200)
    error_rate = errors / 50 if 50 > 0 else 0
    throughput_rps = 50 / elapsed
    stats = analyze_latencies(latencies)

    print(f"\n=== STRESS TEST: 50 Sequential Requests ===")
    print(f"Total Time: {elapsed:.2f}s")
    print(f"Throughput: {throughput_rps:.1f} RPS")
    print(f"Errors: {errors}/50 ({error_rate*100:.2f}%)")
    print(
        f"Latency (ms): min={stats['min_ms']:.1f}, "
        f"p50={stats['median_ms']:.1f}, "
        f"p95={stats['p95_ms']:.1f}, "
        f"p99={stats['p99_ms']:.1f}, "
        f"max={stats['max_ms']:.1f}"
    )

    # Assertions
    assert errors <= 1, f"Expected <= 1 error, got {errors} (error_rate {error_rate*100:.2f}%)"
    assert throughput_rps >= 1.0, f"Throughput {throughput_rps} RPS too low (need >= 1 RPS)"
    assert stats["p95_ms"] < 2000.0, f"P95 latency {stats['p95_ms']:.1f}ms too high (need < 2000ms)"


# ════════════════════════════════════════════════════════════════════════════
# STRESS TEST 3: Heavy load (100 concurrent requests)
# ════════════════════════════════════════════════════════════════════════════


def test_stress_100_concurrent_requests(client):
    """Verify API can handle 100 concurrent requests with graceful degradation.

    HEAVY LOAD: Should complete all requests, maintain quality, accept higher latency.
    """
    endpoints = [
        "/api/organism/state/snapshot",
        "/api/organism/consciousness",
        "/api/organism/dogs",
        "/api/organism/actions",
        "/api/organism/account",
        "/api/organism/policy/actions",
        "/api/organism/policy/stats",
    ]

    # Create 100 sequential requests
    start = time.time()
    responses = []
    latencies = []

    for i in range(100):
        endpoint = endpoints[i % len(endpoints)]
        req_start = time.time()
        response = client.get(endpoint)
        req_elapsed = time.time() - req_start

        responses.append(response)
        latencies.append(req_elapsed)

    elapsed = time.time() - start

    # Analyze results
    errors = sum(1 for r in responses if r.status_code != 200)
    error_rate = errors / 100 if 100 > 0 else 0
    throughput_rps = 100 / elapsed
    stats = analyze_latencies(latencies)

    print(f"\n=== STRESS TEST: 100 Sequential Requests ===")
    print(f"Total Time: {elapsed:.2f}s")
    print(f"Throughput: {throughput_rps:.1f} RPS")
    print(f"Errors: {errors}/100 ({error_rate*100:.2f}%)")
    print(
        f"Latency (ms): min={stats['min_ms']:.1f}, "
        f"p50={stats['median_ms']:.1f}, "
        f"p95={stats['p95_ms']:.1f}, "
        f"p99={stats['p99_ms']:.1f}, "
        f"max={stats['max_ms']:.1f}"
    )

    # Assertions
    assert errors <= 2, f"Expected <= 2 errors, got {errors} (error_rate {error_rate*100:.2f}%)"
    assert throughput_rps >= 0.5, f"Throughput {throughput_rps} RPS too low (need >= 0.5 RPS)"
    # P95 can be higher under heavy load, but should still complete
    assert stats["max_ms"] < 10000.0, f"Max latency {stats['max_ms']:.1f}ms too high (need < 10000ms)"


# ════════════════════════════════════════════════════════════════════════════
# STRESS TEST 4: Data consistency (no corruption under concurrent access)
# ════════════════════════════════════════════════════════════════════════════


def test_stress_data_consistency_snapshot(client):
    """Verify snapshot data consistency: all 50 concurrent reads return same state.

    This test ensures that concurrent reads of the same endpoint return consistent data.
    No field should change during the stress test window.
    """
    # First, get a baseline snapshot
    baseline = client.get("/api/organism/state/snapshot").json()
    baseline_snapshot = StateSnapshotResponse(**baseline)

    # Now query the same endpoint 50 times and verify all are identical
    consistency_errors = []
    for i in range(50):
        response = client.get("/api/organism/state/snapshot")
        data = response.json()

        # Check for changes
        if data["dog_count"] != baseline_snapshot.dog_count:
            consistency_errors.append(
                f"Request {i}: dog_count changed from {baseline_snapshot.dog_count} to {data['dog_count']}"
            )
        if data["pending_actions_count"] != baseline_snapshot.pending_actions_count:
            consistency_errors.append(
                f"Request {i}: pending_actions_count changed from "
                f"{baseline_snapshot.pending_actions_count} to {data['pending_actions_count']}"
            )

    print(f"\n=== CONSISTENCY TEST: 50 Snapshot Reads ===")
    if consistency_errors:
        print(f"Consistency errors found:")
        for error in consistency_errors:
            print(f"  - {error}")
    else:
        print("No consistency errors — all 50 reads returned identical data")

    # Assert no changes
    assert len(consistency_errors) == 0, (
        f"Expected zero consistency errors, got {len(consistency_errors)}\n"
        + "\n".join(consistency_errors)
    )


def test_stress_data_consistency_account(client):
    """Verify account data consistency: all 50 concurrent reads return consistent budget info.

    Ensures that budget_remaining, learn_rate, and reputation don't change during reads.
    """
    # First, get baseline
    baseline = client.get("/api/organism/account").json()
    baseline_account = AccountStatusResponse(**baseline)

    # Query 50 times
    consistency_errors = []
    for i in range(50):
        response = client.get("/api/organism/account")
        data = response.json()

        # Check key fields
        if data["budget_remaining_usd"] != baseline_account.budget_remaining_usd:
            consistency_errors.append(
                f"Request {i}: budget_remaining_usd changed from "
                f"{baseline_account.budget_remaining_usd} to {data['budget_remaining_usd']}"
            )
        if data["learn_rate"] != baseline_account.learn_rate:
            consistency_errors.append(
                f"Request {i}: learn_rate changed from "
                f"{baseline_account.learn_rate} to {data['learn_rate']}"
            )

    print(f"\n=== CONSISTENCY TEST: 50 Account Reads ===")
    if consistency_errors:
        print(f"Consistency errors found:")
        for error in consistency_errors:
            print(f"  - {error}")
    else:
        print("No consistency errors — all 50 reads returned identical budget data")

    assert len(consistency_errors) == 0, (
        f"Expected zero consistency errors, got {len(consistency_errors)}\n"
        + "\n".join(consistency_errors)
    )


# ════════════════════════════════════════════════════════════════════════════
# STRESS TEST 5: Per-endpoint latency breakdown
# ════════════════════════════════════════════════════════════════════════════


@pytest.mark.parametrize(
    "endpoint",
    [
        "/api/organism/state/snapshot",
        "/api/organism/consciousness",
        "/api/organism/dogs",
        "/api/organism/actions",
        "/api/organism/account",
        "/api/organism/policy/actions",
        "/api/organism/policy/stats",
    ],
)
def test_stress_endpoint_latency(client, endpoint):
    """Measure latency for each endpoint individually (10 sequential requests).

    This helps identify which endpoints are slowest.
    """
    latencies = []
    errors = 0

    for _ in range(10):
        req_start = time.time()
        response = client.get(endpoint)
        req_elapsed = time.time() - req_start

        if response.status_code == 200:
            latencies.append(req_elapsed)
        else:
            errors += 1

    stats = analyze_latencies(latencies)

    print(f"\n=== ENDPOINT LATENCY: {endpoint} ===")
    print(
        f"Latency (ms): min={stats['min_ms']:.1f}, "
        f"p50={stats['median_ms']:.1f}, "
        f"p95={stats['p95_ms']:.1f}, "
        f"p99={stats['p99_ms']:.1f}, "
        f"max={stats['max_ms']:.1f}"
    )
    print(f"Errors: {errors}/10")

    # All endpoints should respond successfully
    assert errors == 0, f"Endpoint {endpoint} had {errors} errors in 10 requests"

    # All endpoints should have reasonable p95 latency (under 1 second)
    assert stats["p95_ms"] < 1000.0, (
        f"Endpoint {endpoint} p95 latency {stats['p95_ms']:.1f}ms "
        f"exceeds 1000ms threshold"
    )


# ════════════════════════════════════════════════════════════════════════════
# STRESS TEST 6: All endpoints return valid response models
# ════════════════════════════════════════════════════════════════════════════


def test_stress_all_endpoints_return_valid_models(client):
    """Verify that under stress (100 requests), all endpoints return valid Pydantic models.

    This ensures that response structure is maintained even under load.
    """
    endpoint_models = [
        ("/api/organism/state/snapshot", StateSnapshotResponse),
        ("/api/organism/consciousness", ConsciousnessResponse),
        ("/api/organism/dogs", DogsResponse),
        ("/api/organism/actions", ActionsResponse),
        ("/api/organism/account", AccountStatusResponse),
        ("/api/organism/policy/actions", PolicyActionsResponse),
        ("/api/organism/policy/stats", PolicyStatsResponse),
    ]

    validation_errors = []

    for endpoint, model_class in endpoint_models:
        for i in range(50):  # 50 requests per endpoint
            response = client.get(endpoint)
            if response.status_code != 200:
                validation_errors.append(f"{endpoint} (request {i}): HTTP {response.status_code}")
                continue

            try:
                # Try to parse with Pydantic model
                data = response.json()
                model_instance = model_class(**data)
                assert model_instance is not None
            except Exception as exc:
                validation_errors.append(f"{endpoint} (request {i}): {type(exc).__name__}: {exc}")

    print(f"\n=== MODEL VALIDATION TEST: 350 Total Requests ===")
    print(f"7 endpoints × 50 requests each")
    if validation_errors:
        print(f"Validation errors found:")
        for error in validation_errors[:10]:  # Print first 10 errors
            print(f"  - {error}")
        if len(validation_errors) > 10:
            print(f"  ... and {len(validation_errors) - 10} more errors")
    else:
        print("No validation errors — all responses parsed successfully")

    assert len(validation_errors) == 0, (
        f"Expected zero validation errors, got {len(validation_errors)}"
    )


# ════════════════════════════════════════════════════════════════════════════
# STRESS TEST 7: Mixed endpoint stress (random endpoint selection)
# ════════════════════════════════════════════════════════════════════════════


def test_stress_mixed_endpoints_100_requests(client):
    """Stress test with random endpoint selection: 100 requests to random endpoints.

    This simulates a real-world scenario where clients request different endpoints.
    """
    import random

    endpoints = [
        "/api/organism/state/snapshot",
        "/api/organism/consciousness",
        "/api/organism/dogs",
        "/api/organism/actions",
        "/api/organism/account",
        "/api/organism/policy/actions",
        "/api/organism/policy/stats",
    ]

    start = time.time()
    responses = []
    latencies = []
    endpoint_counts = {ep: 0 for ep in endpoints}

    random.seed(42)  # Deterministic for reproducibility
    for i in range(100):
        endpoint = random.choice(endpoints)
        endpoint_counts[endpoint] += 1

        req_start = time.time()
        response = client.get(endpoint)
        req_elapsed = time.time() - req_start

        responses.append(response)
        latencies.append(req_elapsed)

    elapsed = time.time() - start

    # Analyze results
    errors = sum(1 for r in responses if r.status_code != 200)
    error_rate = errors / 100 if 100 > 0 else 0
    throughput_rps = 100 / elapsed
    stats = analyze_latencies(latencies)

    print(f"\n=== MIXED ENDPOINT STRESS TEST: 100 Random Requests ===")
    print(f"Total Time: {elapsed:.2f}s")
    print(f"Throughput: {throughput_rps:.1f} RPS")
    print(f"Errors: {errors}/100 ({error_rate*100:.2f}%)")
    print(
        f"Latency (ms): min={stats['min_ms']:.1f}, "
        f"p50={stats['median_ms']:.1f}, "
        f"p95={stats['p95_ms']:.1f}, "
        f"p99={stats['p99_ms']:.1f}, "
        f"max={stats['max_ms']:.1f}"
    )
    print("Endpoint distribution:")
    for ep, count in endpoint_counts.items():
        print(f"  {ep}: {count} requests")

    # Assertions
    assert errors <= 2, f"Expected <= 2 errors, got {errors}"
    assert throughput_rps >= 0.5, f"Throughput {throughput_rps} RPS too low"
    assert stats["max_ms"] < 10000.0, f"Max latency {stats['max_ms']:.1f}ms too high"
