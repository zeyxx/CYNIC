from __future__ import annotations

from organs.freebox.src.core.config import EdgeExposureEvidence, FreeboxConfig, PortForwardIntent
from organs.freebox.src.core.operations import FreeboxRuntime


class _StubProvider:
    def __init__(self) -> None:
        self.authorized = False
        self.port_payload = None

    def request_authorization(self, payload: dict[str, str]) -> dict:
        return {"success": True, "result": {"app_token": "token-abc", "track_id": "track-1"}}

    def read_authorization_status(self, track_id: str) -> str:
        return "granted"

    def get_challenge(self) -> str:
        return "challenge-1"

    def request_session_token(self, app_id: str, password: str) -> str:
        return "session-1"

    def request_port_open(self, session_token: str, payload: dict[str, object]) -> dict:
        self.port_payload = payload
        return {"status": "ok"}


class _StubTokenStore:
    def __init__(self) -> None:
        self.token = None

    def read_token(self) -> str | None:
        return self.token

    def write_token(self, token: str) -> None:
        self.token = token


def test_authorization_and_approved_port_open_flow():
    provider = _StubProvider()
    token_store = _StubTokenStore()
    config = FreeboxConfig()

    messages = []
    runtime = FreeboxRuntime(
        provider=provider,
        token_store=token_store,
        config=config,
        printer=messages.append,
    )

    intent = PortForwardIntent(
        lan_ip="10.0.0.42",
        wan_port_start=8443,
        wan_port_end=8443,
        lan_port=443,
        comment="test approved exposure",
    )
    evidence = EdgeExposureEvidence(
        registry_intent_id="edge:test-approved-exposure",
        synapse_target="cynic-kernel-primary.synapse.cynic",
    )

    assert runtime.run_bridge(intent, approved=True, evidence=evidence)
    assert token_store.token == "token-abc"
    assert provider.port_payload == {
        "enabled": True,
        "ip": "10.0.0.42",
        "wan_port_start": 8443,
        "wan_port_end": 8443,
        "lan_port": 443,
        "lan_ip": "10.0.0.42",
        "src_ip": "0.0.0.0",
        "ip_proto": "tcp",
        "comment": "test approved exposure",
    }


def test_router_mutation_requires_explicit_intent_and_approval():
    provider = _StubProvider()
    runtime = FreeboxRuntime(
        provider=provider,
        token_store=_StubTokenStore(),
        config=FreeboxConfig(),
        printer=lambda message: None,
    )
    intent = PortForwardIntent(
        lan_ip="10.0.0.42",
        wan_port_start=8443,
        wan_port_end=8443,
        lan_port=443,
    )

    assert not runtime.run_bridge()
    assert not runtime.run_bridge(intent, approved=False)
    assert not runtime.run_bridge(intent, approved=True)
    assert not runtime.run_bridge(
        intent,
        approved=True,
        evidence=EdgeExposureEvidence(
            registry_intent_id="edge:test",
            synapse_target="not-synapse.example",
        ),
    )
    assert provider.port_payload is None
