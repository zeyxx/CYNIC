from types import SimpleNamespace

from organs.twitter.src.adapters.x_kernel_transport import (
    KernelTransport,
    build_convergence_payload,
    build_heartbeat_payload,
    build_social_judgment_payload,
    build_token_judgment_payload,
    build_tweet_observe_payload,
    kernel_headers,
    normalize_kernel_addr,
)


class FakeRequests:
    def __init__(self, statuses):
        self.statuses = list(statuses)
        self.calls = []

    def post(self, url, json, headers, timeout):
        self.calls.append({"url": url, "json": json, "headers": headers, "timeout": timeout})
        status = self.statuses.pop(0)
        return SimpleNamespace(status_code=status, json=lambda: {"ok": True, "status": status})


def test_normalize_kernel_addr_adds_scheme():
    assert normalize_kernel_addr("10.0.0.7:3000") == "http://10.0.0.7:3000"
    assert normalize_kernel_addr("https://kernel") == "https://kernel"


def test_kernel_headers_include_bearer_token():
    assert kernel_headers("secret")["Authorization"] == "Bearer secret"


def test_build_tweet_observe_payload_limits_tags_and_context():
    payload = build_tweet_observe_payload(
        {
            "tweet_id": "1",
            "text": "hello",
            "author_screen_name": "alice",
            "signal_score": 4,
            "cashtags": ["SOL"],
            "narratives": [str(i) for i in range(20)],
        },
        "x-proxy-cynic",
        "D1",
        "cynic",
    )

    assert payload["tool"] == "x-proxy-cynic"
    assert payload["agent_id"] == "hermes-cynic"
    assert payload["context"] == "@alice [4]: hello"
    assert len(payload["tags"]) == 10


def test_build_convergence_payload_uses_resolved_mint_when_present():
    signal = SimpleNamespace(
        cashtag="SOL",
        resolved_mint="mint123",
        author_count=3,
        authors=["a", "b", "c"],
        coordination_score=0.1,
        sentiment="neutral",
        window_hours=6,
        key_quotes=[],
        domain="D1",
    )

    payload = build_convergence_payload(signal, "cynic")
    assert payload["target"] == "mint123"
    assert "mint:mint123" in payload["tags"]
    assert payload["agent_id"] == "convergence-cynic"


def test_build_token_judgment_payload():
    signal = SimpleNamespace(cashtag="SOL", resolved_mint="mint123", author_count=3, window_hours=6)

    payload = build_token_judgment_payload(signal)
    assert payload["content"] == "mint123"
    assert payload["priority"] == "hermes"


def test_build_social_judgment_payload_contains_stimulus():
    payload = build_social_judgment_payload({"text": "signal", "author_screen_name": "alice"}, "D2", "cynic")

    assert payload["domain"] == "D2"
    assert payload["account_id"] == "cynic"
    assert "[DOMAIN: D2]" in payload["content"]


def test_build_heartbeat_payload_sets_recovery_consumer_on_critical():
    payload = build_heartbeat_payload("x-proxy-cynic", "cynic", "critical", "auth_failed=True", "x_auth_expired")

    assert payload["domain"] == "organ-health"
    assert payload["failure_reason"] == "x_auth_expired"
    assert payload["consumer"] == "hermes-recovery@cynic"


def test_transport_posts_observe():
    fake = FakeRequests([200])
    transport = KernelTransport("kernel:3000", "secret", requests_module=fake)

    assert transport.post_observe({"x": 1}) is True
    assert fake.calls[0]["url"] == "http://kernel:3000/observe"


def test_transport_social_judge_retries_rate_limit():
    sleeps = []
    fake = FakeRequests([429, 200])
    transport = KernelTransport("kernel:3000", "secret", requests_module=fake, sleep_fn=sleeps.append)

    result = transport.post_social_judge({"content": "x"}, max_retries=1)
    assert result == {"ok": True, "status": 200}
    assert sleeps == [5]
    assert len(fake.calls) == 2
