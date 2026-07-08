import json

from organs.twitter.src.adapters.x_ingest_store import (
    detect_auth_failure,
    load_cursor,
    save_cursor,
    write_verdict_to_organ,
)


def test_cursor_round_trip(tmp_path):
    state = tmp_path / "cursor.txt"

    assert load_cursor(state) == 0
    save_cursor(state, 42)
    assert load_cursor(state) == 42


def test_load_cursor_returns_zero_for_invalid_value(tmp_path):
    state = tmp_path / "cursor.txt"
    state.write_text("not-an-int")

    assert load_cursor(state) == 0


def test_detect_auth_failure_when_recent_rows_lack_identity(tmp_path):
    dataset = tmp_path / "dataset.jsonl"
    dataset.write_text(
        "\n".join([
            json.dumps({"tweet_id": "1", "text": "ok"}),
            json.dumps({"tweet_id": "", "text": ""}),
            json.dumps({"tweet_id": "", "text": ""}),
        ])
    )

    assert detect_auth_failure(dataset, rows_to_check=3) is True


def test_detect_auth_failure_false_for_structured_rows(tmp_path):
    dataset = tmp_path / "dataset.jsonl"
    dataset.write_text(json.dumps({"tweet_id": "1", "text": "ok"}) + "\n")

    assert detect_auth_failure(dataset) is False


def test_write_verdict_to_organ_is_idempotent(tmp_path):
    assert write_verdict_to_organ(
        {"tweet_id": "abc", "signal_score": 5, "capture_ts": "2026-01-01T00:00:00Z"},
        {"verdict": "WAG"},
        tmp_path,
    ) is True
    assert write_verdict_to_organ(
        {"tweet_id": "abc", "signal_score": 1, "capture_ts": "changed"},
        {"verdict": "BARK"},
        tmp_path,
    ) is True

    payload = json.loads((tmp_path / "verdicts" / "abc.json").read_text())
    assert payload["signal_score"] == 5
    assert payload["verdict"] == {"verdict": "WAG"}


def test_write_verdict_to_organ_rejects_missing_tweet_id(tmp_path):
    assert write_verdict_to_organ({}, {"verdict": "WAG"}, tmp_path) is False
