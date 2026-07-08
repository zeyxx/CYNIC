from __future__ import annotations

from organs.rag.src.core.status import build_status


def test_empty_rag_membrane_is_non_mutating_and_ok():
    status = build_status(["MANIFEST.yaml", "src/core/status.py"])

    assert status.ok
    assert not status.active_retrieval
    assert status.to_dict()["mutates"] is False


def test_local_state_artifacts_block_activation():
    status = build_status(["state/old-index.jsonl"])

    assert not status.ok
    assert status.local_state_artifacts == ("state/old-index.jsonl",)
