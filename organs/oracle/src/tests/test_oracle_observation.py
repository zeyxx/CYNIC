from __future__ import annotations

from pathlib import Path
import json

from organs.oracle.src.adapters.observation_readback import JsonlObservationReadback
from organs.oracle.src.core.anchoring import AnchorReceipt
from organs.oracle.src.core.observation import observation_from_receipt


def test_observation_payload_from_anchor_receipt():
    receipt = AnchorReceipt(
        status="anchored",
        state_root="hash-1",
        transaction_hash="tx-1",
        network="solana-devnet",
        mode="mock",
    )
    observation = observation_from_receipt(receipt).to_dict()

    assert observation["domain"] == "oracle"
    assert observation["event"] == "transparency_anchor"
    assert observation["state_root"] == "hash-1"
    assert observation["mode"] == "mock"


def test_jsonl_observation_readback(tmp_path: Path):
    receipt = AnchorReceipt(
        status="anchored",
        state_root="hash-1",
        transaction_hash="tx-1",
        network="solana-devnet",
        mode="mock",
    )
    observation = observation_from_receipt(receipt)
    path = tmp_path / "observations.jsonl"
    path.write_text(json.dumps({"observation": observation.to_dict()}) + "\n", encoding="utf-8")

    assert JsonlObservationReadback(path).visible(observation)
