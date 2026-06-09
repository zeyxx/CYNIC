import pytest

from sensors.talaria_poh_bridge import to_talaria_event


def test_poh_event_maps_to_talaria_poh_user() -> None:
    event = to_talaria_event({
        "kind": "bnc.poh.game_completed",
        "user_id": "u-123",
        "wallet_address": "wallet-abc",
        "value": {"games_completed": 1},
    })

    envelope = event.envelope()
    assert envelope["scope"] == "talaria.poh.user"
    assert envelope["actor"] == "u-123"
    assert envelope["walletAddress"] == "wallet-abc"
    assert envelope["confidence"] == "observed"
    assert envelope["visibility"] == "internal"
    assert envelope["evidenceHash"]


def test_chess_event_maps_to_chess_signal() -> None:
    event = to_talaria_event({
        "kind": "bnc.chess.rating_updated",
        "player_id": "player-1",
        "value": {"rating": 1420},
    })

    envelope = event.envelope()
    assert envelope["scope"] == "talaria.chess.signal"
    assert envelope["actor"] == "player-1"


def test_unknown_event_is_rejected() -> None:
    with pytest.raises(ValueError, match="unsupported B&C event kind"):
        to_talaria_event({"kind": "bnc.admin.promote_to_governance", "user_id": "u-1"})
