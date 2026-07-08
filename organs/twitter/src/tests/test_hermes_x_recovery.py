import json
import time
from types import SimpleNamespace

from organs.twitter.src.entrypoints import hermes_x_recovery as recovery


def write_dataset(path, rows, stale=True):
    path.write_text("\n".join(json.dumps(row) for row in rows) + "\n")
    if stale:
        old = time.time() - 600
        path.touch()
        import os

        os.utime(path, (old, old))


def test_normalize_kernel_addr_rejects_loopback(monkeypatch):
    monkeypatch.delenv("CYNIC_ALLOW_LOCAL_KERNEL", raising=False)

    assert recovery._normalize_kernel_addr("localhost:3030") == ""


def test_check_heartbeat_detects_missing_dataset(monkeypatch, tmp_path):
    monkeypatch.setenv("X_DATASET_PATH", str(tmp_path / "missing.jsonl"))

    assert recovery.check_heartbeat() == {
        "status": "critical",
        "failure_reason": "dataset_missing",
    }


def test_check_heartbeat_detects_stale_extraction_breakage(monkeypatch, tmp_path):
    dataset = tmp_path / "dataset.jsonl"
    write_dataset(
        dataset,
        [
            {"tweet_id": "", "text": ""},
            {"tweet_id": "", "text": ""},
            {"tweet_id": "3", "text": "healthy"},
        ],
    )
    monkeypatch.setenv("X_DATASET_PATH", str(dataset))

    assert recovery.check_heartbeat() == {
        "status": "critical",
        "failure_reason": "x_extraction_broken",
    }


def test_layer2_retry_login_uses_organ_entrypoint(monkeypatch):
    calls = []

    def fake_run(args, **kwargs):
        calls.append({"args": args, "kwargs": kwargs})
        return SimpleNamespace(returncode=0, stderr="")

    monkeypatch.setattr(recovery.subprocess, "run", fake_run)
    monkeypatch.setattr(recovery, "LAYER2_RETRY_COUNT", 0)

    assert recovery.layer2_retry_login() is True
    assert "organs/twitter/src/entrypoints/hermes_x_login.py" in str(calls[0]["args"])
    assert "--force" in calls[0]["args"]


def test_layer4_alert_kernel_posts_observation(monkeypatch):
    calls = []
    monkeypatch.setattr(recovery, "REST_ADDR", "http://100.64.0.10:3030")
    monkeypatch.setattr(recovery, "API_KEY", "test-token")

    def fake_post(url, json, headers, timeout):
        calls.append({"url": url, "json": json, "headers": headers, "timeout": timeout})
        return SimpleNamespace(status_code=200)

    import requests

    monkeypatch.setattr(requests, "post", fake_post)

    recovery.layer4_alert_kernel()

    assert calls[0]["url"] == "http://100.64.0.10:3030/observe"
    assert calls[0]["json"]["domain"] == "organ-health"
    assert calls[0]["json"]["status"] == "critical"
    assert calls[0]["headers"]["Authorization"] == "Bearer test-token"

