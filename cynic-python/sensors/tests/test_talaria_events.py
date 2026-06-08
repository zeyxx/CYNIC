from sensors.talaria_events import TalariaEvent, evidence_hash


def test_talaria_event_observe_payload_carries_envelope() -> None:
    event = TalariaEvent(
        scope="talaria.poh.user",
        actor="user-1",
        actor_kind="app_user",
        kind="bnc.poh.game_completed",
        title="Game completed",
        summary="Raw B&C PoH evidence recorded for review.",
        source="blitz-and-chill",
        confidence="observed",
        visibility="internal",
        subject_id="user-1",
        value={"games_completed": 1},
    )

    payload = event.observe_payload(agent_id="talaria-poh-bridge")

    assert payload["domain"] == "talaria.poh.user"
    assert payload["tool"] == "bnc.poh.game_completed"
    assert payload["target"] == "user-1"
    assert payload["confidence"] == "observed"
    assert payload["value"]["scope"] == "talaria.poh.user"
    assert "TALARIA_EVENT=" in payload["context"]
    assert "visibility:internal" in payload["tags"]


def test_evidence_hash_is_stable_for_key_order() -> None:
    assert evidence_hash({"a": 1, "b": 2}) == evidence_hash({"b": 2, "a": 1})
