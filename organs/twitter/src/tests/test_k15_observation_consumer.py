from types import SimpleNamespace

import pytest

from organs.twitter.src.entrypoints import k15_observation_consumer as k15


def test_normalize_kernel_url_rejects_loopback_without_override(monkeypatch):
    monkeypatch.delenv("CYNIC_ALLOW_LOCAL_KERNEL", raising=False)

    with pytest.raises(RuntimeError, match="loopback"):
        k15.K15ObservationConsumer._normalize_kernel_url("localhost:8080")


def test_normalize_kernel_url_accepts_mesh_address():
    assert k15.K15ObservationConsumer._normalize_kernel_url("100.64.0.10:8080") == (
        "http://100.64.0.10:8080"
    )


def test_consumer_headers_include_bearer_token():
    consumer = k15.K15ObservationConsumer(kernel_url="100.64.0.10:8080", api_key="test-token")

    assert consumer.headers() == {
        "Content-Type": "application/json",
        "Authorization": "Bearer test-token",
    }


def test_dispatch_task_posts_high_signal_observation(monkeypatch):
    consumer = k15.K15ObservationConsumer(kernel_url="100.64.0.10:8080", api_key="test-token")
    calls = []

    def fake_post(url, json, headers, timeout):
        calls.append({"url": url, "json": json, "headers": headers, "timeout": timeout})
        return SimpleNamespace(status_code=201, json=lambda: {"task_id": "task-1"})

    monkeypatch.setattr(k15.requests, "post", fake_post)

    task_id = consumer.dispatch_task(
        {"_id": "obs-1", "context": "rug pull warning"},
        {
            "observation_id": "obs-1",
            "signal_score": 8.0,
            "verdict": "BARK",
            "q_score": 0.3,
            "reasoning": {"reason": "test"},
            "high_signal": True,
        },
    )

    assert task_id == "task-1"
    assert calls[0]["url"] == "http://100.64.0.10:8080/agent-tasks"
    assert calls[0]["json"]["kind"] == "hermes"
    assert calls[0]["json"]["domain"] == "social-signal"
    assert calls[0]["headers"]["Authorization"] == "Bearer test-token"


def test_run_allows_zero_iteration_dry_run(monkeypatch):
    consumer = k15.K15ObservationConsumer(kernel_url="100.64.0.10:8080")
    monkeypatch.setattr(consumer, "fetch_observations", lambda limit=100: pytest.fail("should not poll"))

    consumer.run(max_iterations=0)
